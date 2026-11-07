// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The core well-formedness rules C1 to C12 (spec/metamodel.md §8) and the form of values
//! (F1 to F4, spec/xml.md §14).
//!
//! The checker looks at every rule, including those the XSD already checks, because a model can
//! also be built in code. The juiz rules (J1 to J13) are not checked here (Step 7).

use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::model::*;

/// The rule a diagnostic is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Rule {
    /// The form of an id: `[A-Za-z_][A-Za-z0-9_.-]*`.
    F1,
    /// The form of a name, tag, path or unit: not empty, no white space at either end.
    F2,
    /// The form of a multiplicity: `1`, `*`, `0..1`, `2..*`, `3..5`.
    F3,
    /// The format version.
    F4,
    C1,
    C2,
    C3,
    C4,
    C5,
    C6,
    C7,
    C8,
    C9,
    C10,
    C11,
    C12,
}

impl fmt::Display for Rule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One broken rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: Rule,
    pub severity: Severity,
    /// The id of the element the rule is about.
    pub id: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}[{}] {}: {}", s, self.rule, self.id, self.message)
    }
}

/// The result of [`check`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    /// True when there is no error. Warnings do not count.
    pub fn is_ok(&self) -> bool {
        self.errors().next().is_none()
    }

    /// Whether some diagnostic is about this rule.
    pub fn has(&self, rule: Rule) -> bool {
        self.diagnostics.iter().any(|d| d.rule == rule)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for d in &self.diagnostics {
            writeln!(f, "{}", d)?;
        }
        Ok(())
    }
}

/// Check a model against the core rules.
pub fn check(model: &Model) -> Report {
    let mut c = Checker::new(model);
    c.form();
    c.c1_ids();
    c.c2_names();
    c.links();
    c.c7_defaults();
    c.c8_changes();
    c.c9_in_zooms();
    c.c10_diagrams();
    c.c11_fans();
    c.c12_cycles();
    Report { diagnostics: c.out }
}

/// What an id points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Model,
    Object(usize),
    Process(usize),
    State(usize),
    Link(usize),
    Fan,
    InZoom,
    Diagram,
    Appearance,
}

impl Target {
    fn describe(self) -> &'static str {
        match self {
            Target::Model => "the model",
            Target::Object(_) => "an object",
            Target::Process(_) => "a process",
            Target::State(_) => "a state",
            Target::Link(_) => "a link",
            Target::Fan => "a fan",
            Target::InZoom => "an in-zoom",
            Target::Diagram => "a diagram",
            Target::Appearance => "an appearance",
        }
    }

    fn is_thing(self) -> bool {
        matches!(self, Target::Object(_) | Target::Process(_))
    }
}

struct Checker<'m> {
    m: &'m Model,
    ids: HashMap<&'m str, Target>,
    duplicates: Vec<&'m str>,
    out: Vec<Diagnostic>,
}

