// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Writing a [`Model`] as interchange XML (spec/xml.md).
//!
//! The output is laid out the same way every time (two-space indentation, attributes in a fixed
//! order, default values left out, empty lists left out), so writing what was read gives the
//! same text again. A model that passes [`crate::check`] without errors is written as a
//! document that is valid against the XSD.

use std::fmt::Write as _;

use crate::ext::{ExtElement, ExtNode, Extensions};
use crate::model::*;

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

/// Write a model as an interchange document.
pub fn to_string(model: &Model) -> String {
    let mut w = Writer {
        out: String::new(),
        prefixes: prefixes(model),
    };
    w.model(model);
    w.out
}

/// Write a model to a file.
pub fn to_path(model: &Model, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    std::fs::write(path, to_string(model))
}

/// The prefix of every namespace used, `opm` first. Prefixes the model asks for are kept;
/// namespaces it does not declare get `juiz` (for the juiz namespace) or `ns1`, `ns2`, ...
fn prefixes(model: &Model) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![("opm".into(), OPM_NS.into())];
    let free = |out: &Vec<(String, String)>, p: &str| {
        p != "xml" && !p.starts_with("xmlns") && out.iter().all(|(q, _)| q != p)
    };
    for (p, uri) in &model.prefixes {
        if uri != OPM_NS && uri != XML_NS && free(&out, p) && out.iter().all(|(_, u)| u != uri) {
            out.push((p.clone(), uri.clone()));
        }
    }
    let mut used = Vec::new();
    collect_namespaces(model, &mut used);
    let mut n = 1;
    for uri in used {
        if uri.is_empty() || uri == XML_NS || out.iter().any(|(_, u)| u == uri) {
            continue;
        }
        let p = if uri == JUIZ_NS && free(&out, "juiz") {
            "juiz".to_string()
        } else {
            loop {
                let p = format!("ns{}", n);
                n += 1;
                if free(&out, &p) {
                    break p;
                }
            }
        };
        out.push((p, uri.to_string()));
    }
    out
}

fn collect_namespaces<'a>(model: &'a Model, out: &mut Vec<&'a str>) {
    model.extensions.namespaces(out);
    for t in &model.things {
        t.extensions.namespaces(out);
        for s in t.states() {
            s.extensions.namespaces(out);
        }
    }
    for l in &model.links {
        l.extensions.namespaces(out);
    }
    for f in &model.fans {
        f.extensions.namespaces(out);
    }
    for z in &model.in_zooms {
        z.extensions.namespaces(out);
    }
    for d in model.diagrams() {
        d.extensions.namespaces(out);
        for a in &d.appearances {
            a.extensions.namespaces(out);
        }
    }
}

fn escape_attr(v: &str) -> String {
    let mut s = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '"' => s.push_str("&quot;"),
            '\n' => s.push_str("&#10;"),
            '\r' => s.push_str("&#13;"),
            '\t' => s.push_str("&#9;"),
            c => s.push(c),
        }
    }
    s
}

fn escape_text(v: &str) -> String {
    let mut s = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' => s.push_str("&amp;"),
            '<' => s.push_str("&lt;"),
            '>' => s.push_str("&gt;"),
            '\r' => s.push_str("&#13;"),
            c => s.push(c),
        }
    }
    s
}

fn number(n: f64) -> String {
    // f64's Display gives the shortest text that reads back to the same value ("40", "0.5").
    format!("{}", n)
}

/// Attributes of one element, in the order they are written.
#[derive(Default)]
struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn add(&mut self, name: &str, value: impl Into<String>) -> &mut Self {
        self.0.push((name.to_string(), value.into()));
        self
    }

    fn opt(&mut self, name: &str, value: &Option<String>) -> &mut Self {
        if let Some(v) = value {
            self.add(name, v.clone());
        }
        self
    }

    fn flag(&mut self, name: &str, value: bool) -> &mut Self {
        if value {
            self.add(name, "true");
        }
        self
    }
}

struct Writer {
    out: String,
    prefixes: Vec<(String, String)>,
}

