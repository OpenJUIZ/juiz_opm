// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! From the syntax tree to the model: names are resolved, the rules O1 to O12 are checked
//! (spec/opl.md §8) and missing ids are made (§10.4).

use std::collections::{HashMap, HashSet};

use opm_model::{
    Affiliation, Duration, Essence, Fan, InZoom, Link, LinkKind, Model, State, Thing, ThingKind,
};

use crate::diag::{Code, Diagnostic, Pos};
use crate::juiz::{self, Target};
use crate::lex::{Block, Value};
use crate::names;
use crate::parse::{Document, End, LinkSpec, Name, Sentence, Statement};
use crate::write;

/// The model read from a document, with where each element was written.
#[derive(Debug, Clone)]
pub struct Built {
    pub model: Model,
    /// Element id → the sentence that wrote it.
    pub positions: HashMap<String, Pos>,
    /// Some element had `{id: ...}`.
    pub explicit_ids: bool,
}

const TEMP: char = '\u{1}';

pub fn build(doc: &Document) -> (Built, Vec<Diagnostic>) {
    let mut b = Builder {
        diags: Vec::new(),
        used: HashSet::new(),
        positions: HashMap::new(),
        temp: 0,
    };
    let built = b.run(doc);
    (built, b.diags)
}

struct Builder {
    diags: Vec<Diagnostic>,
    used: HashSet<String>,
    positions: HashMap<String, Pos>,
    temp: usize,
}

/// Core attributes read from `{...}`.
#[derive(Default)]
struct Core {
    id: Option<String>,
    description: Option<String>,
    ordered: Option<bool>,
    duration: Option<Duration>,
    link_ids: Option<(Vec<String>, Pos)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CoreKind {
    Header,
    Decl { process: bool },
    State,
    Link { aggregation: bool },
    Zoom,
    Fan,
}

impl Builder {
    fn o(&mut self, rule: u8, pos: Pos, msg: impl Into<String>) {
        self.diags
            .push(Diagnostic::error(Code::O(rule), Some(pos), msg));
    }

    fn temp_id(&mut self) -> String {
        self.temp += 1;
        format!("{}{}", TEMP, self.temp)
    }

    /// `base`, or `base<sep>2`, `base<sep>3`, ... whichever is free first.
    fn fresh(&mut self, base: &str, sep: &str) -> String {
        if self.used.insert(base.to_string()) {
            return base.to_string();
        }
        (2..)
            .map(|n| format!("{}{}{}", base, sep, n))
            .find(|c| self.used.insert(c.clone()))
            .expect("a free id")
    }

    // ---- blocks