fn id_form_ok(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

fn name_form_ok(name: &str) -> bool {
    !name.is_empty() && name.trim() == name
}

fn multiplicity_form_ok(m: &str) -> bool {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if m == "*" || digits(m) {
        return true;
    }
    match m.split_once("..") {
        Some((lo, hi)) => digits(lo) && (hi == "*" || digits(hi)),
        None => false,
    }
}

impl<'m> Checker<'m> {
    fn new(m: &'m Model) -> Self {
        let mut c = Checker {
            m,
            ids: HashMap::new(),
            duplicates: Vec::new(),
            out: Vec::new(),
        };
        c.index();
        c
    }

    fn error(&mut self, rule: Rule, id: &str, message: impl Into<String>) {
        self.out.push(Diagnostic {
            rule,
            severity: Severity::Error,
            id: id.to_string(),
            message: message.into(),
        });
    }

    fn warning(&mut self, rule: Rule, id: &str, message: impl Into<String>) {
        self.out.push(Diagnostic {
            rule,
            severity: Severity::Warning,
            id: id.to_string(),
            message: message.into(),
        });
    }

    fn insert(&mut self, id: &'m str, t: Target) {
        if self.ids.contains_key(id) {
            self.duplicates.push(id);
        } else {
            self.ids.insert(id, t);
        }
    }

    fn index(&mut self) {
        let m = self.m;
        self.insert(&m.id, Target::Model);
        for (i, t) in m.things.iter().enumerate() {
            self.insert(
                &t.id,
                if t.is_object() {
                    Target::Object(i)
                } else {
                    Target::Process(i)
                },
            );
            for s in t.states() {
                self.insert(&s.id, Target::State(i));
            }
        }
        for (i, l) in m.links.iter().enumerate() {
            self.insert(&l.id, Target::Link(i));
        }
        for f in &m.fans {
            self.insert(&f.id, Target::Fan);
        }
        for z in &m.in_zooms {
            self.insert(&z.id, Target::InZoom);
        }
        for d in m.diagrams() {
            self.insert(&d.id, Target::Diagram);
            for a in &d.appearances {
                self.insert(&a.id, Target::Appearance);
            }
        }
    }

    /// Every id in the model with the element it belongs to (for F1).
    fn all_ids(&self) -> Vec<&'m str> {
        let m = self.m;
        let mut v = vec![m.id.as_str()];
        for t in &m.things {
            v.push(&t.id);
            v.extend(t.states().iter().map(|s| s.id.as_str()));
        }
        v.extend(m.links.iter().map(|l| l.id.as_str()));
        v.extend(m.fans.iter().map(|f| f.id.as_str()));
        v.extend(m.in_zooms.iter().map(|z| z.id.as_str()));
        for d in m.diagrams() {
            v.push(&d.id);
            v.extend(d.appearances.iter().map(|a| a.id.as_str()));
        }
        v
    }

    /// Resolve a reference. A missing id is reported under C1 and gives `None`.
    fn resolve(&mut self, from: &str, what: &str, id: &str) -> Option<Target> {
        match self.ids.get(id) {
            Some(t) => Some(*t),
            None => {
                self.error(Rule::C1, from, format!("{} {:?} does not exist", what, id));
                None
            }
        }
    }

    // ----- F1 to F4 -----

    fn form(&mut self) {
        let m = self.m;
        for id in self.all_ids() {
            if !id_form_ok(id) {
                self.error(Rule::F1, id, "an id is ASCII: [A-Za-z_][A-Za-z0-9_.-]*");
            }
        }
        if let Some(n) = &m.name {
            self.name_form(&m.id, "name", n);
        }
        for t in &m.things {
            self.name_form(&t.id, "name", &t.name);
            for s in t.states() {
                self.name_form(&s.id, "name", &s.name);
            }
            if let ThingKind::Process { duration: Some(d) } = &t.kind {
                if d.unit.trim().is_empty() {
                    self.error(Rule::F2, &t.id, "the duration needs a unit");
                }
            }
        }
        for l in &m.links {
            for (what, v) in [
                ("tag", &l.tag),
                ("backward_tag", &l.backward_tag),
                ("path", &l.path),
            ] {
                if let Some(v) = v {
                    self.name_form(&l.id, what, v);
                }
            }
            for (what, v) in [
                ("source_multiplicity", &l.source_multiplicity),
                ("target_multiplicity", &l.target_multiplicity),
            ] {
                if let Some(v) = v {
                    if !multiplicity_form_ok(v) {
                        self.error(
                            Rule::F3,
                            &l.id,
                            format!("{} {:?} is not 1, *, 0..1, 2..* or 3..5", what, v),
                        );
                    }
                }
            }
        }
        for d in m.diagrams() {
            self.name_form(&d.id, "name", &d.name);
        }
        if m.format_version != FORMAT_VERSION {
            self.error(
                Rule::F4,
                &m.id,
                format!(
                    "format_version {:?} is not {}",
                    m.format_version, FORMAT_VERSION
                ),
            );
        }
    }

    fn name_form(&mut self, id: &str, what: &str, v: &str) {
        if !name_form_ok(v) {
            self.error(
                Rule::F2,
                id,
                format!("{} {:?} is empty or has white space at an end", what, v),
            );
        }
    }

    // ----- C1, C2 -----

    fn c1_ids(&mut self) {
        let dups = std::mem::take(&mut self.duplicates);
        for id in dups {
            self.error(Rule::C1, id, "the id is used by more than one element");
        }
    }

    fn c2_names(&mut self) {
        let m = self.m;
        let mut seen = HashSet::new();
        for t in &m.things {
            if !seen.insert(t.name.as_str()) {
                self.error(
                    Rule::C2,
                    &t.id,
                    format!("another thing is also named {:?}", t.name),
                );
            }
            let mut states = HashSet::new();
            for s in t.states() {
                if !states.insert(s.name.as_str()) {
                    self.error(
                        Rule::C2,
                        &s.id,
                        format!("{:?} has another state named {:?}", t.name, s.name),
                    );
                }
            }
        }
    }

    // ----- C3 to C6 -----

    fn links(&mut self) {
        let m = self.m;
        for l in &m.links {
            let src = self.resolve(&l.id, "source", &l.source);
            let tgt = self.resolve(&l.id, "target", &l.target);
            if l.kind.is_procedural() {
                self.c3_procedural(l, src, tgt);
            } else {
                self.c6_structural(l, src, tgt);
            }
            // C4
            if l.modifier.is_some() && !l.kind.takes_modifier() {
                self.error(
                    Rule::C4,
                    &l.id,
                    format!("{} takes no modifier", l.kind.name()),
                );
            }
            self.c5_states(l, tgt);
            // Attributes that belong to other kinds (C6 for structural ones).
            if l.ordered && l.kind != LinkKind::Aggregation {
                self.error(
                    Rule::C6,
                    &l.id,
                    format!("ordered is for aggregation, not {}", l.kind.name()),
                );
            }
            if l.kind != LinkKind::Tagged && (l.tag.is_some() || l.backward_tag.is_some()) {
                self.error(
                    Rule::C6,
                    &l.id,
                    format!("tag is for tagged links, not {}", l.kind.name()),
                );
            }
            if l.kind == LinkKind::Tagged && l.tag.is_none() {
                self.error(Rule::C6, &l.id, "a tagged link needs a tag");
            }
        }
    }

    fn c3_procedural(&mut self, l: &Link, src: Option<Target>, tgt: Option<Target>) {
        let object_or_state = |t: Target| matches!(t, Target::Object(_) | Target::State(_));
        let process = |t: Target| matches!(t, Target::Process(_));
        let object = |t: Target| matches!(t, Target::Object(_));
        type End = fn(Target) -> bool;
        let (src_ok, src_want, tgt_ok, tgt_want): (End, &str, End, &str) = match l.kind {
            k if k.is_enabling_or_consuming() => (
                object_or_state,
                "an object or a state",
                process,
                "a process",
            ),
            LinkKind::Result => (
                process,
                "a process",
                object_or_state,
                "an object or a state",
            ),
            LinkKind::Effect => (process, "a process", object, "an object"),
            _ => (process, "a process", process, "a process"),
        };
        for (end, t, ok, want) in [
            ("source", src, src_ok, src_want),
            ("target", tgt, tgt_ok, tgt_want),
        ] {
            if let Some(t) = t {
                if !ok(t) {
                    self.error(
                        Rule::C3,
                        &l.id,
                        format!(
                            "the {} of {} is {}, not {}",
                            end,
                            l.kind.name(),
                            want,
                            t.describe()
                        ),
                    );
                }
            }
        }
    }

    fn c6_structural(&mut self, l: &Link, src: Option<Target>, tgt: Option<Target>) {
        let (Some(s), Some(t)) = (src, tgt) else {
            return;
        };
        if !s.is_thing() || !t.is_thing() {
            self.error(
                Rule::C6,
                &l.id,
                format!("both ends of {} are things", l.kind.name()),
            );
            return;
        }
        let same_kind = matches!(
            (s, t),
            (Target::Object(_), Target::Object(_)) | (Target::Process(_), Target::Process(_))
        );
        match l.kind {
            LinkKind::Aggregation | LinkKind::Generalization | LinkKind::Classification
                if !same_kind =>
            {
                self.error(
                    Rule::C6,
                    &l.id,
                    format!(
                        "both ends of {} are objects, or both are processes",
                        l.kind.name()
                    ),
                );
            }
            _ => {}
        }
    }

    fn c5_states(&mut self, l: &Link, tgt: Option<Target>) {
        for (what, s) in [("from_state", &l.from_state), ("to_state", &l.to_state)] {
            let Some(s) = s else { continue };
            if l.kind != LinkKind::Effect {
                self.error(
                    Rule::C5,
                    &l.id,
                    format!("{} is for effect links, not {}", what, l.kind.name()),
                );
                continue;
            }
            match self.resolve(&l.id, what, s) {
                Some(Target::State(owner)) => {
                    if let Some(Target::Object(o)) = tgt {
                        if owner != o {
                            self.error(
                                Rule::C5,
                                &l.id,
                                format!(
                                    "{} {:?} is a state of {:?}, not of the target {:?}",
                                    what, s, self.m.things[owner].name, self.m.things[o].name
                                ),
                            );
                        }
                    }
                }
                Some(other) => self.error(
                    Rule::C5,
                    &l.id,
                    format!("{} {:?} is {}, not a state", what, s, other.describe()),
                ),
                None => {}
            }
        }
    }

    // ----- C7 -----

    fn c7_defaults(&mut self) {
        let m = self.m;
        for t in &m.things {
            if t.states().iter().filter(|s| s.default).count() > 1 {
                self.error(Rule::C7, &t.id, "more than one state is the default");
            }
        }
    }

    // ----- C8 -----

    /// The thing id of a link end; a state's owner for a state.
    fn end_thing(&self, id: &str) -> Option<&'m str> {
        match self.ids.get(id)? {
            Target::Object(i) | Target::Process(i) | Target::State(i) => {
                Some(self.m.things[*i].id.as_str())
            }
            _ => None,
        }
    }

    fn c8_changes(&mut self) {
        let m = self.m;
        // Processes that have a consumption, result or effect link of their own.
        let mut changing: HashSet<&str> = HashSet::new();
        for l in &m.links {
            let end = match l.kind {
                LinkKind::Consumption => self.end_thing(&l.target),
                LinkKind::Result | LinkKind::Effect => self.end_thing(&l.source),
                _ => None,
            };
            if let Some(p) = end {
                changing.insert(p);
            }
        }
        for t in m.things.iter().filter(|t| t.is_process()) {
            // The process itself and, if it is in-zoomed, every process inside (nested too).
            let mut seen = HashSet::new();
            let mut stack = vec![t.id.as_str()];
            let mut found = false;
            while let Some(p) = stack.pop() {
                if !seen.insert(p) {
                    continue;
                }
                if changing.contains(p) {
                    found = true;
                    break;
                }
                if let Some(z) = m.in_zoom_of(p) {
                    stack.extend(z.steps.iter().flatten().map(String::as_str));
                }
            }
            if !found {
                self.warning(
                    Rule::C8,
                    &t.id,
                    format!(
                        "{:?} has no consumption, result or effect link, so it changes nothing",
                        t.name
                    ),
                );
            }
        }
    }

    // ----- C9 -----

    fn c9_in_zooms(&mut self) {
        let m = self.m;
        let mut refinees: HashSet<&str> = HashSet::new();
        // inner thing -> the in-zoom that holds it
        let mut owner: HashMap<&str, &InZoom> = HashMap::new();
        for z in &m.in_zooms {
            let refinee = self.resolve(&z.id, "refinee", &z.refinee);
            match refinee {
                Some(t) if t.is_thing() => {}
                Some(t) => self.error(
                    Rule::C9,
                    &z.id,
                    format!("the refinee is {}, not a thing", t.describe()),
                ),
                None => {}
            }
            if !refinees.insert(z.refinee.as_str()) {
                self.error(
                    Rule::C9,
                    &z.id,
                    format!("{:?} has more than one in-zoom", z.refinee),
                );
            }
            if matches!(refinee, Some(Target::Object(_))) && !z.steps.is_empty() {
                self.error(Rule::C9, &z.id, "an in-zoomed object has no steps");
            }
            for (i, step) in z.steps.iter().enumerate() {
                if step.is_empty() {
                    self.error(Rule::C9, &z.id, format!("step {} has no member", i + 1));
                }
                for p in step {
                    match self.resolve(&z.id, "step member", p) {
                        Some(Target::Process(_)) | None => {}
                        Some(t) => self.error(
                            Rule::C9,
                            &z.id,
                            format!("step member {:?} is {}, not a process", p, t.describe()),
                        ),
                    }
                }
            }
            for p in &z.parts {
                match self.resolve(&z.id, "part", p) {
                    Some(t) if !t.is_thing() => self.error(
                        Rule::C9,
                        &z.id,
                        format!("part {:?} is {}, not a thing", p, t.describe()),
                    ),
                    _ => {}
                }
            }
            let mut inside = HashSet::new();
            for x in z.inner() {
                if x == &z.refinee {
                    self.error(
                        Rule::C9,
                        &z.id,
                        format!("{:?} is inside its own in-zoom", x),
                    );
                }
                if !inside.insert(x.as_str()) {
                    self.error(
                        Rule::C9,
                        &z.id,
                        format!("{:?} appears more than once in the in-zoom", x),
                    );
                    continue;
                }
                match owner.get(x.as_str()) {
                    Some(other) => self.error(
                        Rule::C9,
                        &z.id,
                        format!("{:?} is also inside the in-zoom {:?}", x, other.id),
                    ),
                    None => {
                        owner.insert(x.as_str(), z);
                    }
                }
            }
        }
        // Nesting has no cycle: walk from each refinee to the in-zoom that holds it.
        let mut reported = HashSet::new();
        for z in &m.in_zooms {
            let mut seen = HashSet::new();
            let mut cur = z.refinee.as_str();
            while let Some(parent) = owner.get(cur) {
                if !seen.insert(cur) {
                    break;
                }
                if parent.refinee == z.refinee {
                    if reported.insert(z.id.as_str()) {
                        self.error(
                            Rule::C9,
                            &z.id,
                            format!("the in-zoom of {:?} is nested in itself", z.refinee),
                        );
                    }
                    break;
                }
                cur = parent.refinee.as_str();
            }
        }
    }

    // ----- C10 -----

    fn c10_diagrams(&mut self) {
        let Some(root) = &self.m.diagram else { return };
        if root.refinement != Refinement::Root {
            self.error(Rule::C10, &root.id, "the root diagram is not a refinement");
        }
        if root.refinee.is_some() {
            self.error(Rule::C10, &root.id, "the root diagram has no refinee");
        }
        self.c10_walk(root);
    }

    fn c10_walk(&mut self, d: &'m Diagram) {
        for c in &d.children {
            if c.refinement == Refinement::Root {
                self.error(Rule::C10, &c.id, "only the root diagram is the root");
            }
            let needs_refinee = matches!(c.refinement, Refinement::InZoom | Refinement::Unfold);
            match &c.refinee {
                None if needs_refinee => self.error(
                    Rule::C10,
                    &c.id,
                    "an in_zoom or unfold diagram needs a refinee",
                ),
                None => {}
                Some(r) => match self.resolve(&c.id, "refinee", r) {
                    Some(t) if !t.is_thing() => self.error(
                        Rule::C10,
                        &c.id,
                        format!("the refinee {:?} is {}, not a thing", r, t.describe()),
                    ),
                    Some(_)
                        if c.refinement == Refinement::InZoom && self.m.in_zoom_of(r).is_none() =>
                    {
                        self.error(Rule::C10, &c.id, format!("{:?} has no in-zoom to show", r));
                    }
                    _ => {}
                },
            }
            self.c10_walk(c);
        }
        for a in &d.appearances {
            let thing = self.resolve(&a.id, "thing", &a.thing);
            match thing {
                Some(t) if !t.is_thing() => self.error(
                    Rule::C10,
                    &a.id,
                    format!("{:?} is {}, not a thing", a.thing, t.describe()),
                ),
                _ => {}
            }
            for s in &a.suppressed_states {
                match self.resolve(&a.id, "suppressed_state", s) {
                    Some(Target::State(owner)) => {
                        if thing.is_some() && self.m.things[owner].id != a.thing {
                            self.error(
                                Rule::C10,
                                &a.id,
                                format!("the state {:?} is not a state of {:?}", s, a.thing),
                            );
                        }
                    }
                    Some(t) => self.error(
                        Rule::C10,
                        &a.id,
                        format!("suppressed_state {:?} is {}, not a state", s, t.describe()),
                    ),
                    None => {}
                }
            }
        }
    }

    // ----- C11 -----

    fn c11_fans(&mut self) {
        let m = self.m;
        for f in &m.fans {
            let mut set = HashSet::new();
            for r in &f.links {
                if !set.insert(r.as_str()) {
                    self.error(
                        Rule::C11,
                        &f.id,
                        format!("the link {:?} is in the fan twice", r),
                    );
                }
            }
            if set.len() < 2 {
                self.error(Rule::C11, &f.id, "a fan has two or more links");
            }
            let mut first: Option<&Link> = None;
            for r in &f.links {
                let l = match self.resolve(&f.id, "link", r) {
                    Some(Target::Link(i)) => &m.links[i],
                    Some(t) => {
                        self.error(
                            Rule::C11,
                            &f.id,
                            format!("{:?} is {}, not a link", r, t.describe()),
                        );
                        continue;
                    }
                    None => continue,
                };
                if !l.kind.is_procedural() {
                    self.error(
                        Rule::C11,
                        &f.id,
                        format!("{:?} is a structural link; fans are of procedural links", r),
                    );
                    continue;
                }
                match first {
                    None => first = Some(l),
                    Some(a) => {
                        if a.kind != l.kind {
                            self.error(
                                Rule::C11,
                                &f.id,
                                format!(
                                    "{:?} is {} but {:?} is {}",
                                    a.id,
                                    a.kind.name(),
                                    l.id,
                                    l.kind.name()
                                ),
                            );
                        } else if self.end_thing(a.process_end()) != self.end_thing(l.process_end())
                        {
                            self.error(
                                Rule::C11,
                                &f.id,
                                format!("{:?} and {:?} are on different processes", a.id, l.id),
                            );
                        }
                    }
                }
            }
        }
    }

    // ----- C12 -----

    fn c12_cycles(&mut self) {
        for kind in [LinkKind::Generalization, LinkKind::Classification] {
            let mut edges: HashMap<&str, Vec<&Link>> = HashMap::new();
            for l in self.m.links.iter().filter(|l| l.kind == kind) {
                edges.entry(l.source.as_str()).or_default().push(l);
            }
            // Depth-first search; a back edge closes a cycle. Report its link once.
            #[derive(Clone, Copy, PartialEq)]
            enum Mark {
                Open,
                Done,
            }
            let mut mark: HashMap<&str, Mark> = HashMap::new();
            let mut bad: Vec<&Link> = Vec::new();
            let mut starts: Vec<&str> = edges.keys().copied().collect();
            starts.sort();
            for s in starts {
                if mark.contains_key(s) {
                    continue;
                }
                // (node, next edge index)
                let mut stack: Vec<(&str, usize)> = vec![(s, 0)];
                mark.insert(s, Mark::Open);
                while let Some((node, i)) = stack.pop() {
                    let out = edges.get(node).map(Vec::as_slice).unwrap_or(&[]);
                    if i < out.len() {
                        stack.push((node, i + 1));
                        let l = out[i];
                        let next = l.target.as_str();
                        match mark.get(next) {
                            Some(Mark::Open) => bad.push(l),
                            Some(Mark::Done) => {}
                            None => {
                                mark.insert(next, Mark::Open);
                                stack.push((next, 0));
                            }
                        }
                    } else {
                        mark.insert(node, Mark::Done);
                    }
                }
            }
            for l in bad {
                self.error(
                    Rule::C12,
                    &l.id,
                    format!("{} links make a cycle through this link", kind.name()),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Model {
        let mut m = Model::new("m");
        let mut a = Thing::object("a", "A");
        if let ThingKind::Object { states } = &mut a.kind {
            states.push(State::new("a_s1", "s1"));
            states.push(State::new("a_s2", "s2"));
        }
        m.things.push(a);
        m.things.push(Thing::object("b", "B"));
        m.things.push(Thing::process("p", "P"));
        m.links.push(Link::new("l_res", LinkKind::Result, "p", "b"));
        m
    }

    fn rules(m: &Model) -> Vec<Rule> {
        check(m).diagnostics.iter().map(|d| d.rule).collect()
    }

    #[test]
    fn base_is_ok() {
        assert_eq!(rules(&base()), Vec::<Rule>::new());
    }

    #[test]
    fn forms() {
        let mut m = base();
        m.things[1].id = "1b".into();
        m.links[0].target = "1b".into();
        m.things[2].name = " P".into();
        m.links[0].target_multiplicity = Some("many".into());
        m.format_version = "9".into();
        let r = rules(&m);
        for x in [Rule::F1, Rule::F2, Rule::F3, Rule::F4] {
            assert!(r.contains(&x), "{:?} in {:?}", x, r);
        }
    }

    #[test]
    fn c1_duplicate_and_missing() {
        let mut m = base();
        m.links
            .push(Link::new("a", LinkKind::Result, "p", "nothing"));
        let r = check(&m);
        assert_eq!(
            r.errors().filter(|d| d.rule == Rule::C1).count(),
            2,
            "{}",
            r
        );
    }

    #[test]
    fn c3_c4_c5_c6_on_built_links() {
        let mut m = base();
        let mut l = Link::new("l1", LinkKind::Result, "p", "b");
        l.modifier = Some(Modifier::Event);
        l.from_state = Some("a_s1".into());
        m.links.push(l);
        m.links.push(Link::new("l2", LinkKind::Effect, "p", "a_s1"));
        m.links.push(Link::new("l3", LinkKind::Tagged, "a", "b"));
        let r = rules(&m);
        for x in [Rule::C3, Rule::C4, Rule::C5, Rule::C6] {
            assert!(r.contains(&x), "{:?} in {:?}", x, r);
        }
    }

    #[test]
    fn c5_state_of_another_object() {
        let mut m = base();
        let mut l = Link::new("l1", LinkKind::Effect, "p", "b");
        l.to_state = Some("a_s2".into());
        m.links.push(l);
        assert_eq!(rules(&m), vec![Rule::C5]);
    }

    #[test]
    fn c8_is_a_warning() {
        let mut m = base();
        m.things.push(Thing::process("q", "Q"));
        let r = check(&m);
        assert!(r.is_ok());
        assert_eq!(r.warnings().count(), 1);
        assert_eq!(r.diagnostics[0].rule, Rule::C8);
        assert_eq!(r.diagnostics[0].id, "q");
    }

    #[test]
    fn c10_root() {
        let mut m = base();
        let mut d = Diagram::new("sd", "SD", Refinement::View);
        d.refinee = Some("p".into());
        m.diagram = Some(d);
        let r = check(&m);
        assert_eq!(
            r.errors().filter(|d| d.rule == Rule::C10).count(),
            2,
            "{}",
            r
        );
    }

    #[test]
    fn c12_self_loop() {
        let mut m = base();
        m.links
            .push(Link::new("g", LinkKind::Generalization, "a", "a"));
        assert_eq!(rules(&m), vec![Rule::C12]);
    }
}
