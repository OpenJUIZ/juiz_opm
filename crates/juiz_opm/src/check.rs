// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The juiz rules J1 to J13 (spec/metamodel.md §9.6, spec/conf.md §6).
//!
//! They look only at Things with a `juiz:role`; a model without roles has nothing to break.
//! J10 needs the type names of the plugins and is not checked from the model alone.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use opm_model::{Affiliation, Link, LinkKind, Model, Modifier, Thing, JUIZ_NS};

/// One broken juiz rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 1 to 13, or 0 for an annotation that juiz does not have or a value out of its set
    /// (spec/xml.md §8).
    pub rule: u8,
    /// The id of the element the rule is about.
    pub id: String,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.rule == 0 {
            write!(f, "error[juiz] {}: {}", self.id, self.message)
        } else {
            write!(f, "error[J{}] {}: {}", self.rule, self.id, self.message)
        }
    }
}

fn ann<'a>(ext: &'a opm_model::Extensions, key: &str) -> Option<&'a str> {
    ext.attribute(JUIZ_NS, key)
}

fn role(t: &Thing) -> &str {
    ann(&t.extensions, "role").unwrap_or("")
}

fn value_kind(t: &Thing) -> &str {
    ann(&t.extensions, "value_kind").unwrap_or("")
}

struct Ctx<'a> {
    m: &'a Model,
    things: HashMap<&'a str, &'a Thing>,
    /// state id → owner.
    states: HashMap<&'a str, &'a Thing>,
    out: Vec<Diagnostic>,
}

impl<'a> Ctx<'a> {
    fn thing(&self, id: &str) -> Option<&'a Thing> {
        self.things.get(id).copied()
    }

    /// The Thing at an end, the owner for a State.
    fn end(&self, id: &str) -> Option<&'a Thing> {
        self.thing(id).or_else(|| self.states.get(id).copied())
    }

    fn role_of(&self, id: &str) -> &'a str {
        self.end(id).map(role).unwrap_or("")
    }

    fn err(&mut self, rule: u8, id: &str, message: String) {
        self.out.push(Diagnostic {
            rule,
            id: id.to_string(),
            message,
        });
    }

    fn links(&self) -> impl Iterator<Item = &'a Link> + 'a {
        self.m.links.iter()
    }

    fn from(&self, id: &'a str, kind: LinkKind) -> impl Iterator<Item = &'a Link> + 'a {
        self.m
            .links
            .iter()
            .filter(move |l| l.kind == kind && l.source == id)
    }

    fn to(&self, id: &'a str, kind: LinkKind) -> impl Iterator<Item = &'a Link> + 'a {
        self.m
            .links
            .iter()
            .filter(move |l| l.kind == kind && l.target == id)
    }
}

const CALLABLE: &[&str] = &["process", "container_process"];
const ENTITIES: &[&str] = &[
    "container",
    "composite_container",
    "topic",
    "transform",
    "process",
    "container_process",
    "ec",
];

/// Check J1 to J13 (J10 excepted). The model should pass the core rules first.
pub fn check(m: &Model) -> Vec<Diagnostic> {
    let mut c = Ctx {
        m,
        things: m.things.iter().map(|t| (t.id.as_str(), t)).collect(),
        states: m
            .things
            .iter()
            .flat_map(|t| t.states().iter().map(move |s| (s.id.as_str(), t)))
            .collect(),
        out: Vec::new(),
    };
    annotations(&mut c);
    j1(&mut c);
    j2(&mut c);
    j3(&mut c);
    j4(&mut c);
    j5(&mut c);
    j6(&mut c);
    j7(&mut c);
    j8(&mut c);
    j9(&mut c);
    j11(&mut c);
    j12(&mut c);
    j13(&mut c);
    c.out
}