    fn core(&mut self, block: Option<&Block>, kind: CoreKind, stmt: Pos) -> Core {
        let mut c = Core::default();
        let Some(block) = block else {
            return c;
        };
        let mut dmin = None;
        let mut dexp = None;
        let mut dmax = None;
        let mut unit: Option<(String, Pos)> = None;
        for (k, e) in block.entries.iter().enumerate() {
            if block.entries[..k].iter().any(|f| f.key == e.key) {
                self.o(7, e.pos, format!("`{}` is written twice", e.key));
                continue;
            }
            let allowed = match e.key.as_str() {
                "id" | "description" => true,
                "ordered" => kind == CoreKind::Link { aggregation: true },
                "duration_min" | "duration_expected" | "duration_max" | "duration_unit" => {
                    kind == CoreKind::Decl { process: true }
                }
                "link_ids" => kind == CoreKind::Fan,
                _ => false,
            };
            if !allowed {
                self.o(
                    7,
                    e.pos,
                    format!("`{}` cannot be written on this sentence", e.key),
                );
                continue;
            }
            let atoms: Vec<&str> = match &e.value {
                Value::Atoms(a) => a.iter().map(|a| a.text.as_str()).collect(),
                Value::Json(_) => unreachable!("no JSON in {{...}}"),
            };
            let one = match atoms.as_slice() {
                [a] => Some(*a),
                _ => None,
            };
            match e.key.as_str() {
                "id" => match one {
                    Some(id) => c.id = Some(id.to_string()),
                    None => self.o(7, e.pos, "`id` takes one id"),
                },
                "description" => match one {
                    Some(d) => c.description = Some(d.to_string()),
                    None => self.o(7, e.pos, "`description` takes one quoted string"),
                },
                "ordered" => match one {
                    Some("true") => c.ordered = Some(true),
                    Some("false") => c.ordered = Some(false),
                    _ => self.o(7, e.pos, "`ordered` takes `true` or `false`"),
                },
                "duration_unit" => match one {
                    Some(u) => unit = Some((u.to_string(), e.pos)),
                    None => self.o(7, e.pos, "`duration_unit` takes one word"),
                },
                "link_ids" => {
                    c.link_ids = Some((atoms.iter().map(|a| a.to_string()).collect(), e.pos))
                }
                key => {
                    let x = one.and_then(names::parse_number);
                    match x {
                        Some(x) => match key {
                            "duration_min" => dmin = Some(x),
                            "duration_expected" => dexp = Some(x),
                            _ => dmax = Some(x),
                        },
                        None => self.o(7, e.pos, format!("`{}` takes a number", key)),
                    }
                }
            }
        }
        let any = dmin.is_some() || dexp.is_some() || dmax.is_some();
        match (any, unit) {
            (true, Some((unit, _))) => {
                c.duration = Some(Duration {
                    min: dmin,
                    expected: dexp,
                    max: dmax,
                    unit,
                })
            }
            (true, None) => self.o(7, stmt, "a duration needs `duration_unit`"),
            (false, Some((_, p))) => self.o(7, p, "`duration_unit` without a duration"),
            (false, None) => {}
        }
        c
    }

    fn juiz(&mut self, block: Option<&Block>, target: Option<Target>) -> opm_model::Extensions {
        let Some(block) = block else {
            return Default::default();
        };
        let Some(target) = target else {
            let pos = block.entries.first().map(|e| e.pos).unwrap_or_default();
            self.o(7, pos, "`[...]` cannot be written on this sentence");
            return Default::default();
        };
        let (ext, errors) = juiz::to_extensions(target, &block.entries);
        for (k, m) in errors {
            self.o(7, block.entries[k].pos, m);
        }
        ext
    }

    // ---- the whole document

    fn run(&mut self, doc: &Document) -> Built {
        // O8: explicit ids first, so that made ids keep clear of them.
        let mut explicit_ids = false;
        let blocks = doc
            .header
            .iter()
            .filter_map(|h| h.core.as_ref())
            .chain(doc.statements.iter().filter_map(|s| s.core.as_ref()));
        let mut explicit: Vec<(String, Pos)> = Vec::new();
        for b in blocks {
            for e in &b.entries {
                if e.key == "id" {
                    if let Value::Atoms(a) = &e.value {
                        if let [a] = a.as_slice() {
                            explicit.push((a.text.clone(), e.pos));
                        }
                    }
                }
            }
        }
        for (id, pos) in explicit {
            explicit_ids = true;
            if !names::is_id(&id) {
                self.o(
                    8,
                    pos,
                    format!("{:?} is not an id ([A-Za-z_][A-Za-z0-9_.-]*)", id),
                );
            } else if !self.used.insert(id.clone()) {
                self.o(8, pos, format!("the id {:?} is used twice", id));
            }
        }

        let mut model = Model::new("");
        if let Some(h) = &doc.header {
            let c = self.core(h.core.as_ref(), CoreKind::Header, h.pos);
            model.name = Some(h.name.clone());
            model.description = c.description;
            if let Some(id) = c.id {
                model.id = id;
            }
        }
        if model.id.is_empty() {
            model.id = self.fresh("opm-model", "-");
        }

        let things = self.things(doc, &mut model);
        self.states(doc, &mut model, &things);
        self.zooms(doc, &mut model, &things);
        let link_pos = self.links(doc, &mut model, &things);
        self.name_links(&mut model, link_pos);

        Built {
            model,
            positions: std::mem::take(&mut self.positions),
            explicit_ids,
        }
    }

