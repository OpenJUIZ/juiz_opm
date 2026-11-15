// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The canonical form of juiz-OPL (spec/opl.md §9) written from a model.

use std::collections::HashMap;

use opm_model::{
    Affiliation, Essence, FanOperator, Link, LinkKind, Model, Modifier, State, Thing, ThingKind,
};

use crate::diag::{Code, Diagnostic};
use crate::juiz::{self, Target};
use crate::names;

/// How to write.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Options {
    /// Write `{id: ...}` on every element (spec/opl.md §10.4).
    pub ids: bool,
}

/// The text, and what could not be written. The text is usable only without errors.
#[derive(Debug, Clone)]
pub struct Output {
    pub text: String,
    pub diagnostics: Vec<Diagnostic>,
}

/// Write a model in the canonical form.
pub fn to_string(model: &Model, options: Options) -> Output {
    let mut w = W::new(model, options);
    let stmts = w.statements();
    let mut text = String::new();
    if let Some(h) = w.header() {
        text.push_str(&h);
        text.push_str("\n\n");
    }
    let mut last: Option<usize> = None;
    for s in &stmts {
        if last.is_some_and(|l| l != s.owner) {
            text.push('\n');
        }
        last = Some(s.owner);
        text.push_str(&s.text);
        text.push('\n');
    }
    if text.ends_with("\n\n") {
        text.pop();
    }
    Output {
        text,
        diagnostics: w.diags,
    }
}

/// A link or a Fan, by index in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Elem {
    Link(usize),
    Fan(usize),
}

/// Links and Fans in the order of their canonical sentences, links of a Fan in the order the Fan
/// sentence writes them (spec/opl.md §10.4).
pub fn element_order(model: &Model) -> Vec<Elem> {
    let mut w = W::new(model, Options::default());
    let mut out = Vec::new();
    for s in w.statements() {
        match s.elem {
            Some(Elem::Fan(k)) => {
                out.push(Elem::Fan(k));
                for l in w.fan_link_order(k) {
                    out.push(Elem::Link(l));
                }
            }
            Some(e) => out.push(e),
            None => {}
        }
    }
    out
}

struct Stmt {
    owner: usize,
    rank: u8,
    sub: usize,
    text: String,
    elem: Option<Elem>,
}

struct W<'a> {
    m: &'a Model,
    ids: bool,
    diags: Vec<Diagnostic>,
    things: HashMap<&'a str, usize>,
    states: HashMap<&'a str, (usize, &'a State)>,
    links: HashMap<&'a str, usize>,
    /// Thing index → rank by name.
    by_name: Vec<usize>,
}

const KIND_ORDER: [LinkKind; 13] = [
    LinkKind::Consumption,
    LinkKind::Effect,
    LinkKind::Result,
    LinkKind::Agent,
    LinkKind::Instrument,
    LinkKind::Invocation,
    LinkKind::OvertimeException,
    LinkKind::UndertimeException,
    LinkKind::Aggregation,
    LinkKind::Exhibition,
    LinkKind::Generalization,
    LinkKind::Classification,
    LinkKind::Tagged,
];

fn kind_rank(k: LinkKind) -> usize {
    KIND_ORDER.iter().position(|x| *x == k).unwrap()
}

/// `a`, `a or b`, `a, b, or c`.
fn list(items: &[String], conj: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} {} {}", items[0], conj, items[1]),
        n => format!("{}, {} {}", items[..n - 1].join(", "), conj, items[n - 1]),
    }
}

fn block(open: char, close: char, entries: &[String]) -> String {
    if entries.is_empty() {
        String::new()
    } else {
        format!(" {}{}{}", open, entries.join(", "), close)
    }
}

/// A multiplicity in words (spec/opl.md §5.7).
fn mult_words(m: &str) -> Option<String> {
    let num = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if num(m) {
        return Some(m.to_string());
    }
    if m == "*" || m == "0..*" {
        return Some("optional".into());
    }
    let (lo, hi) = m.split_once("..")?;
    if !num(lo) {
        return None;
    }
    match (lo, hi) {
        ("0", "1") => Some("an optional".into()),
        ("1", "*") => Some("at least one".into()),
        (lo, "*") => Some(format!("{} to many", lo)),
        (lo, hi) if num(hi) => Some(format!("{} to {}", lo, hi)),
        _ => None,
    }
}

