// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! A [`View`] as Graphviz DOT, in the OPM form (spec/diagram.md §3.11) with the colors of the
//! implementation status (§3.10).
//!
//! - Process: ellipse. Object: box. physical: thick outline. environmental: dashed outline.
//! - State: a rounded box inside its Object. initial: thick outline; final and default are
//!   written under the name.
//! - In-zoomed Thing: a frame around the inner Things (rounded for a Process).
//! - Procedural links: consumption and result are arrows, effect a two-headed arrow (or one
//!   arrow from `from_state` and one into `to_state`), instrument ends in a white circle, agent
//!   in a black circle, invocation is an arrow labelled `invokes`. Event and condition add `e`
//!   and `c`.
//! - Structural links: one triangle per source and kind, joined to every target (black:
//!   aggregation, black in white: exhibition, white: generalization, black dot in white:
//!   classification). tagged is an open arrow with its tag.
//! - A view with an in-zoomed Thing is drawn top down (the steps run downwards, as in OPM);
//!   others left to right.
//! - A link in a XOR or OR fan is labelled `XOR` or `OR`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use opm_model::{Affiliation, Essence, FanOperator, LinkKind, Modifier, Thing, JUIZ_NS};

use crate::view::{End, ShownLink, View};

const FONT: &str = "Helvetica";
const GREY: &str = "#777777";

/// The implementation status (opm-mapping.md §5, `juiz:status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Released,
    Merged,
    InProgress,
    Planned,
    Defect,
}

impl Status {
    pub fn from_name(s: &str) -> Option<Status> {
        Some(match s {
            "released" => Status::Released,
            "merged" => Status::Merged,
            "in_progress" => Status::InProgress,
            "planned" => Status::Planned,
            "defect" => Status::Defect,
            _ => return None,
        })
    }

    /// The fill of a Thing; `None` for released (white).
    pub fn fill(self) -> Option<&'static str> {
        match self {
            Status::Released => None,
            Status::Merged => Some("#dcf2dc"),
            Status::InProgress => Some("#fff3c4"),
            Status::Planned => Some("#eeeeee"),
            Status::Defect => Some("#f9d4d4"),
        }
    }

    /// The color of a link line and of its label.
    pub fn line(self) -> &'static str {
        match self {
            Status::Released => "#333333",
            Status::Merged => "#3c9a3c",
            Status::InProgress => "#c49a00",
            Status::Planned => "#999999",
            Status::Defect => "#c83232",
        }
    }

    /// The color of the text in a Thing.
    pub fn text(self) -> &'static str {
        match self {
            Status::Planned => "#888888",
            _ => "#111111",
        }
    }
}

fn status_of(ext: &opm_model::Extensions) -> (Status, Option<&str>) {
    let s = ext
        .attribute(JUIZ_NS, "status")
        .and_then(Status::from_name)
        .unwrap_or(Status::Released);
    let refs = ext
        .attribute(JUIZ_NS, "refs")
        .filter(|_| s != Status::Released);
    (s, refs)
}

/// Escape text for an HTML-like label.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            _ => o.push(c),
        }
    }
    o
}

/// Escape text for a quoted DOT string.
fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The small grey line under a name: the juiz role and type, if any.
fn juiz_note(t: &Thing) -> Option<String> {
    let role = t.extensions.attribute(JUIZ_NS, "role")?;
    let mut s = role.to_string();
    if let Some(ty) = t.extensions.attribute(JUIZ_NS, "type") {
        s.push_str(" : ");
        s.push_str(ty);
    } else if let Some(k) = t.extensions.attribute(JUIZ_NS, "value_kind") {
        s.push_str(&format!(" ({})", k));
    }
    Some(s)
}

/// Draw a view as DOT.
pub fn to_dot(view: &View) -> String {
    Writer::new(view).write()
}