    /// Declarations (O1, O11) and the Things, in name order.
    fn things(&mut self, doc: &Document, model: &mut Model) -> HashMap<String, usize> {
        let mut decls: Vec<(&Statement, &Name, bool, &Vec<Name>)> = Vec::new();
        for s in &doc.statements {
            if let Sentence::Decl {
                thing,
                process,
                qualifiers,
            } = &s.sentence
            {
                if let Some(prev) = decls.iter().find(|d| d.1.text == thing.text) {
                    let line = prev.1.pos.line;
                    self.o(
                        1,
                        thing.pos,
                        format!(
                            "{:?} is declared twice (first on line {})",
                            thing.text, line
                        ),
                    );
                    continue;
                }
                decls.push((s, thing, *process, qualifiers));
            }
        }
        decls.sort_by(|a, b| a.1.text.cmp(&b.1.text));
        let mut index = HashMap::new();
        for (s, name, process, qualifiers) in decls {
            let c = self.core(s.core.as_ref(), CoreKind::Decl { process }, s.pos);
            let id = match c.id {
                Some(id) => id,
                None => {
                    let base = names::id_from_name(&name.text, "t");
                    self.fresh(&base, "_")
                }
            };
            let mut t = if process {
                Thing::process(id.clone(), name.text.clone())
            } else {
                Thing::object(id.clone(), name.text.clone())
            };
            for q in qualifiers {
                match q.text.as_str() {
                    "physical" => t.essence = Essence::Physical,
                    "informatical" => t.essence = Essence::Informatical,
                    "environmental" => t.affiliation = Affiliation::Environmental,
                    _ => t.affiliation = Affiliation::Systemic,
                }
            }
            t.description = c.description;
            if let ThingKind::Process { duration } = &mut t.kind {
                *duration = c.duration;
            }
            t.extensions = self.juiz(s.juiz.as_ref(), Some(Target::Thing));
            self.positions.insert(id, s.pos);
            index.insert(name.text.clone(), model.things.len());
            model.things.push(t);
        }
        index
    }

    fn thing_ref(&mut self, things: &HashMap<String, usize>, n: &Name) -> Option<usize> {
        let r = things.get(&n.text).copied();
        if r.is_none() {
            self.o(1, n.pos, format!("{:?} has no declaration", n.text));
        }
        r
    }

    fn object_ref(
        &mut self,
        model: &Model,
        things: &HashMap<String, usize>,
        n: &Name,
        why: &str,
    ) -> Option<usize> {
        let k = self.thing_ref(things, n)?;
        if !model.things[k].is_object() {
            self.o(
                4,
                n.pos,
                format!("{:?} is a process, but {} is an object", n.text, why),
            );
            return None;
        }
        Some(k)
    }

    fn process_ref(
        &mut self,
        model: &Model,
        things: &HashMap<String, usize>,
        n: &Name,
        why: &str,
    ) -> Option<usize> {
        let k = self.thing_ref(things, n)?;
        if !model.things[k].is_process() {
            self.o(
                4,
                n.pos,
                format!("{:?} is an object, but {} is a process", n.text, why),
            );
            return None;
        }
        Some(k)
    }

    /// The id of State `s` of the Object at `k` (O2).
    fn state_ref(&mut self, model: &Model, k: usize, s: &Name) -> Option<String> {
        let t = &model.things[k];
        let r = t
            .states()
            .iter()
            .find(|x| x.name == s.text)
            .map(|x| x.id.clone());
        if r.is_none() {
            self.o(
                2,
                s.pos,
                format!("{:?} is not a State of {:?} (`can be`)", s.text, t.name),
            );
        }
        r
    }