impl Writer {
    fn prefix(&self, uri: &str) -> Option<&str> {
        if uri == XML_NS {
            return Some("xml");
        }
        self.prefixes
            .iter()
            .find(|(_, u)| u == uri)
            .map(|(p, _)| p.as_str())
    }

    fn qname(&self, uri: &str, name: &str) -> String {
        match self.prefix(uri) {
            Some(p) if !uri.is_empty() => format!("{}:{}", p, name),
            _ => name.to_string(),
        }
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str("  ");
        }
    }

    fn open_tag(&mut self, depth: usize, name: &str, attrs: &Attrs, ext: Option<&Extensions>) {
        self.indent(depth);
        let _ = write!(self.out, "<opm:{}", name);
        for (k, v) in &attrs.0 {
            let _ = write!(self.out, " {}=\"{}\"", k, escape_attr(v));
        }
        if let Some(ext) = ext {
            for a in &ext.attributes {
                let q = self.qname(&a.namespace, &a.name);
                let _ = write!(self.out, " {}=\"{}\"", q, escape_attr(&a.value));
            }
        }
    }

    /// An element with an id: description and foreign elements first, then `body`.
    /// Written as an empty element when there is nothing inside.
    #[allow(clippy::too_many_arguments)]
    fn element(
        &mut self,
        depth: usize,
        name: &str,
        attrs: &Attrs,
        description: &Option<String>,
        ext: &Extensions,
        has_body: bool,
        body: impl FnOnce(&mut Self),
    ) {
        self.open_tag(depth, name, attrs, Some(ext));
        if description.is_none() && ext.elements.is_empty() && !has_body {
            self.out.push_str("/>\n");
            return;
        }
        self.out.push_str(">\n");
        if let Some(d) = description {
            self.indent(depth + 1);
            let _ = writeln!(
                self.out,
                "<opm:description>{}</opm:description>",
                escape_text(d)
            );
        }
        for e in &ext.elements {
            self.ext_element(depth + 1, e);
            self.out.push('\n');
        }
        body(self);
        self.indent(depth);
        let _ = writeln!(self.out, "</opm:{}>", name);
    }

    fn empty(&mut self, depth: usize, name: &str, attrs: &Attrs) {
        self.open_tag(depth, name, attrs, None);
        self.out.push_str("/>\n");
    }

    fn list(&mut self, depth: usize, name: &str, body: impl FnOnce(&mut Self)) {
        self.indent(depth);
        let _ = writeln!(self.out, "<opm:{}>", name);
        body(self);
        self.indent(depth);
        let _ = writeln!(self.out, "</opm:{}>", name);
    }

    /// A foreign element. Elements with only element children are indented; elements with text
    /// are written on one line so that the text is kept exactly. No trailing newline.
    fn ext_element(&mut self, depth: usize, e: &ExtElement) {
        let has_text = e.children.iter().any(|c| matches!(c, ExtNode::Text(_)));
        self.indent(depth);
        self.ext_inline_open(e);
        if e.children.is_empty() {
            self.out.push_str("/>");
            return;
        }
        self.out.push('>');
        if has_text {
            for c in &e.children {
                self.ext_inline(c);
            }
        } else {
            self.out.push('\n');
            for c in &e.children {
                if let ExtNode::Element(c) = c {
                    self.ext_element(depth + 1, c);
                    self.out.push('\n');
                }
            }
            self.indent(depth);
        }
        let q = self.qname(&e.namespace, &e.name);
        let _ = write!(self.out, "</{}>", q);
    }

    fn ext_inline_open(&mut self, e: &ExtElement) {
        let q = self.qname(&e.namespace, &e.name);
        let _ = write!(self.out, "<{}", q);
        for a in &e.attributes {
            let q = match &a.namespace {
                Some(ns) => self.qname(ns, &a.name),
                None => a.name.clone(),
            };
            let _ = write!(self.out, " {}=\"{}\"", q, escape_attr(&a.value));
        }
    }

    fn ext_inline(&mut self, n: &ExtNode) {
        match n {
            ExtNode::Text(t) => self.out.push_str(&escape_text(t)),
            ExtNode::Element(e) => {
                self.ext_inline_open(e);
                if e.children.is_empty() {
                    self.out.push_str("/>");
                } else {
                    self.out.push('>');
                    for c in &e.children {
                        self.ext_inline(c);
                    }
                    let q = self.qname(&e.namespace, &e.name);
                    let _ = write!(self.out, "</{}>", q);
                }
            }
        }
    }

    fn model(&mut self, m: &Model) {
        self.out
            .push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        let mut attrs = Attrs::default();
        for (p, uri) in self.prefixes.clone() {
            attrs.add(&format!("xmlns:{}", p), uri);
        }
        attrs.add("format_version", m.format_version.clone());
        attrs.add("id", m.id.clone());
        attrs.opt("name", &m.name);
        self.element(
            0,
            "model",
            &attrs,
            &m.description,
            &m.extensions,
            true,
            |w| {
                if m.things.is_empty() {
                    w.indent(1);
                    w.out.push_str("<opm:things/>\n");
                } else {
                    w.list(1, "things", |w| {
                        for t in &m.things {
                            w.thing(2, t);
                        }
                    });
                }
                if !m.links.is_empty() {
                    w.list(1, "links", |w| {
                        for l in &m.links {
                            w.link(2, l);
                        }
                    });
                }
                if !m.fans.is_empty() {
                    w.list(1, "fans", |w| {
                        for f in &m.fans {
                            w.fan(2, f);
                        }
                    });
                }
                if !m.in_zooms.is_empty() {
                    w.list(1, "in_zooms", |w| {
                        for z in &m.in_zooms {
                            w.in_zoom(2, z);
                        }
                    });
                }
                if let Some(d) = &m.diagram {
                    w.diagram(1, d);
                }
            },
        );
    }

    fn thing(&mut self, depth: usize, t: &Thing) {
        let mut attrs = Attrs::default();
        attrs.add("id", t.id.clone()).add("name", t.name.clone());
        if t.essence == Essence::Physical {
            attrs.add("essence", "physical");
        }
        if t.affiliation == Affiliation::Environmental {
            attrs.add("affiliation", "environmental");
        }
        match &t.kind {
            ThingKind::Object { states } => {
                self.element(
                    depth,
                    "object",
                    &attrs,
                    &t.description,
                    &t.extensions,
                    !states.is_empty(),
                    |w| {
                        for s in states {
                            w.state(depth + 1, s);
                        }
                    },
                );
            }
            ThingKind::Process { duration } => {
                self.element(
                    depth,
                    "process",
                    &attrs,
                    &t.description,
                    &t.extensions,
                    duration.is_some(),
                    |w| {
                        if let Some(d) = duration {
                            let mut a = Attrs::default();
                            if let Some(n) = d.min {
                                a.add("min", number(n));
                            }
                            if let Some(n) = d.expected {
                                a.add("expected", number(n));
                            }
                            if let Some(n) = d.max {
                                a.add("max", number(n));
                            }
                            a.add("unit", d.unit.clone());
                            w.empty(depth + 1, "duration", &a);
                        }
                    },
                );
            }
        }
    }

    fn state(&mut self, depth: usize, s: &State) {
        let mut attrs = Attrs::default();
        attrs
            .add("id", s.id.clone())
            .add("name", s.name.clone())
            .flag("initial", s.initial)
            .flag("final", s.final_)
            .flag("default", s.default);
        self.element(
            depth,
            "state",
            &attrs,
            &s.description,
            &s.extensions,
            false,
            |_| {},
        );
    }

    fn link(&mut self, depth: usize, l: &Link) {
        let mut attrs = Attrs::default();
        attrs
            .add("id", l.id.clone())
            .add("source", l.source.clone())
            .add("target", l.target.clone());
        if let Some(m) = l.modifier {
            attrs.add(
                "modifier",
                match m {
                    Modifier::Event => "event",
                    Modifier::Condition => "condition",
                },
            );
        }
        attrs
            .opt("from_state", &l.from_state)
            .opt("to_state", &l.to_state)
            .flag("ordered", l.ordered)
            .opt("tag", &l.tag)
            .opt("backward_tag", &l.backward_tag)
            .opt("path", &l.path)
            .opt("source_multiplicity", &l.source_multiplicity)
            .opt("target_multiplicity", &l.target_multiplicity);
        self.element(
            depth,
            l.kind.name(),
            &attrs,
            &l.description,
            &l.extensions,
            false,
            |_| {},
        );
    }

    fn fan(&mut self, depth: usize, f: &Fan) {
        let mut attrs = Attrs::default();
        attrs.add("id", f.id.clone()).add(
            "operator",
            match f.operator {
                FanOperator::Xor => "xor",
                FanOperator::Or => "or",
            },
        );
        self.element(
            depth,
            "fan",
            &attrs,
            &f.description,
            &f.extensions,
            true,
            |w| {
                for r in &f.links {
                    let mut a = Attrs::default();
                    a.add("ref", r.clone());
                    w.empty(depth + 1, "link_ref", &a);
                }
            },
        );
    }

    fn in_zoom(&mut self, depth: usize, z: &InZoom) {
        let mut attrs = Attrs::default();
        attrs
            .add("id", z.id.clone())
            .add("refinee", z.refinee.clone());
        let has_body = !z.steps.is_empty() || !z.parts.is_empty();
        self.element(
            depth,
            "in_zoom",
            &attrs,
            &z.description,
            &z.extensions,
            has_body,
            |w| {
                for step in &z.steps {
                    w.list(depth + 1, "step", |w| {
                        for m in step {
                            let mut a = Attrs::default();
                            a.add("ref", m.clone());
                            w.empty(depth + 2, "member", &a);
                        }
                    });
                }
                for p in &z.parts {
                    let mut a = Attrs::default();
                    a.add("ref", p.clone());
                    w.empty(depth + 1, "part", &a);
                }
            },
        );
    }

    fn diagram(&mut self, depth: usize, d: &Diagram) {
        let mut attrs = Attrs::default();
        attrs.add("id", d.id.clone()).add("name", d.name.clone());
        match d.refinement {
            Refinement::Root => {}
            Refinement::InZoom => {
                attrs.add("refinement", "in_zoom");
            }
            Refinement::Unfold => {
                attrs.add("refinement", "unfold");
            }
            Refinement::View => {
                attrs.add("refinement", "view");
            }
        }
        attrs.opt("refinee", &d.refinee);
        let has_body = !d.appearances.is_empty() || !d.children.is_empty();
        self.element(
            depth,
            "diagram",
            &attrs,
            &d.description,
            &d.extensions,
            has_body,
            |w| {
                for a in &d.appearances {
                    w.appearance(depth + 1, a);
                }
                for c in &d.children {
                    w.diagram(depth + 1, c);
                }
            },
        );
    }

    fn appearance(&mut self, depth: usize, a: &Appearance) {
        let mut attrs = Attrs::default();
        attrs.add("id", a.id.clone()).add("thing", a.thing.clone());
        let has_body = !a.suppressed_states.is_empty() || a.layout.is_some();
        self.element(
            depth,
            "appearance",
            &attrs,
            &a.description,
            &a.extensions,
            has_body,
            |w| {
                for s in &a.suppressed_states {
                    let mut x = Attrs::default();
                    x.add("ref", s.clone());
                    w.empty(depth + 1, "suppressed_state", &x);
                }
                if let Some(l) = &a.layout {
                    let mut x = Attrs::default();
                    x.add("x", number(l.x)).add("y", number(l.y));
                    if let Some(n) = l.width {
                        x.add("width", number(n));
                    }
                    if let Some(n) = l.height {
                        x.add("height", number(n));
                    }
                    w.empty(depth + 1, "layout", &x);
                }
            },
        );
    }
}