fn j1(c: &mut Ctx) {
    let m = c.m;
    for t in &m.things {
        let r = role(t);
        let ok: &[&str] = match r {
            "process" | "container_process" => &["output"],
            "node" => &["intermediate", "output"],
            _ => continue,
        };
        let n = c
            .from(&t.id, LinkKind::Result)
            .filter(|l| {
                c.thing(&l.target)
                    .is_some_and(|v| role(v) == "value" && ok.contains(&value_kind(v)))
            })
            .count();
        if n != 1 {
            c.err(
                1,
                &t.id,
                format!(
                    "{} {:?} yields {} values; it must yield exactly one",
                    r, t.name, n
                ),
            );
        }
    }
    for v in &m.things {
        if role(v) != "value" {
            continue;
        }
        let n = c.to(&v.id, LinkKind::Result).count();
        if n > 1 {
            c.err(
                1,
                &v.id,
                format!("value {:?} is yielded by {} processes", v.name, n),
            );
        }
    }
}

fn j2(c: &mut Ctx) {
    let m = c.m;
    // value(input) feeds exactly one instrument.
    for v in &m.things {
        if role(v) == "value" && value_kind(v) == "input" {
            let n = c.from(&v.id, LinkKind::Instrument).count();
            if n != 1 {
                c.err(
                    2,
                    &v.id,
                    format!("input value {:?} feeds {} instruments, not one", v.name, n),
                );
            }
        }
    }
    for t in &m.things {
        if !matches!(role(t), "process" | "container_process" | "node") {
            continue;
        }
        // arg → (indices, sources' kinds)
        let mut args: BTreeMap<&str, (HashSet<&str>, Vec<&str>)> = BTreeMap::new();
        for l in c.to(&t.id, LinkKind::Instrument) {
            let src_role = c.role_of(&l.source);
            if !matches!(src_role, "value" | "topic" | "pose") {
                continue;
            }
            let Some(arg) = ann(&l.extensions, "arg") else {
                c.err(
                    2,
                    &l.id,
                    format!("an argument of {:?} has no juiz:arg", t.name),
                );
                continue;
            };
            let Some(index) = ann(&l.extensions, "arg_index") else {
                c.err(
                    2,
                    &l.id,
                    format!("argument {} of {:?} has no juiz:arg_index", arg, t.name),
                );
                continue;
            };
            let kind = match c.end(&l.source) {
                Some(s) if src_role == "value" && value_kind(s) == "input" => "input",
                _ => "other",
            };
            let e = args.entry(arg).or_default();
            e.0.insert(index);
            e.1.push(kind);
        }
        let mut seen: HashMap<usize, &str> = HashMap::new();
        for (arg, (indices, kinds)) in &args {
            if indices.len() != 1 {
                c.err(
                    2,
                    &t.id,
                    format!(
                        "argument {} of {:?} has several juiz:arg_index",
                        arg, t.name
                    ),
                );
                continue;
            }
            if kinds.contains(&"input") && kinds.len() > 1 {
                c.err(
                    2,
                    &t.id,
                    format!(
                        "argument {} of {:?} has an input value and other sources",
                        arg, t.name
                    ),
                );
            }
            let ix = indices.iter().next().unwrap();
            match ix.parse::<usize>() {
                Ok(k) => {
                    if let Some(other) = seen.insert(k, arg) {
                        c.err(
                            2,
                            &t.id,
                            format!(
                                "arguments {} and {} of {:?} share juiz:arg_index {}",
                                other, arg, t.name, k
                            ),
                        );
                    }
                }
                Err(_) => c.err(
                    2,
                    &t.id,
                    format!("juiz:arg_index {:?} is not an integer", ix),
                ),
            }
        }
        let n = seen.len();
        if (0..n).any(|k| !seen.contains_key(&k)) {
            c.err(
                2,
                &t.id,
                format!(
                    "juiz:arg_index of {:?} does not run 0 to {} without a gap",
                    t.name,
                    n.saturating_sub(1)
                ),
            );
        }
    }
}

/// The process that yields a value(output).
fn producer<'a>(c: &Ctx<'a>, value: &'a str) -> Option<&'a Thing> {
    c.to(value, LinkKind::Result)
        .find_map(|l| c.thing(&l.source))
        .filter(|p| CALLABLE.contains(&role(p)))
}