    /// `can be`, `is initially`, State sentences (O2, O3).
    fn states(&mut self, doc: &Document, model: &mut Model, things: &HashMap<String, usize>) {
        // State sentences, by (thing, state), for their ids.
        let mut lists: Vec<Option<(&Statement, &Vec<Name>)>> = vec![None; model.things.len()];
        for s in &doc.statements {
            if let Sentence::StateList { thing, states } = &s.sentence {
                self.no_blocks(s);
                let Some(k) = self.object_ref(model, things, thing, "the owner of States") else {
                    continue;
                };
                if lists[k].is_some() {
                    self.o(2, s.pos, format!("{:?} has a second `can be`", thing.text));
                    continue;
                }
                for (i, n) in states.iter().enumerate() {
                    if states[..i].iter().any(|m| m.text == n.text) {
                        self.o(2, n.pos, format!("the State {:?} is written twice", n.text));
                    }
                }
                lists[k] = Some((s, states));
            }
        }
        let mut sentences: HashMap<(usize, String), &Statement> = HashMap::new();
        for s in &doc.statements {
            if let Sentence::StateSentence { state, thing, .. } = &s.sentence {
                let Some(&k) = things.get(&thing.text) else {
                    continue;
                };
                if sentences.insert((k, state.text.clone()), s).is_some() {
                    self.o(
                        3,
                        s.pos,
                        format!(
                            "State {:?} of {:?} has a second sentence",
                            state.text, thing.text
                        ),
                    );
                }
            }
        }
        // States and their ids, in Thing order then `can be` order.
        for (k, list) in lists.iter().enumerate() {
            let Some((ls, names_)) = list else {
                continue;
            };
            let owner = model.things[k].id.clone();
            let mut states = Vec::new();
            for n in names_.iter() {
                if states.iter().any(|x: &State| x.name == n.text) {
                    continue;
                }
                let sentence = sentences.get(&(k, n.text.clone())).copied();
                let c = match sentence {
                    Some(st) => self.core(st.core.as_ref(), CoreKind::State, st.pos),
                    None => Core::default(),
                };
                let id = match c.id {
                    Some(id) => id,
                    None => {
                        let base = format!("{}.{}", owner, names::id_from_name(&n.text, "s"));
                        self.fresh(&base, "_")
                    }
                };
                let mut st = State::new(id.clone(), n.text.clone());
                st.description = c.description;
                if let Some(sen) = sentence {
                    st.extensions = self.juiz(sen.juiz.as_ref(), Some(Target::State));
                }
                self.positions
                    .insert(id, sentence.map(|x| x.pos).unwrap_or(ls.pos));
                states.push(st);
            }
            if let ThingKind::Object { states: s } = &mut model.things[k].kind {
                *s = states;
            }
        }
        // State sentences that name an unknown State or Thing.
        for s in &doc.statements {
            if let Sentence::StateSentence {
                state,
                thing,
                flags,
            } = &s.sentence
            {
                let Some(k) = self.object_ref(model, things, thing, "the owner of a State") else {
                    continue;
                };
                let Some(id) = self.state_ref(model, k, state) else {
                    continue;
                };
                let st = state_mut(model, k, &id);
                for f in flags {
                    match f.text.as_str() {
                        "initial" => st.initial = true,
                        "final" => st.final_ = true,
                        _ => st.default = true,
                    }
                }
            }
        }
        // `is initially ... and finally ...`
        let mut ends_seen: HashSet<usize> = HashSet::new();
        for s in &doc.statements {
            if let Sentence::StateEnds {
                thing,
                initial,
                final_,
            } = &s.sentence
            {
                self.no_blocks(s);
                let Some(k) = self.object_ref(model, things, thing, "the owner of States") else {
                    continue;
                };
                if !ends_seen.insert(k) {
                    self.o(
                        3,
                        s.pos,
                        format!(
                            "{:?} has a second `is initially` / `is finally`",
                            thing.text
                        ),
                    );
                    continue;
                }
                for (names_, initial_flag) in [(initial, true), (final_, false)] {
                    for n in names_ {
                        let Some(id) = self.state_ref(model, k, n) else {
                            continue;
                        };
                        let st = state_mut(model, k, &id);
                        let flag = if initial_flag {
                            &mut st.initial
                        } else {
                            &mut st.final_
                        };
                        if *flag {
                            self.o(3, n.pos, format!("{:?} is flagged twice", n.text));
                        }
                        *flag = true;
                    }
                }
            }
        }
    }