impl<'a> W<'a> {
    fn new(m: &'a Model, options: Options) -> Self {
        let things = m
            .things
            .iter()
            .enumerate()
            .map(|(k, t)| (t.id.as_str(), k))
            .collect();
        let mut states = HashMap::new();
        for (k, t) in m.things.iter().enumerate() {
            for s in t.states() {
                states.insert(s.id.as_str(), (k, s));
            }
        }
        let links = m
            .links
            .iter()
            .enumerate()
            .map(|(k, l)| (l.id.as_str(), k))
            .collect();
        let mut order: Vec<usize> = (0..m.things.len()).collect();
        order.sort_by(|a, b| m.things[*a].name.cmp(&m.things[*b].name));
        let mut by_name = vec![0; m.things.len()];
        for (r, k) in order.into_iter().enumerate() {
            by_name[k] = r;
        }
        W {
            m,
            ids: options.ids,
            diags: Vec::new(),
            things,
            states,
            links,
            by_name,
        }
    }

    fn err(&mut self, id: &str, msg: impl Into<String>) {
        self.diags
            .push(Diagnostic::error(Code::Unwritable, None, msg).with_id(id));
    }

    fn dropped(&mut self, id: &str, what: impl Into<String>) {
        self.diags.push(
            Diagnostic::warning(
                Code::Dropped,
                None,
                format!("left out of OPL: {}", what.into()),
            )
            .with_id(id),
        );
    }

    fn text_ok(&mut self, id: &str, s: &str, what: &str) {
        if !names::quotable(s) {
            self.err(
                id,
                format!("the {} holds a line break, which OPL cannot write", what),
            );
        }
    }

    fn thing_name(&self, k: usize) -> String {
        names::thing(&self.m.things[k].name)
    }

    /// The Thing an end id names (a Thing, or the owner of a State) and the State.
    fn resolve(&self, id: &str) -> Option<(usize, Option<&'a State>)> {
        if let Some(&k) = self.things.get(id) {
            return Some((k, None));
        }
        self.states.get(id).map(|(k, s)| (*k, Some(*s)))
    }

    fn end(&self, id: &str, mult: Option<&str>) -> Option<String> {
        let (k, s) = self.resolve(id)?;
        let mut out = String::new();
        if let Some(m) = mult {
            out.push_str(&mult_words(m)?);
            out.push(' ');
        }
        if let Some(s) = s {
            out.push_str(&names::state(&s.name));
            out.push(' ');
        }
        out.push_str(&self.thing_name(k));
        Some(out)
    }

    fn sort_key(&self, s: &Stmt) -> (usize, u8, usize, String) {
        (self.by_name[s.owner], s.rank, s.sub, s.text.clone())
    }

    fn header(&mut self) -> Option<String> {
        let m = self.m;
        if m.diagram.is_some() {
            self.dropped(&m.id, "the OPDs (diagram)");
        }
        if !m.extensions.is_empty() {
            self.dropped(&m.id, "the annotations of the model");
        }
        if m.name.is_none() && m.description.is_none() && !self.ids {
            return None;
        }
        let Some(name) = &m.name else {
            self.err(
                &m.id,
                "the model has no name, which the `Model` heading needs",
            );
            return None;
        };
        self.text_ok(&m.id, name, "model name");
        let mut core = Vec::new();
        if self.ids {
            core.push(format!("id: {}", m.id));
        }
        if let Some(d) = &m.description {
            self.text_ok(&m.id, d, "description");
            core.push(format!("description: {}", names::quote(d)));
        }
        Some(format!(
            "Model {}{}.",
            names::quote(name),
            block('{', '}', &core)
        ))
    }

    fn juiz(&mut self, id: &str, target: Target, ext: &opm_model::Extensions) -> String {
        let mut dropped = Vec::new();
        let mut errors = Vec::new();
        let s = juiz::from_extensions(target, ext, &mut dropped, &mut errors);
        for d in dropped {
            self.dropped(id, d);
        }
        for e in errors {
            self.err(id, e);
        }
        match s {
            Some(s) => format!(" [{}]", s),
            None => String::new(),
        }
    }