fn j3(c: &mut Ctx) {
    for l in c.links().collect::<Vec<_>>() {
        if l.kind != LinkKind::Instrument {
            continue;
        }
        let has = ann(&l.extensions, "connection");
        let between = c
            .thing(&l.source)
            .is_some_and(|v| role(v) == "value" && value_kind(v) == "output")
            && producer(c, &l.source).is_some_and(|p| p.id != l.target)
            && CALLABLE.contains(&c.role_of(&l.target));
        match (has, between) {
            (Some(k), true) if k != "push" && k != "pull" => c.err(
                3,
                &l.id,
                format!("juiz:connection is {:?}, not push or pull", k),
            ),
            (Some(_), false) => c.err(
                3,
                &l.id,
                "juiz:connection is only on an instrument from another process's output"
                    .to_string(),
            ),
            (None, true) => c.err(
                3,
                &l.id,
                "an instrument from another process's output needs juiz:connection".to_string(),
            ),
            _ => {}
        }
    }
}

fn j4(c: &mut Ctx) {
    let mut push: HashSet<(String, String)> = HashSet::new();
    for l in c.links() {
        if l.kind == LinkKind::Instrument && ann(&l.extensions, "connection") == Some("push") {
            if let Some(p) = producer(c, &l.source) {
                push.insert((p.id.clone(), l.target.clone()));
            }
        }
    }
    let mut inv: HashMap<(String, String), Vec<&Link>> = HashMap::new();
    for l in c.links() {
        if l.kind == LinkKind::Invocation
            && CALLABLE.contains(&c.role_of(&l.source))
            && CALLABLE.contains(&c.role_of(&l.target))
        {
            inv.entry((l.source.clone(), l.target.clone()))
                .or_default()
                .push(l);
        }
    }
    for (pair, ls) in &inv {
        if !push.contains(pair) {
            c.err(
                4,
                &ls[0].id,
                "an invocation between processes needs a push connection".to_string(),
            );
        } else if ls.len() > 1 {
            c.err(
                4,
                &ls[1].id,
                "one invocation per pair of processes".to_string(),
            );
        }
    }
    let mut push: Vec<_> = push.into_iter().collect();
    push.sort();
    for pair in push {
        if !inv.contains_key(&pair) {
            c.err(
                4,
                &pair.1,
                format!("the push connection from {} needs an invocation", pair.0),
            );
        }
    }
}

fn j5(c: &mut Ctx) {
    let m = c.m;
    for t in &m.things {
        if role(t) != "container_process" {
            continue;
        }
        let n = m
            .links
            .iter()
            .filter(|l| {
                (l.source == t.id && l.kind == LinkKind::Effect
                    || l.target == t.id && l.kind == LinkKind::Instrument)
                    && {
                        let other = if l.source == t.id {
                            &l.target
                        } else {
                            &l.source
                        };
                        matches!(c.role_of(other), "container" | "composite_container")
                    }
            })
            .count();
        if n != 1 {
            c.err(
                5,
                &t.id,
                format!(
                    "container process {:?} has {} owners; it needs one",
                    t.name, n
                ),
            );
        }
    }
}

fn j6(c: &mut Ctx) {
    for l in c.links().collect::<Vec<_>>() {
        let (topic_end, other) = match l.kind {
            LinkKind::Result => (&l.target, &l.source),
            LinkKind::Instrument => (&l.source, &l.target),
            _ => continue,
        };
        if c.role_of(topic_end) != "topic" {
            continue;
        }
        if !CALLABLE.contains(&c.role_of(other)) {
            c.err(
                6,
                &l.id,
                "only a process or container process publishes or subscribes a topic".to_string(),
            );
        }
        if l.kind == LinkKind::Instrument && l.modifier != Some(Modifier::Event) {
            c.err(
                6,
                &l.id,
                "a subscription is an event instrument".to_string(),
            );
        }
    }
}