    fn no_blocks(&mut self, s: &Statement) {
        if s.core.is_some() || s.juiz.is_some() {
            let what = if matches!(s.sentence, Sentence::Links(_)) {
                (6, "a list sentence takes no `{...}` or `[...]`")
            } else {
                (7, "this sentence takes no `{...}` or `[...]`")
            };
            self.o(what.0, s.pos, what.1);
        }
    }

    fn zooms(&mut self, doc: &Document, model: &mut Model, things: &HashMap<String, usize>) {
        let mut zooms = Vec::new();
        for s in &doc.statements {
            let Sentence::Zoom { thing, body } = &s.sentence else {
                continue;
            };
            let c = self.core(s.core.as_ref(), CoreKind::Zoom, s.pos);
            let ext = self.juiz(s.juiz.as_ref(), None);
            let Some(k) = self.thing_ref(things, thing) else {
                continue;
            };
            let refinee = model.things[k].id.clone();
            let mut z = InZoom {
                id: String::new(),
                refinee: refinee.clone(),
                steps: Vec::new(),
                parts: Vec::new(),
                description: c.description,
                extensions: ext,
            };
            if model.things[k].is_process() {
                if !body.sequenced && !body.parallel && body.steps.len() > 1 {
                    self.o(
                        4,
                        s.pos,
                        "the steps of a Process need `in that sequence` or `parallel`",
                    );
                }
                for step in &body.steps {
                    let ids: Vec<String> = step
                        .iter()
                        .filter_map(|n| self.process_ref(model, things, n, "a step"))
                        .map(|k| model.things[k].id.clone())
                        .collect();
                    z.steps.push(ids);
                }
                for n in &body.parts {
                    if let Some(k) = self.object_ref(model, things, n, "a part of a Process") {
                        z.parts.push(model.things[k].id.clone());
                    }
                }
            } else {
                if body.nothing || body.sequenced || body.parallel || !body.parts.is_empty() {
                    self.o(
                        4,
                        s.pos,
                        "an Object zooms into one list of parts, without `nothing`, `parallel`, `in that sequence` or `as well as`",
                    );
                }
                for n in body.steps.iter().flatten() {
                    if let Some(k) = self.thing_ref(things, n) {
                        z.parts.push(model.things[k].id.clone());
                    }
                }
            }
            zooms.push((c.id, z, s.pos));
        }
        for (id, mut z, pos) in zooms {
            z.id = match id {
                Some(id) => id,
                None => {
                    let base = format!("in_zoom-{}", z.refinee);
                    self.fresh(&base, "-")
                }
            };
            self.positions.insert(z.id.clone(), pos);
            model.in_zooms.push(z);
        }
    }

    /// The id of an end: the Thing, or its State.
    fn end_ref(
        &mut self,
        model: &Model,
        things: &HashMap<String, usize>,
        e: &End,
        process: bool,
        why: &str,
    ) -> Option<String> {
        let k = if process {
            self.process_ref(model, things, &e.thing, why)?
        } else {
            self.object_ref(model, things, &e.thing, why)?
        };
        match &e.state {
            Some(s) => self.state_ref(model, k, s),
            None => Some(model.things[k].id.clone()),
        }
    }