struct Writer<'v, 'a> {
    view: &'v View<'a>,
    out: String,
    /// Thing id -> DOT node id.
    node: HashMap<&'a str, String>,
    /// State id -> port name.
    port: HashMap<&'a str, String>,
    /// In-zoomed Thing id -> cluster name.
    cluster: HashMap<&'a str, String>,
    /// In-zoomed Thing id -> every Thing drawn inside it, at any depth.
    inside: HashMap<&'a str, HashSet<&'a str>>,
    fan: HashMap<&'a str, FanOperator>,
}

impl<'v, 'a> Writer<'v, 'a> {
    fn new(view: &'v View<'a>) -> Self {
        let mut node = HashMap::new();
        let mut port = HashMap::new();
        let mut cluster = HashMap::new();
        for (i, t) in view.things.iter().enumerate() {
            if view.zoom_of(&t.id).is_some() {
                cluster.insert(t.id.as_str(), format!("cluster_z{}", i));
                node.insert(t.id.as_str(), format!("z{}", i));
            } else {
                node.insert(t.id.as_str(), format!("t{}", i));
            }
            for (j, s) in t.states().iter().enumerate() {
                port.insert(s.id.as_str(), format!("s{}", j));
            }
        }
        let mut inside: HashMap<&str, HashSet<&str>> = HashMap::new();
        fn collect<'a>(view: &View<'a>, id: &'a str, out: &mut HashSet<&'a str>) {
            if let Some(z) = view.zoom_of(id) {
                for &i in &z.inner {
                    if out.insert(i) {
                        collect(view, i, out);
                    }
                }
            }
        }
        for z in &view.zooms {
            let mut s = HashSet::new();
            collect(view, z.in_zoom.refinee.as_str(), &mut s);
            inside.insert(z.in_zoom.refinee.as_str(), s);
        }
        let mut fan = HashMap::new();
        for f in &view.model.fans {
            for l in &f.links {
                fan.insert(l.as_str(), f.operator);
            }
        }
        Writer {
            view,
            out: String::new(),
            node,
            port,
            cluster,
            inside,
            fan,
        }
    }

    fn write(mut self) -> String {
        let v = self.view;
        let title = match (&v.model.name, v.diagram) {
            (Some(m), Some(d)) => format!("{} — {}", m, d.name),
            (Some(m), None) => m.clone(),
            (None, _) => v.name(),
        };
        let _ = writeln!(self.out, "digraph {} {{", q(&v.name()));
        let _ = writeln!(
            self.out,
            "  graph [rankdir={r}, compound=true, nodesep=0.35, ranksep=0.7, fontname={f}, fontsize=12, labelloc=t, label=<<B>{t}</B>>];",
            r = if self.top_down() { "TB" } else { "LR" },
            f = q(FONT),
            t = esc(&title)
        );
        let _ = writeln!(
            self.out,
            "  node [fontname={f}, fontsize=11, margin=\"0.12,0.05\"];\n  edge [fontname={f}, fontsize=9, color=\"#333333\", fontcolor=\"#333333\"];",
            f = q(FONT)
        );

        let held: HashSet<&str> = v
            .zooms
            .iter()
            .flat_map(|z| z.inner.iter().copied())
            .collect();
        for t in &v.things {
            if !held.contains(t.id.as_str()) {
                self.thing(t, 1);
            }
        }
        self.links();
        self.out.push_str("}\n");
        self.out
    }

    /// In-zooms run their steps from top to bottom as in OPM, so a view with one is drawn top
    /// down. Others are drawn from left to right, which keeps the flow of values readable.
    fn top_down(&self) -> bool {
        !self.view.zooms.is_empty()
    }

    fn thing(&mut self, t: &'a Thing, depth: usize) {
        let pad = "  ".repeat(depth);
        if let Some(z) = self.view.zoom_of(&t.id) {
            let inner: Vec<&str> = z.inner.clone();
            let steps = z.in_zoom.steps.clone();
            let (status, refs) = status_of(&t.extensions);
            let mut style = vec![];
            if t.is_process() {
                style.push("rounded");
            }
            if t.affiliation == Affiliation::Environmental {
                style.push("dashed");
            }
            if status.fill().is_some() {
                style.push("filled");
            }
            let _ = writeln!(
                self.out,
                "{}subgraph {} {{",
                pad,
                self.cluster[t.id.as_str()]
            );
            let _ = writeln!(
                self.out,
                "{}  label=<{}>; labeljust=l; margin=14; style={}; penwidth={}; fillcolor={}; fontcolor={};",
                pad,
                self.label_text(t, refs),
                q(&style.join(",")),
                if t.essence == Essence::Physical { 2.5 } else { 1.2 },
                q(status.fill().unwrap_or("white")),
                q(status.text())
            );
            let _ = writeln!(
                self.out,
                "{}  {} [shape=point, width=0.01, style=invis];",
                pad,
                self.node[t.id.as_str()]
            );
            for i in &inner {
                if let Some(it) = self.view.model.thing(i) {
                    self.thing(it, depth + 1);
                }
            }
            // keep the steps in order
            let shown: Vec<Vec<String>> = steps
                .iter()
                .map(|s| {
                    s.iter()
                        .filter(|p| inner.contains(&p.as_str()))
                        .map(|p| self.node[p.as_str()].clone())
                        .collect::<Vec<_>>()
                })
                .filter(|s| !s.is_empty())
                .collect();
            for w in shown.windows(2) {
                for a in &w[0] {
                    for b in &w[1] {
                        let _ = writeln!(self.out, "{}  {} -> {} [style=invis];", pad, a, b);
                    }
                }
            }
            let _ = writeln!(self.out, "{}}}", pad);
            return;
        }

        let (status, refs) = status_of(&t.extensions);
        let mut style = vec![];
        let shape = if t.is_process() { "ellipse" } else { "box" };
        if t.affiliation == Affiliation::Environmental {
            style.push("dashed");
        }
        if status.fill().is_some() {
            style.push("filled");
        }
        let mut attrs = vec![
            format!("shape={}", shape),
            format!("label=<{}>", self.thing_label(t, refs, status)),
        ];
        if !style.is_empty() {
            attrs.push(format!("style={}", q(&style.join(","))));
        }
        if let Some(f) = status.fill() {
            attrs.push(format!("fillcolor={}", q(f)));
        }
        if t.essence == Essence::Physical {
            attrs.push("penwidth=2.5".into());
        }
        if status == Status::Planned {
            attrs.push(format!("color={}", q("#999999")));
        }
        let _ = writeln!(
            self.out,
            "{}{} [{}];",
            pad,
            self.node[t.id.as_str()],
            attrs.join(", ")
        );
    }

    /// Name, juiz note and status refs, as HTML text lines (for a cluster label).
    fn label_text(&self, t: &Thing, refs: Option<&str>) -> String {
        let mut s = format!("<B>{}</B>", esc(&t.name));
        if let Some(n) = juiz_note(t) {
            let _ = write!(
                s,
                "<BR/><FONT POINT-SIZE=\"8\" COLOR=\"{}\">{}</FONT>",
                GREY,
                esc(&n)
            );
        }
        if let Some(r) = refs {
            let _ = write!(s, "<BR/><FONT POINT-SIZE=\"8\">{}</FONT>", esc(r));
        }
        s
    }

    fn thing_label(&self, t: &Thing, refs: Option<&str>, status: Status) -> String {
        let states: Vec<_> = t
            .states()
            .iter()
            .filter(|s| !self.view.hidden_states.contains(s.id.as_str()))
            .collect();
        let mut rows = vec![format!(
            "<TR><TD COLSPAN=\"{}\"><FONT COLOR=\"{}\">{}</FONT></TD></TR>",
            states.len().max(1),
            status.text(),
            esc(&t.name)
        )];
        if let Some(n) = juiz_note(t) {
            rows.push(format!(
                "<TR><TD COLSPAN=\"{}\"><FONT POINT-SIZE=\"8\" COLOR=\"{}\">{}</FONT></TD></TR>",
                states.len().max(1),
                GREY,
                esc(&n)
            ));
        }
        if let Some(r) = refs {
            rows.push(format!(
                "<TR><TD COLSPAN=\"{}\"><FONT POINT-SIZE=\"8\">{}</FONT></TD></TR>",
                states.len().max(1),
                esc(r)
            ));
        }
        if !states.is_empty() {
            let mut cells = String::new();
            for s in &states {
                let mut marks = vec![];
                if s.final_ {
                    marks.push("final");
                }
                if s.default {
                    marks.push("default");
                }
                let mark = if marks.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<BR/><FONT POINT-SIZE=\"7\" COLOR=\"{}\">{}</FONT>",
                        GREY,
                        marks.join(", ")
                    )
                };
                let _ = write!(
                    cells,
                    "<TD PORT=\"{}\" BORDER=\"{}\" STYLE=\"rounded\" CELLPADDING=\"4\">{}{}</TD>",
                    self.port[s.id.as_str()],
                    if s.initial { 2 } else { 1 },
                    esc(&s.name),
                    mark
                );
            }
            rows.push(format!("<TR>{}</TR>", cells));
        }
        format!(
            "<TABLE BORDER=\"0\" CELLBORDER=\"0\" CELLSPACING=\"4\" CELLPADDING=\"0\">{}</TABLE>",
            rows.concat()
        )
    }

    /// The DOT end of a link end, and the cluster to clip at if the end is an in-zoomed Thing.
    fn end(&self, e: &End, other: &End) -> (String, Option<String>) {
        let id = e.thing.as_str();
        let mut n = self.node[id].clone();
        if let Some(s) = &e.state {
            if let Some(p) = self.port.get(s.as_str()) {
                n = format!("{}:{}", n, p);
            }
        }
        let clip = self.cluster.get(id).and_then(|c| {
            let inner = &self.inside[id];
            (!inner.contains(other.thing.as_str()) && other.thing != e.thing).then(|| c.clone())
        });
        (n, clip)
    }

    fn edge(&mut self, s: &End, t: &End, mut attrs: Vec<String>) {
        let (sn, sc) = self.end(s, t);
        let (tn, tc) = self.end(t, s);
        if let Some(c) = sc {
            attrs.push(format!("ltail={}", c));
        }
        if let Some(c) = tc {
            attrs.push(format!("lhead={}", c));
        }
        let _ = writeln!(self.out, "  {} -> {} [{}];", sn, tn, attrs.join(", "));
    }

    fn links(&mut self) {
        let links: Vec<ShownLink<'a>> = self.view.links.clone();
        // structural links that share a source and kind share one triangle
        let mut forks: BTreeMap<(usize, String, bool), Vec<&ShownLink<'a>>> = BTreeMap::new();
        for l in &links {
            match l.link.kind {
                LinkKind::Aggregation
                | LinkKind::Exhibition
                | LinkKind::Generalization
                | LinkKind::Classification => {
                    let k = LinkKind::ALL
                        .iter()
                        .position(|&k| k == l.link.kind)
                        .unwrap();
                    forks
                        .entry((k, l.source.thing.clone(), l.link.ordered))
                        .or_default()
                        .push(l);
                }
                _ => self.procedural(l),
            }
        }
        for (i, ((_, _, ordered), ls)) in forks.into_iter().enumerate() {
            let kind = ls[0].link.kind;
            let tri = format!("f{}", i);
            let (fill, label) = match kind {
                LinkKind::Aggregation => ("black", ""),
                LinkKind::Exhibition => ("white", "<FONT POINT-SIZE=\"6\">▲</FONT>"),
                LinkKind::Generalization => ("white", ""),
                _ => ("white", "<FONT POINT-SIZE=\"6\">●</FONT>"),
            };
            let _ = writeln!(
                self.out,
                "  {} [shape=triangle, orientation={}, fixedsize=true, width=0.28, height=0.24, style=filled, fillcolor={}, label=<{}>, margin=0];",
                tri,
                // the apex points at the whole
                if self.top_down() { 0 } else { 90 },
                fill,
                label
            );
            let tri_end = End {
                thing: String::new(),
                state: None,
            };
            let src = &ls[0].source;
            let (sn, sc) = self.end(src, &tri_end);
            let mut a = vec!["arrowhead=none".to_string()];
            if ordered {
                a.push("label=\"{ordered}\"".into());
            }
            if let Some(c) = sc {
                a.push(format!("ltail={}", c));
            }
            let _ = writeln!(self.out, "  {} -> {} [{}];", sn, tri, a.join(", "));
            for l in ls {
                let (tn, tc) = self.end(&l.target, &tri_end);
                let mut a = vec!["arrowhead=none".to_string()];
                a.extend(self.common(l));
                if let Some(c) = tc {
                    a.push(format!("lhead={}", c));
                }
                let _ = writeln!(self.out, "  {} -> {} [{}];", tri, tn, a.join(", "));
            }
        }
    }

    /// Attributes every link gets: status color, multiplicities, path, fan, refs.
    fn common(&self, l: &ShownLink) -> Vec<String> {
        let mut a = vec![];
        let (status, refs) = status_of(&l.link.extensions);
        if status != Status::Released {
            a.push(format!("color={0}, fontcolor={0}", q(status.line())));
        }
        if let Some(m) = &l.link.source_multiplicity {
            a.push(format!("taillabel={}", q(m)));
        }
        if let Some(m) = &l.link.target_multiplicity {
            a.push(format!("headlabel={}", q(m)));
        }
        let mut xl = vec![];
        if let Some(p) = &l.link.path {
            xl.push(format!("path {}", p));
        }
        if let Some(op) = self.fan.get(l.link.id.as_str()) {
            xl.push(
                match op {
                    FanOperator::Xor => "XOR",
                    FanOperator::Or => "OR",
                }
                .to_string(),
            );
        }
        if let Some(r) = refs {
            xl.push(r.to_string());
        }
        if !xl.is_empty() {
            a.push(format!("xlabel={}", q(&xl.join(" "))));
        }
        a
    }

    fn procedural(&mut self, l: &ShownLink<'a>) {
        let link = l.link;
        let mut label = vec![];
        match link.modifier {
            Some(Modifier::Event) => label.push("e".to_string()),
            Some(Modifier::Condition) => label.push("c".to_string()),
            None => {}
        }
        let mut a = vec![];
        match link.kind {
            LinkKind::Consumption | LinkKind::Result => a.push("arrowhead=normal".to_string()),
            LinkKind::Instrument => a.push("arrowhead=odot".to_string()),
            LinkKind::Agent => a.push("arrowhead=dot".to_string()),
            LinkKind::Invocation => label.insert(0, "invokes".into()),
            LinkKind::OvertimeException => label.insert(0, "overtime".into()),
            LinkKind::UndertimeException => label.insert(0, "undertime".into()),
            LinkKind::Tagged => {
                label.insert(0, link.tag.clone().unwrap_or_default());
                if let Some(b) = &link.backward_tag {
                    label[0] = format!("{} / {}", label[0], b);
                    a.push("dir=both, arrowtail=vee".into());
                }
                a.push("arrowhead=vee".into());
            }
            LinkKind::Effect => {
                if l.from_state.is_some() || l.to_state.is_some() {
                    // input link and output link
                    let obj = |s: &Option<String>| End {
                        thing: l.target.thing.clone(),
                        state: s.clone(),
                    };
                    let mut c = self.common(l);
                    if !label.is_empty() {
                        c.push(format!("label={}", q(&label.join(" "))));
                    }
                    self.edge(&obj(&l.from_state), &l.source, c.clone());
                    let c: Vec<String> =
                        c.into_iter().filter(|x| !x.starts_with("label=")).collect();
                    self.edge(&l.source, &obj(&l.to_state), c);
                    return;
                }
                a.push("dir=both, arrowtail=normal".into());
            }
            _ => {}
        }
        if !label.is_empty() {
            a.push(format!("label={}", q(&label.join(" "))));
        }
        a.extend(self.common(l));
        let (s, t) = (l.source.clone(), l.target.clone());
        self.edge(&s, &t, a);
    }
}