fn j7(c: &mut Ctx) {
    let m = c.m;
    let mut cycles_of_ec: HashMap<&str, Vec<&str>> = HashMap::new();
    for t in &m.things {
        if role(t) != "ec_cycle" {
            continue;
        }
        let conds: Vec<&Link> = c
            .to(&t.id, LinkKind::Instrument)
            .filter(|l| l.modifier == Some(Modifier::Condition))
            .filter(|l| c.states.contains_key(l.source.as_str()))
            .collect();
        let ok = conds.len() == 1
            && m.state(&conds[0].source)
                .is_some_and(|(o, s)| role(o) == "ec" && s.name == "started");
        if !ok {
            c.err(
                7,
                &t.id,
                format!(
                    "ec cycle {:?} needs exactly one condition from a started EC",
                    t.name
                ),
            );
            continue;
        }
        let (ec, _) = m.state(&conds[0].source).unwrap();
        cycles_of_ec.entry(&ec.id).or_default().push(&t.id);
        // Invocations: itself, or processes with juiz:order 0..n.
        let mut orders = Vec::new();
        let mut self_inv = false;
        for l in c.from(&t.id, LinkKind::Invocation) {
            if l.target == t.id {
                self_inv = true;
            } else if !CALLABLE.contains(&c.role_of(&l.target)) {
                c.err(7, &l.id, "an EC binds only processes".to_string());
            } else {
                match ann(&l.extensions, "order").and_then(|v| v.parse::<usize>().ok()) {
                    Some(k) => orders.push(k),
                    None => c.err(7, &l.id, "a bind needs juiz:order".to_string()),
                }
            }
        }
        orders.sort();
        if orders.iter().enumerate().any(|(i, k)| i != *k) {
            c.err(
                7,
                &t.id,
                "juiz:order of the binds does not run 0 to n without a gap".to_string(),
            );
        }
        let stop = c.from(&t.id, LinkKind::Effect).any(|l| {
            l.target == ec.id
                && l.from_state
                    .as_deref()
                    .and_then(|s| m.state(s))
                    .map(|x| x.1.name.as_str())
                    == Some("started")
                && l.to_state
                    .as_deref()
                    .and_then(|s| m.state(s))
                    .map(|x| x.1.name.as_str())
                    == Some("stopped")
        });
        match ann(&ec.extensions, "type") {
            Some("main_loop_ec") if !self_inv => c.err(
                7,
                &t.id,
                "the cycle of a main_loop_ec invokes itself".to_string(),
            ),
            Some("one_shot_ec") if self_inv || !stop => c.err(
                7,
                &t.id,
                "the cycle of a one_shot_ec does not invoke itself and changes the EC from started to stopped"
                    .to_string(),
            ),
            _ => {}
        }
    }
    for t in &m.things {
        if role(t) != "ec" {
            continue;
        }
        let mut names: Vec<&str> = t.states().iter().map(|s| s.name.as_str()).collect();
        names.sort();
        if names != ["started", "stopped"] {
            c.err(
                7,
                &t.id,
                format!("EC {:?} has exactly the states stopped and started", t.name),
            );
        }
        if t.states().iter().filter(|s| s.initial).count() != 1 {
            c.err(
                7,
                &t.id,
                format!("EC {:?} has exactly one initial state", t.name),
            );
        }
        let n = cycles_of_ec.get(t.id.as_str()).map_or(0, Vec::len);
        if n != 1 {
            c.err(
                7,
                &t.id,
                format!("EC {:?} has {} cycles; it needs one", t.name, n),
            );
        }
    }
}

fn j8(c: &mut Ctx) {
    let m = c.m;
    for t in &m.things {
        if !ENTITIES.contains(&role(t)) {
            continue;
        }
        // A container of another System (named by its ID) is environmental and not held.
        if t.affiliation == Affiliation::Environmental && role(t) == "container" {
            continue;
        }
        let n = m
            .links
            .iter()
            .filter(|l| {
                l.target == t.id
                    && matches!(l.kind, LinkKind::Aggregation | LinkKind::Exhibition)
                    && c.role_of(&l.source) == "system"
            })
            .count();
        if n != 1 {
            c.err(
                8,
                &t.id,
                format!("{:?} is held by {} systems; it needs one", t.name, n),
            );
        }
    }
}