    fn link(
        &mut self,
        model: &Model,
        things: &HashMap<String, usize>,
        l: &LinkSpec,
    ) -> Option<Link> {
        use LinkKind::*;
        let (sp, tp, why) = match l.kind {
            Consumption | Instrument | Agent => (Some(false), Some(true), l.kind.name()),
            Result | Effect => (Some(true), Some(false), l.kind.name()),
            Invocation | OvertimeException | UndertimeException => {
                (Some(true), Some(true), l.kind.name())
            }
            _ => (None, None, ""),
        };
        let end = |b: &mut Self, e: &End, p: Option<bool>, side: &str| -> Option<String> {
            match p {
                Some(p) => b.end_ref(model, things, e, p, &format!("the {} of {}", side, why)),
                None => {
                    let k = b.thing_ref(things, &e.thing)?;
                    Some(model.things[k].id.clone())
                }
            }
        };
        let source = end(self, &l.source, sp, "source");
        let target = end(self, &l.target, tp, "target");
        let (source, target) = (source?, target?);
        let mut link = Link::new(String::new(), l.kind, source, target.clone());
        link.modifier = l.modifier;
        link.source_multiplicity = l.source.mult.clone();
        link.target_multiplicity = l.target.mult.clone();
        link.tag = l.tag.clone();
        link.backward_tag = l.backward_tag.clone();
        if l.from_state.is_some() || l.to_state.is_some() {
            let k = things[&l.target.thing.text];
            link.from_state = l
                .from_state
                .as_ref()
                .and_then(|s| self.state_ref(model, k, s));
            link.to_state = l
                .to_state
                .as_ref()
                .and_then(|s| self.state_ref(model, k, s));
        }
        if let Some((n, unit)) = &l.exception {
            // O12: the number and unit are those of the duration of P1.
            let p1 = &model.things[things[&l.source.thing.text]];
            let d = match &p1.kind {
                ThingKind::Process { duration: Some(d) } => Some(d),
                _ => None,
            };
            let want = d.and_then(|d| {
                let x = if l.kind == OvertimeException {
                    d.max
                } else {
                    d.min
                };
                x.map(|x| (x, d.unit.clone()))
            });
            match want {
                Some((x, u)) if x == *n && u == *unit => {}
                Some((x, u)) => self.o(
                    12,
                    l.pos,
                    format!(
                        "the duration of {:?} is {} {}, not {} {}",
                        p1.name,
                        names::number(x),
                        u,
                        names::number(*n),
                        unit
                    ),
                ),
                None => self.o(
                    12,
                    l.pos,
                    format!(
                        "{:?} has no {} duration (`{{{}: ...}}`)",
                        p1.name,
                        if l.kind == OvertimeException {
                            "maximum"
                        } else {
                            "minimum"
                        },
                        if l.kind == OvertimeException {
                            "duration_max"
                        } else {
                            "duration_min"
                        },
                    ),
                ),
            }
        }
        Some(link)
    }