    fn statements(&mut self) -> Vec<Stmt> {
        let mut out = Vec::new();
        let m = self.m;
        for (k, t) in m.things.iter().enumerate() {
            self.thing(k, t, &mut out);
        }
        for z in &m.in_zooms {
            self.zoom(z, &mut out);
        }
        let in_fan: std::collections::HashSet<&str> = m
            .fans
            .iter()
            .flat_map(|f| f.links.iter().map(String::as_str))
            .collect();
        for (k, l) in m.links.iter().enumerate() {
            if !in_fan.contains(l.id.as_str()) {
                self.link(k, l, &mut out);
            }
        }
        for k in 0..m.fans.len() {
            self.fan(k, &mut out);
        }
        out.sort_by_key(|s| self.sort_key(s));
        out
    }

    fn thing(&mut self, k: usize, t: &'a Thing, out: &mut Vec<Stmt>) {
        let name = self.thing_name(k);
        self.text_ok(&t.id, &t.name, "name");
        // declaration
        let mut words = Vec::new();
        if t.affiliation == Affiliation::Environmental {
            words.push("environmental");
        }
        if t.essence == Essence::Physical {
            words.push("physical");
        }
        words.push(if t.is_process() { "process" } else { "object" });
        let mut core = Vec::new();
        if self.ids {
            core.push(format!("id: {}", t.id));
        }
        if let Some(d) = &t.description {
            self.text_ok(&t.id, d, "description");
            core.push(format!("description: {}", names::quote(d)));
        }
        if let ThingKind::Process { duration: Some(d) } = &t.kind {
            for (key, v) in [
                ("duration_min", d.min),
                ("duration_expected", d.expected),
                ("duration_max", d.max),
            ] {
                if let Some(v) = v {
                    core.push(format!("{}: {}", key, names::number(v)));
                }
            }
            core.push(format!("duration_unit: {}", names::atom(&d.unit)));
        }
        let juiz = self.juiz(&t.id, Target::Thing, &t.extensions);
        out.push(Stmt {
            owner: k,
            rank: 0,
            sub: 0,
            text: format!(
                "{} is {} {}{}{}.",
                name,
                names::article(words[0]),
                words.join(" "),
                block('{', '}', &core),
                juiz
            ),
            elem: None,
        });
        let states = t.states();
        if states.is_empty() {
            return;
        }
        let sn: Vec<String> = states.iter().map(|s| names::state(&s.name)).collect();
        out.push(Stmt {
            owner: k,
            rank: 1,
            sub: 0,
            text: format!("{} can be {}.", name, list(&sn, "or")),
            elem: None,
        });
        let initial: Vec<String> = states
            .iter()
            .filter(|s| s.initial)
            .map(|s| names::state(&s.name))
            .collect();
        let final_: Vec<String> = states
            .iter()
            .filter(|s| s.final_)
            .map(|s| names::state(&s.name))
            .collect();
        let ends = match (initial.is_empty(), final_.is_empty()) {
            (false, false) => Some(format!(
                "{} is initially {} and finally {}.",
                name,
                list(&initial, "or"),
                list(&final_, "or")
            )),
            (false, true) => Some(format!("{} is initially {}.", name, list(&initial, "or"))),
            (true, false) => Some(format!("{} is finally {}.", name, list(&final_, "or"))),
            (true, true) => None,
        };
        if let Some(text) = ends {
            out.push(Stmt {
                owner: k,
                rank: 2,
                sub: 0,
                text,
                elem: None,
            });
        }
        for (i, s) in states.iter().enumerate() {
            self.text_ok(&s.id, &s.name, "State name");
            let mut core = Vec::new();
            if self.ids {
                core.push(format!("id: {}", s.id));
            }
            if let Some(d) = &s.description {
                self.text_ok(&s.id, d, "description");
                core.push(format!("description: {}", names::quote(d)));
            }
            let juiz = self.juiz(&s.id, Target::State, &s.extensions);
            if !s.default && core.is_empty() && juiz.is_empty() {
                continue;
            }
            out.push(Stmt {
                owner: k,
                rank: 3,
                sub: i,
                text: format!(
                    "State {} of {}{}{}{}.",
                    names::state(&s.name),
                    name,
                    if s.default { " is default" } else { "" },
                    block('{', '}', &core),
                    juiz
                ),
                elem: None,
            });
        }
    }