fn j9(c: &mut Ctx) {
    let m = c.m;
    for z in &m.in_zooms {
        let Some(t) = c.thing(&z.refinee) else {
            continue;
        };
        if role(t) != "composite" {
            continue;
        }
        if z.steps.iter().any(|s| s.len() != 1) {
            c.err(9, &z.id, "a composite runs one node per step".to_string());
            continue;
        }
        let steps: Vec<&str> = z.steps.iter().map(|s| s[0].as_str()).collect();
        let root = steps.iter().copied().find(|n| {
            c.from(n, LinkKind::Result).any(|l| {
                c.thing(&l.target)
                    .is_some_and(|v| value_kind(v) == "output")
            })
        });
        let Some(root) = root else {
            c.err(
                9,
                &z.id,
                "no node yields the composite's output".to_string(),
            );
            continue;
        };
        let mut order = Vec::new();
        post_order(c, root, &mut order, 0);
        if order != steps {
            c.err(
                9,
                &z.id,
                format!(
                    "the steps are not the depth-first order of the tree by juiz:arg_index ({})",
                    order
                        .iter()
                        .map(|n| c.thing(n).map_or(*n, |t| t.name.as_str()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
    }
}

fn post_order<'a>(c: &Ctx<'a>, node: &'a str, out: &mut Vec<&'a str>, depth: usize) {
    if depth > 64 || out.contains(&node) {
        return;
    }
    let mut ins: Vec<&Link> = c.to(node, LinkKind::Instrument).collect();
    ins.sort_by_key(|l| ann(&l.extensions, "arg_index").and_then(|v| v.parse::<usize>().ok()));
    for l in ins {
        if c.thing(&l.source).map(value_kind) == Some("intermediate") {
            if let Some(child) = c.to(&l.source, LinkKind::Result).next() {
                post_order(c, &child.source, out, depth + 1);
            }
        }
    }
    out.push(node);
}

fn j11(c: &mut Ctx) {
    let m = c.m;
    let mut parent: HashMap<&str, &str> = HashMap::new();
    for t in &m.things {
        if role(t) != "transform" {
            continue;
        }
        let mut ends: HashMap<&str, Vec<&str>> = HashMap::new();
        for l in c.from(&t.id, LinkKind::Tagged) {
            if let Some(e) = ann(&l.extensions, "end") {
                ends.entry(e).or_default().push(&l.target);
            }
        }
        let (Some([s]), Some([d])) = (
            ends.get("source").map(Vec::as_slice),
            ends.get("destination").map(Vec::as_slice),
        ) else {
            c.err(
                11,
                &t.id,
                format!(
                    "transform {:?} needs exactly one source and one destination",
                    t.name
                ),
            );
            continue;
        };
        let (s, d) = (*s, *d);
        if c.role_of(s) != "container" || c.role_of(d) != "container" {
            c.err(
                11,
                &t.id,
                format!("both ends of transform {:?} are containers", t.name),
            );
            continue;
        }
        if s == d {
            c.err(11, &t.id, "a transform joins two containers".to_string());
        }
        if parent.insert(d, s).is_some() {
            c.err(
                11,
                &t.id,
                "a container is the destination of one transform at most".to_string(),
            );
        }
    }
    // No cycle.
    for start in parent.keys().copied().collect::<Vec<_>>() {
        let mut seen = HashSet::new();
        let mut x = start;
        while let Some(p) = parent.get(x).copied() {
            if !seen.insert(x) {
                c.err(11, start, "the transforms form a cycle".to_string());
                break;
            }
            x = p;
        }
    }
}

fn j12(c: &mut Ctx) {
    let m = c.m;
    for t in &m.things {
        if role(t) != "pose" {
            continue;
        }
        let n = c
            .to(&t.id, LinkKind::Exhibition)
            .filter(|l| c.role_of(&l.source) == "container")
            .count();
        if n != 1 {
            c.err(
                12,
                &t.id,
                format!(
                    "pose {:?} is exhibited by {} containers; it needs one",
                    t.name, n
                ),
            );
        }
        let listened = c
            .from(&t.id, LinkKind::Instrument)
            .any(|l| l.modifier == Some(Modifier::Event));
        if !listened {
            c.err(
                12,
                &t.id,
                format!("pose {:?} appears only with a pose listener", t.name),
            );
        }
    }
}

fn j13(c: &mut Ctx) {
    let m = c.m;
    for l in &m.links {
        if !matches!(
            l.kind,
            LinkKind::Consumption
                | LinkKind::Agent
                | LinkKind::OvertimeException
                | LinkKind::UndertimeException
        ) {
            continue;
        }
        if !c.role_of(&l.source).is_empty() || !c.role_of(&l.target).is_empty() {
            c.err(
                13,
                &l.id,
                format!("a juiz element has no {} link", l.kind.name()),
            );
        }
    }
    for f in &m.fans {
        let touches = f.links.iter().any(|id| {
            m.link(id).is_some_and(|l| {
                !c.role_of(&l.source).is_empty() || !c.role_of(&l.target).is_empty()
            })
        });
        if touches {
            c.err(13, &f.id, "a juiz element is in no fan".to_string());
        }
    }
}

const ROLES: &[&str] = &[
    "system",
    "container",
    "composite_container",
    "topic",
    "value",
    "transform",
    "pose",
    "slot",
    "process",
    "container_process",
    "composite",
    "node",
    "ec",
    "ec_cycle",
    "client",
];

/// The juiz annotations an element may carry (metamodel §9.3, §9.4) and the values of the
/// enumerated ones.
fn annotations(c: &mut Ctx) {
    use opm_opl::juiz::{keys, Target};
    let m = c.m;
    let mut found = Vec::new();
    let mut look = |ext: &opm_model::Extensions, id: &str, target: Target| {
        let known = keys(target);
        let names = ext
            .attributes
            .iter()
            .filter(|a| a.namespace == JUIZ_NS)
            .map(|a| a.name.as_str())
            .chain(
                ext.elements
                    .iter()
                    .filter(|e| e.namespace == JUIZ_NS)
                    .map(|e| e.name.as_str()),
            );
        for n in names {
            if !known.iter().any(|(k, _)| *k == n) {
                found.push((
                    id.to_string(),
                    format!("juiz:{} is not a juiz annotation here", n),
                ));
            }
        }
        let allowed: &[(&str, &[&str])] = &[
            ("role", ROLES),
            ("value_kind", &["output", "input", "intermediate", "const"]),
            ("connection", &["push", "pull"]),
            ("end", &["source", "destination"]),
            (
                "status",
                &["released", "merged", "in_progress", "planned", "defect"],
            ),
        ];
        if let Some(v) = ext.attribute(JUIZ_NS, "datatype") {
            if !is_datatype(v) {
                found.push((
                    id.to_string(),
                    format!("juiz:datatype {:?} is not <full name>[@sha256:<64 hex>]", v),
                ));
            }
        }
        for (key, values) in allowed {
            if let Some(v) = ext.attribute(JUIZ_NS, key) {
                if !values.contains(&v) {
                    found.push((
                        id.to_string(),
                        format!("juiz:{} {:?} is not one of {:?}", key, v, values),
                    ));
                }
            }
        }
    };
    for t in &m.things {
        look(&t.extensions, &t.id, Target::Thing);
        for st in t.states() {
            look(&st.extensions, &st.id, Target::State);
        }
    }
    for l in &m.links {
        look(&l.extensions, &l.id, Target::Link);
    }
    for (id, message) in found {
        c.err(0, &id, message);
    }
}

/// juiz datatype.md §5: a full name (`juiz_std.geometry.Pose`), optionally `@sha256:` and 64
/// lower-case hex digits.
fn is_datatype(v: &str) -> bool {
    let (name, hash) = match v.split_once('@') {
        Some((n, h)) => (n, Some(h)),
        None => (v, None),
    };
    let name_ok = !name.is_empty()
        && name.split('.').all(|part| {
            part.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        });
    let hash_ok = hash.is_none_or(|h| {
        h.strip_prefix("sha256:").is_some_and(|x| {
            x.len() == 64
                && x.chars()
                    .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        })
    });
    name_ok && hash_ok
}