    /// Links and Fans, with temporary ids. Returns the position of each link by index.
    fn links(
        &mut self,
        doc: &Document,
        model: &mut Model,
        things: &HashMap<String, usize>,
    ) -> Vec<(Option<String>, Pos)> {
        let mut out = Vec::new();
        for s in &doc.statements {
            match &s.sentence {
                Sentence::Link(l) => {
                    let c = self.core(
                        s.core.as_ref(),
                        CoreKind::Link {
                            aggregation: l.kind == LinkKind::Aggregation,
                        },
                        s.pos,
                    );
                    let ext = self.juiz(s.juiz.as_ref(), Some(Target::Link));
                    if let Some(mut link) = self.link(model, things, l) {
                        link.id = self.temp_id();
                        link.description = c.description;
                        link.ordered = c.ordered.unwrap_or(false);
                        link.path = s.path.as_ref().map(|p| p.text.clone());
                        link.extensions = ext;
                        model.links.push(link);
                        out.push((c.id, s.pos));
                    }
                }
                Sentence::Links(ls) => {
                    self.no_blocks(s);
                    for l in ls {
                        if let Some(mut link) = self.link(model, things, l) {
                            link.id = self.temp_id();
                            link.path = s.path.as_ref().map(|p| p.text.clone());
                            model.links.push(link);
                            out.push((None, s.pos));
                        }
                    }
                }
                Sentence::Fan { operator, links } => {
                    let c = self.core(s.core.as_ref(), CoreKind::Fan, s.pos);
                    let _ = self.juiz(s.juiz.as_ref(), None);
                    if s.path.is_some() {
                        self.o(11, s.pos, "the links of a Fan take no path label");
                    }
                    for l in links {
                        if l.source.mult.is_some() || l.target.mult.is_some() {
                            self.o(10, l.pos, "the links of a Fan take no multiplicity");
                        }
                    }
                    let ids = match &c.link_ids {
                        Some((ids, pos)) if ids.len() != links.len() => {
                            self.o(
                                9,
                                *pos,
                                format!(
                                    "`link_ids` has {} ids for {} links",
                                    ids.len(),
                                    links.len()
                                ),
                            );
                            None
                        }
                        Some((ids, pos)) => {
                            for id in ids {
                                if !names::is_id(id) {
                                    self.o(8, *pos, format!("{:?} is not an id", id));
                                } else if !self.used.insert(id.clone()) {
                                    self.o(8, *pos, format!("the id {:?} is used twice", id));
                                }
                            }
                            Some(ids.clone())
                        }
                        None => None,
                    };
                    let mut members = Vec::new();
                    for (k, l) in links.iter().enumerate() {
                        if let Some(mut link) = self.link(model, things, l) {
                            link.id = self.temp_id();
                            members.push(link.id.clone());
                            model.links.push(link);
                            out.push((ids.as_ref().map(|v| v[k].clone()), s.pos));
                        }
                    }
                    let fid = self.temp_id();
                    self.positions.insert(fid.clone(), s.pos);
                    model.fans.push(Fan {
                        id: fid,
                        operator: *operator,
                        links: members,
                        description: c.description,
                        extensions: Default::default(),
                    });
                    // The explicit Fan id waits for name_links.
                    if let Some(id) = c.id {
                        let f = model.fans.last_mut().unwrap();
                        f.id = format!("{}{}", TEMP, id);
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// Give links and Fans their ids, in the order of the canonical sentences (§10.4).
    fn name_links(&mut self, model: &mut Model, link_pos: Vec<(Option<String>, Pos)>) {
        let order = write::element_order(model);
        let mut rename: HashMap<String, String> = HashMap::new();
        let index: HashMap<String, usize> = model
            .links
            .iter()
            .enumerate()
            .map(|(k, l)| (l.id.clone(), k))
            .collect();
        let end_id = |id: &str| id.to_string();
        for e in order {
            match e {
                write::Elem::Link(k) => {
                    let l = &model.links[k];
                    let at = index[&l.id];
                    let (explicit, pos) = &link_pos[at];
                    let new = match explicit {
                        Some(id) => id.clone(),
                        None => {
                            let base = format!(
                                "{}-{}-{}",
                                l.kind.name(),
                                end_id(&l.source),
                                end_id(&l.target)
                            );
                            self.fresh(&base, "-")
                        }
                    };
                    self.positions.insert(new.clone(), *pos);
                    rename.insert(l.id.clone(), new);
                }
                write::Elem::Fan(k) => {
                    let f = &model.fans[k];
                    let new = if let Some(id) =
                        f.id.strip_prefix(TEMP)
                            .filter(|r| !r.chars().all(|c| c.is_ascii_digit()))
                    {
                        id.to_string()
                    } else {
                        let first = &model.links[index[&f.links[0]]];
                        let base = format!("fan-{}", first.process_end());
                        self.fresh(&base, "-")
                    };
                    if let Some(p) = self.positions.remove(&f.id) {
                        self.positions.insert(new.clone(), p);
                    }
                    rename.insert(f.id.clone(), new);
                }
            }
        }
        for l in &mut model.links {
            if let Some(n) = rename.get(&l.id) {
                l.id = n.clone();
            }
        }
        for f in &mut model.fans {
            if let Some(n) = rename.get(&f.id) {
                f.id = n.clone();
            }
            for m in &mut f.links {
                if let Some(n) = rename.get(m) {
                    *m = n.clone();
                }
            }
        }
    }
}

fn state_mut<'a>(model: &'a mut Model, k: usize, id: &str) -> &'a mut State {
    match &mut model.things[k].kind {
        ThingKind::Object { states } => states.iter_mut().find(|s| s.id == id).expect("state"),
        ThingKind::Process { .. } => unreachable!("states belong to objects"),
    }
}