    fn sorted_names(&self, ids: &[String]) -> Option<Vec<String>> {
        let mut ks = Vec::new();
        for id in ids {
            ks.push(*self.things.get(id.as_str())?);
        }
        ks.sort_by_key(|k| self.by_name[*k]);
        Some(ks.into_iter().map(|k| self.thing_name(k)).collect())
    }

    fn zoom(&mut self, z: &opm_model::InZoom, out: &mut Vec<Stmt>) {
        let Some(&k) = self.things.get(z.refinee.as_str()) else {
            self.err(&z.id, "the refinee is not a Thing");
            return;
        };
        if !z.extensions.is_empty() {
            self.dropped(&z.id, "the annotations of an in-zoom");
        }
        let refinee_is_process = self.m.things[k].is_process();
        let Some(parts) = self.sorted_names(&z.parts) else {
            self.err(&z.id, "a part is not a Thing");
            return;
        };
        let body = if refinee_is_process {
            let mut steps = Vec::new();
            let mut parallel = Vec::new();
            for step in &z.steps {
                let Some(names_) = self.sorted_names(step) else {
                    self.err(&z.id, "a step member is not a Thing");
                    return;
                };
                match names_.len() {
                    0 => {
                        self.err(&z.id, "an empty step cannot be written");
                        return;
                    }
                    1 => {
                        steps.push(names_[0].clone());
                        parallel.push(false);
                    }
                    _ => {
                        steps.push(format!("parallel {}", list(&names_, "and")));
                        parallel.push(true);
                    }
                }
            }
            let mut body = match steps.len() {
                0 => "nothing".to_string(),
                1 => steps[0].clone(),
                2 if parallel[0] => format!("{}, and {}, in that sequence", steps[0], steps[1]),
                _ => format!("{}, in that sequence", list(&steps, "and")),
            };
            if !parts.is_empty() {
                body.push_str(&format!(", as well as {}", list(&parts, "and")));
            }
            body
        } else {
            if !z.steps.is_empty() || parts.is_empty() {
                self.err(&z.id, "an Object zooms into parts only, and at least one");
                return;
            }
            list(&parts, "and")
        };
        let mut core = Vec::new();
        if self.ids {
            core.push(format!("id: {}", z.id));
        }
        if let Some(d) = &z.description {
            self.text_ok(&z.id, d, "description");
            core.push(format!("description: {}", names::quote(d)));
        }
        out.push(Stmt {
            owner: k,
            rank: 4,
            sub: 0,
            text: format!(
                "{} zooms into {}{}.",
                self.thing_name(k),
                body,
                block('{', '}', &core)
            ),
            elem: None,
        });
    }

    /// The sentence of one link without the final `.`, and the Thing it belongs to (§9.2).
    fn link_sentence(&mut self, l: &Link) -> Option<(String, usize)> {
        use LinkKind::*;
        let fail = |w: &mut Self, why: &str| {
            w.err(
                &l.id,
                format!("this {} cannot be written: {}", l.kind.name(), why),
            );
            None
        };
        let (Some((sk, ss)), Some((tk, ts))) = (self.resolve(&l.source), self.resolve(&l.target))
        else {
            return fail(self, "an end is not a Thing or State");
        };
        if l.modifier.is_some()
            && (l.source_multiplicity.is_some() || l.target_multiplicity.is_some())
        {
            return fail(self, "it has both a modifier and a multiplicity");
        }
        if l.modifier.is_some() && !l.kind.takes_modifier() {
            return fail(self, "this kind takes no modifier");
        }
        if l.path.is_some() && l.kind.is_structural() {
            return fail(self, "a structural link takes no path label");
        }
        let src = self.end(&l.source, l.source_multiplicity.as_deref());
        let tgt = self.end(&l.target, l.target_multiplicity.as_deref());
        let (Some(src), Some(tgt)) = (src, tgt) else {
            return fail(self, "a multiplicity is malformed");
        };
        let sname = self.thing_name(sk);
        let tname = self.thing_name(tk);
        let cond = |k: usize, s: Option<&State>, w: &Self| match s {
            Some(s) => format!("{} is {}", w.thing_name(k), names::state(&s.name)),
            None => format!("{} exists", w.thing_name(k)),
        };
        let state_of = |w: &Self, id: &Option<String>| -> std::result::Result<Option<String>, ()> {
            match id {
                None => Ok(None),
                Some(id) => match w.states.get(id.as_str()) {
                    Some((k, s)) if *k == tk => Ok(Some(names::state(&s.name))),
                    _ => Err(()),
                },
            }
        };
        let (Ok(from), Ok(to)) = (state_of(self, &l.from_state), state_of(self, &l.to_state))
        else {
            return fail(
                self,
                "a from-state or to-state is not a State of the target",
            );
        };
        let change = match (&from, &to) {
            (Some(f), Some(t)) => Some(format!(" from {} to {}", f, t)),
            (Some(f), None) => Some(format!(" from {}", f)),
            (None, Some(t)) => Some(format!(" to {}", t)),
            (None, None) => None,
        };
        let plain_only = |w: &mut Self, ok: bool| {
            if ok {
                Some(())
            } else {
                w.err(
                    &l.id,
                    format!(
                        "this {} cannot be written: a State end is not allowed here",
                        l.kind.name()
                    ),
                );
                None
            }
        };
        let text = match (l.kind, l.modifier) {
            (Consumption | Instrument, None) => {
                plain_only(self, ts.is_none())?;
                let verb = if l.kind == Consumption {
                    "consumes"
                } else {
                    "requires"
                };
                format!("{} {} {}", tgt, verb, src)
            }
            (Consumption | Instrument, Some(Modifier::Event)) => {
                plain_only(self, ts.is_none())?;
                let verb = if l.kind == Consumption {
                    "consumes"
                } else {
                    "requires"
                };
                format!("{} initiates {}, which {} {}", src, tname, verb, src)
            }
            (Consumption, Some(Modifier::Condition)) => {
                plain_only(self, ts.is_none())?;
                format!(
                    "{p} occurs if {c}, in which case {p} consumes {e}, otherwise {p} is skipped",
                    p = tname,
                    c = cond(sk, ss, self),
                    e = src
                )
            }
            (Instrument, Some(Modifier::Condition)) => {
                plain_only(self, ts.is_none())?;
                format!(
                    "{p} occurs if {c}, otherwise {p} is skipped",
                    p = tname,
                    c = cond(sk, ss, self)
                )
            }
            (Agent, None) => {
                plain_only(self, ts.is_none())?;
                format!("{} handles {}", src, tgt)
            }
            (Agent, Some(Modifier::Event)) => {
                plain_only(self, ts.is_none())?;
                format!("{} initiates and handles {}", src, tname)
            }
            (Agent, Some(Modifier::Condition)) => {
                plain_only(self, ts.is_none())?;
                format!(
                    "{} handles {p} if {c}, otherwise {p} is skipped",
                    src,
                    p = tname,
                    c = cond(sk, ss, self)
                )
            }
            (Result, None) => {
                plain_only(self, ss.is_none())?;
                format!("{} yields {}", src, tgt)
            }
            (Effect, m) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                let act = match &change {
                    Some(c) => format!("changes {}{}", tname, c),
                    None => format!("affects {}", tname),
                };
                match m {
                    None => match &change {
                        Some(c) => format!("{} changes {}{}", src, tname, c),
                        None => format!("{} affects {}", src, tgt),
                    },
                    Some(Modifier::Event) => {
                        format!("{} initiates {}, which {}", tname, sname, act)
                    }
                    Some(Modifier::Condition) => {
                        let c = match &from {
                            Some(f) => format!("{} is {}", tname, f),
                            None => format!("{} exists", tname),
                        };
                        format!(
                            "{p} occurs if {c}, in which case {p} {act}, otherwise {p} is skipped",
                            p = sname
                        )
                    }
                }
            }
            (Invocation, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                if l.source == l.target {
                    if l.target_multiplicity.is_some() {
                        return fail(self, "`invokes itself` takes no multiplicity on the target");
                    }
                    format!("{} invokes itself", src)
                } else {
                    format!("{} invokes {}", src, tgt)
                }
            }
            (OvertimeException | UndertimeException, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                if l.source_multiplicity.is_some() || l.target_multiplicity.is_some() {
                    return fail(self, "an exception link takes no multiplicity");
                }
                let d = match &self.m.things[sk].kind {
                    ThingKind::Process { duration: Some(d) } => d,
                    _ => return fail(self, "the source has no duration"),
                };
                let (x, verb) = if l.kind == OvertimeException {
                    (d.max, "exceeds")
                } else {
                    (d.min, "falls short of")
                };
                let Some(x) = x else {
                    return fail(
                        self,
                        "the source has no maximum (overtime) or minimum (undertime) duration",
                    );
                };
                format!(
                    "{} occurs if duration of {} {} {} {}",
                    tname,
                    sname,
                    verb,
                    names::number(x),
                    names::atom(&d.unit)
                )
            }
            (Aggregation, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                format!("{} consists of {}", src, tgt)
            }
            (Exhibition, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                format!("{} exhibits {}", src, tgt)
            }
            (Generalization | Classification, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                if l.source_multiplicity.is_some() || l.target_multiplicity.is_some() {
                    return fail(self, "it takes no multiplicity");
                }
                if l.kind == Generalization {
                    format!("{} is {} {}", tname, names::article(&sname), sname)
                } else {
                    format!("{} is an instance of {}", tname, sname)
                }
            }
            (Tagged, None) => {
                plain_only(self, ss.is_none() && ts.is_none())?;
                let Some(tag) = &l.tag else {
                    return fail(self, "it has no tag");
                };
                self.text_ok(&l.id, tag, "tag");
                if l.target_multiplicity
                    .as_deref()
                    .and_then(mult_words)
                    .is_some_and(|w| !w.starts_with(|c: char| c.is_ascii_digit()))
                {
                    return fail(
                        self,
                        "the target multiplicity would read as part of the tag",
                    );
                }
                match &l.backward_tag {
                    Some(b) if b == tag => {
                        if l.source_multiplicity.is_some() || l.target_multiplicity.is_some() {
                            return fail(self, "a reciprocal tagged link takes no multiplicity");
                        }
                        format!("{} and {} are {}", sname, tname, names::tag(tag))
                    }
                    Some(b) => {
                        self.text_ok(&l.id, b, "backward tag");
                        format!(
                            "{} {} {}, and {} {} {}",
                            src,
                            names::tag(tag),
                            tgt,
                            tname,
                            names::tag(b),
                            sname
                        )
                    }
                    None => format!("{} {} {}", src, names::tag(tag), tgt),
                }
            }
            _ => return fail(self, "this kind takes no modifier"),
        };
        let owner = match l.kind {
            Consumption | Instrument | Agent => tk,
            Generalization | Classification | Tagged | Aggregation | Exhibition => sk,
            _ => sk,
        };
        let text = match &l.path {
            Some(p) => {
                self.text_ok(&l.id, p, "path label");
                format!("Following path {}, {}", names::state(p), text)
            }
            None => text,
        };
        Some((text, owner))
    }

    fn link(&mut self, k: usize, l: &Link, out: &mut Vec<Stmt>) {
        let Some((text, owner)) = self.link_sentence(l) else {
            return;
        };
        let mut core = Vec::new();
        if self.ids {
            core.push(format!("id: {}", l.id));
        }
        if let Some(d) = &l.description {
            self.text_ok(&l.id, d, "description");
            core.push(format!("description: {}", names::quote(d)));
        }
        if l.ordered {
            if l.kind == LinkKind::Aggregation {
                core.push("ordered: true".to_string());
            } else {
                self.err(&l.id, "only an aggregation can be ordered");
            }
        }
        let juiz = self.juiz(&l.id, Target::Link, &l.extensions);
        out.push(Stmt {
            owner,
            rank: 5,
            sub: kind_rank(l.kind),
            text: format!("{}{}{}.", text, block('{', '}', &core), juiz),
            elem: Some(Elem::Link(k)),
        });
    }

    /// The links of a Fan in the order its sentence writes the ends.
    fn fan_link_order(&self, k: usize) -> Vec<usize> {
        let f = &self.m.fans[k];
        let mut ls: Vec<usize> = f
            .links
            .iter()
            .filter_map(|id| self.links.get(id.as_str()).copied())
            .collect();
        ls.sort_by_key(|&i| {
            let l = &self.m.links[i];
            let end = if l.kind.is_enabling_or_consuming() {
                &l.source
            } else {
                &l.target
            };
            match self.resolve(end) {
                Some((t, s)) => (self.by_name[t], s.map(|s| s.name.clone())),
                None => (usize::MAX, None),
            }
        });
        ls
    }

    fn fan(&mut self, k: usize, out: &mut Vec<Stmt>) {
        let m: &'a Model = self.m;
        let f = &m.fans[k];
        let fail =
            |w: &mut Self, why: &str| w.err(&f.id, format!("this fan cannot be written: {}", why));
        let order = self.fan_link_order(k);
        if order.len() != f.links.len() {
            return fail(self, "a member is not a link");
        }
        if order.len() < 2 {
            return fail(self, "it has fewer than two links");
        }
        let links: Vec<&Link> = order.iter().map(|&i| &m.links[i]).collect();
        let kind = links[0].kind;
        if links.iter().any(|l| l.kind != kind) {
            return fail(self, "its links are of different kinds");
        }
        let verb = match kind {
            LinkKind::Consumption => "consumes",
            LinkKind::Result => "yields",
            LinkKind::Effect => "affects",
            LinkKind::Instrument => "requires",
            LinkKind::Invocation => "invokes",
            LinkKind::Agent => "handles",
            _ => return fail(self, "a fan of this kind has no sentence"),
        };
        for l in &links {
            if l.modifier.is_some()
                || l.path.is_some()
                || l.source_multiplicity.is_some()
                || l.target_multiplicity.is_some()
                || !l.extensions.is_empty()
                || l.description.is_some()
            {
                return fail(
                    self,
                    "a link has a modifier, path label, multiplicity, annotation or description",
                );
            }
            if l.from_state.is_some() || l.to_state.is_some() {
                return fail(self, "an effect of a fan has a state change");
            }
        }
        let process = links[0].process_end().to_string();
        if links.iter().any(|l| l.process_end() != process) {
            return fail(self, "its links do not share the process end");
        }
        let Some(&pk) = self.things.get(process.as_str()) else {
            return fail(self, "the process end is not a Thing");
        };
        let mut ends = Vec::new();
        for l in &links {
            let id = if kind.is_enabling_or_consuming() {
                &l.source
            } else {
                &l.target
            };
            let Some(e) = self.end(id, None) else {
                return fail(self, "an end is not a Thing or State");
            };
            if matches!(kind, LinkKind::Effect | LinkKind::Invocation)
                && self.states.contains_key(id.as_str())
            {
                return fail(self, "a State end is not allowed here");
            }
            ends.push(e);
        }
        let agent = kind == LinkKind::Agent;
        let cap = |s: &str| -> String {
            if agent {
                let mut c = s.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap()
            } else {
                s.to_string()
            }
        };
        let phrase = match (f.operator, ends.len()) {
            (FanOperator::Xor, 2) => format!("{} {} or {}", cap("either"), ends[0], ends[1]),
            (FanOperator::Xor, _) => format!("{} {}", cap("exactly one of"), list(&ends, "or")),
            (FanOperator::Or, _) => format!("{} {}", cap("at least one of"), list(&ends, "or")),
        };
        let pname = self.thing_name(pk);
        let text = if agent {
            format!("{} handles {}", phrase, pname)
        } else {
            format!("{} {} {}", pname, verb, phrase)
        };
        if !f.extensions.is_empty() {
            self.dropped(&f.id, "the annotations of a fan");
        }
        let mut core = Vec::new();
        if self.ids {
            core.push(format!("id: {}", f.id));
        }
        if let Some(d) = &f.description {
            self.text_ok(&f.id, d, "description");
            core.push(format!("description: {}", names::quote(d)));
        }
        if self.ids {
            let ids: Vec<&str> = links.iter().map(|l| l.id.as_str()).collect();
            core.push(format!("link_ids: {}", ids.join(" ")));
        }
        out.push(Stmt {
            owner: pk,
            rank: 5,
            sub: kind_rank(kind),
            text: format!("{}{}.", text, block('{', '}', &core)),
            elem: Some(Elem::Fan(k)),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_mults() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(list(&v(&["a", "b"]), "or"), "a or b");
        assert_eq!(list(&v(&["a", "b", "c"]), "or"), "a, b, or c");
        assert_eq!(mult_words("0..1").unwrap(), "an optional");
        assert_eq!(mult_words("2..*").unwrap(), "2 to many");
        assert_eq!(mult_words("3..5").unwrap(), "3 to 5");
        assert_eq!(mult_words("*").unwrap(), "optional");
        assert!(mult_words("x").is_none());
    }
}
