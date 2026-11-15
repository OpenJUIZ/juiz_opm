// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The abbreviated diagram (spec/diagram.md §3): a [`View`] gathered into symbols.
//!
//! Fixed groups of model elements become one symbol (§3.1 to §3.9): a Process with its argument
//! and return values becomes an ellipse with ports, a Container holds its ContainerProcesses,
//! an EC and its ec_cycle become a box with a `periodically invokes` line, a System becomes a
//! frame, a Transform a blue line. Every Thing and link that is in no group is kept in the OPM
//! form (§3.11). This module decides what is drawn; [`crate::layout`] places it and
//! [`crate::draw`] writes the SVG.

use std::collections::{HashMap, HashSet};

use opm_model::{
    Affiliation, Essence, Extensions, Link, LinkKind, Model, Modifier, Thing, JUIZ_NS,
};

use crate::dot::Status;
use crate::view::{End, ShownLink, View};

/// What a diagram is made of.
#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub title: String,
    /// Top-level symbols, in model order.
    pub nodes: Vec<Node>,
    pub frames: Vec<Frame>,
    pub edges: Vec<Edge>,
    /// Pairs of nodes to keep in this order from left to right (InZoom steps).
    pub order: Vec<(String, String)>,
}

impl Scene {
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// The top-level node that draws this Thing (a ContainerProcess is drawn by its Container).
    pub fn holder(&self, thing: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| {
            n.id == thing
                || matches!(&n.shape, Shape::Container { procs, .. } if procs.iter().any(|p| p.id == thing))
        })
    }

    pub fn frame(&self, id: &str) -> Option<&Frame> {
        self.frames.iter().find(|f| f.id == id)
    }
}

/// A symbol placed as one box.
#[derive(Debug, Clone)]
pub struct Node {
    /// The Thing id.
    pub id: String,
    pub shape: Shape,
    pub status: Status,
}

#[derive(Debug, Clone)]
pub enum Shape {
    /// §3.1 (and §3.9 when `dashed`), or an OPM Process (§3.11).
    Process(Proc),
    /// §3.3.
    Container {
        name: String,
        detail: String,
        procs: Vec<Proc>,
    },
    /// §3.4.
    Ec {
        name: String,
        detail: String,
        states: Vec<StateSym>,
    },
    /// An OPM Object (§3.11), or a Topic (§3.7, `sub` is `topic`).
    Object {
        name: String,
        sub: Option<String>,
        states: Vec<StateSym>,
        physical: bool,
        environmental: bool,
    },
}

/// A Process ellipse with its ports.
#[derive(Debug, Clone)]
pub struct Proc {
    pub id: String,
    pub name: String,
    /// The juiz type under the name.
    pub sub: Option<String>,
    /// ✎ or 👁 in a Container.
    pub mark: Option<&'static str>,
    pub args: Vec<Port>,
    /// The return value port and its label (the type).
    pub out: Option<String>,
    pub physical: bool,
    pub dashed: bool,
    pub status: Status,
}

/// An input port (§3.1).
#[derive(Debug, Clone)]
pub struct Port {
    pub name: String,
    /// Black: a line comes in. White: its own input value.
    pub connected: bool,
    pub note: Option<Note>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    /// `= value`, grey: `juiz:default`, or the value of a const.
    Default(String),
    /// `← value (p_apply)`, brown: `juiz:applied`.
    Applied(String),
}

#[derive(Debug, Clone)]
pub struct StateSym {
    pub id: String,
    pub name: String,
    pub initial: bool,
    pub final_: bool,
    pub default: bool,
}

/// A frame drawn around other symbols: a System (§3.5) or an in-zoomed Thing (§3.8).
#[derive(Debug, Clone)]
pub struct Frame {
    pub id: String,
    pub label: String,
    pub sub: Option<String>,
    /// Node and frame ids inside, in model order.
    pub members: Vec<String>,
    /// An in-zoomed Process: the OPM rounded frame. A System: a light grey frame.
    pub zoom: bool,
    pub status: Status,
}

/// Where a line starts or ends.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EndRef {
    /// The outline of the symbol of this Thing (node, Container process or frame).
    Thing(String),
    /// Input port `index` of a Process.
    Port(String, usize),
    /// The return value port of a Process.
    Out(String),
    /// A State drawn in an EC or an Object.
    State(String, String),
}

impl EndRef {
    pub fn thing(&self) -> &str {
        match self {
            EndRef::Thing(t) | EndRef::Port(t, _) | EndRef::Out(t) | EndRef::State(t, _) => t,
        }
    }
}

/// The mark at one end of a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Head {
    None,
    Arrow,
    OpenArrow,
    WhiteDot,
    BlackDot,
    /// The triangle of a structural link, at the whole.
    Triangle(LinkKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// §3.2 push.
    Push,
    /// §3.2 pull, and a value passed between nodes of a Composite.
    Pull,
    /// §3.4.
    EcInvokes,
    /// §3.9.
    Call,
    /// §3.6.
    Transform,
    /// §3.6.
    Pose,
    /// §3.7 publish.
    Publish,
    /// §3.7 subscribe.
    Subscribe,
    /// §3.11.
    Opm(LinkKind),
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub kind: EdgeKind,
    pub from: EndRef,
    pub to: EndRef,
    pub label: Option<String>,
    pub head: Head,
    pub tail: Head,
    pub status: Status,
    /// The model links this line stands for.
    pub links: Vec<String>,
}

fn role(ext: &Extensions) -> Option<&str> {
    ext.attribute(JUIZ_NS, "role")
}

fn juiz<'a>(ext: &'a Extensions, name: &str) -> Option<&'a str> {
    ext.attribute(JUIZ_NS, name)
}

/// A juiz value written as a child element (spec/xml.md §7.2) or an attribute.
fn jval(ext: &Extensions, name: &str) -> Option<String> {
    ext.element(JUIZ_NS, name)
        .map(|e| e.text().trim().to_string())
        .or_else(|| juiz(ext, name).map(str::to_string))
}

/// `juiz:translation` / `juiz:rotation` as `(x, y, z)`; omitted components are 0 (w is 1).
fn vector(ext: &Extensions, name: &str, keys: &[&str]) -> Option<String> {
    let e = ext.element(JUIZ_NS, name)?;
    let v: Vec<&str> = keys
        .iter()
        .map(|k| {
            e.attribute(None, k)
                .unwrap_or(if *k == "w" { "1" } else { "0" })
        })
        .collect();
    Some(format!("({})", v.join(", ")))
}

fn status(ext: &Extensions) -> Status {
    juiz(ext, "status")
        .and_then(Status::from_name)
        .unwrap_or(Status::Released)
}

fn states(t: &Thing, view: &View) -> Vec<StateSym> {
    t.states()
        .iter()
        .filter(|s| !view.hidden_states.contains(s.id.as_str()))
        .map(|s| StateSym {
            id: s.id.clone(),
            name: s.name.clone(),
            initial: s.initial,
            final_: s.final_,
            default: s.default,
        })
        .collect()
}

/// `juiz.Image` → `Image`, `a.B@sha256:...` → `B`.
fn short_type(t: &str) -> &str {
    let t = t.split('@').next().unwrap_or(t);
    t.rsplit('.').next().unwrap_or(t)
}

/// A rate in Hz as written: `1` → `1 Hz`.
fn hz(r: &str) -> String {
    format!("{} Hz", r)
}

struct Builder<'v, 'a> {
    view: &'v View<'a>,
    model: &'a Model,
    shown: HashSet<&'a str>,
    /// Things drawn inside another symbol or not at all.
    absorbed: HashSet<String>,
    /// Links drawn by a symbol (or left out by one).
    used: HashSet<String>,
    /// An absorbed value → where its lines go instead.
    value_at: HashMap<String, EndRef>,
    procs: HashMap<String, Proc>,
    scene: Scene,
}

/// Gather a view into the symbols of the abbreviated diagram.
pub fn build(view: &View) -> Scene {
    let model = view.model;
    let title = match (&model.name, view.diagram) {
        (Some(m), Some(d)) => format!("{} — {}", m, d.name),
        (Some(m), None) => m.clone(),
        (None, _) => view.name(),
    };
    let mut b = Builder {
        view,
        model,
        shown: view.things.iter().map(|t| t.id.as_str()).collect(),
        absorbed: HashSet::new(),
        used: HashSet::new(),
        value_at: HashMap::new(),
        procs: HashMap::new(),
        scene: Scene {
            title,
            ..Scene::default()
        },
    };
    b.processes();
    b.ecs();
    b.containers();
    b.transforms();
    b.calls_and_push();
    b.frames();
    b.nodes();
    b.rest();
    b.scene
}

impl<'v, 'a> Builder<'v, 'a> {
    fn role_of(&self, id: &str) -> Option<&'a str> {
        self.model.thing(id).and_then(|t| role(&t.extensions))
    }

    fn links(&self) -> impl Iterator<Item = &'v ShownLink<'a>> {
        self.view.links.iter()
    }

    /// Links of the model (shown or not) that touch this Thing or its States.
    fn touching(&self, thing: &'a Thing) -> Vec<&'a Link> {
        let ids: HashSet<&str> = std::iter::once(thing.id.as_str())
            .chain(thing.states().iter().map(|s| s.id.as_str()))
            .collect();
        self.model
            .links
            .iter()
            .filter(|l| ids.contains(l.source.as_str()) || ids.contains(l.target.as_str()))
            .collect()
    }

    fn producer(&self, value: &str) -> Option<&'v ShownLink<'a>> {
        self.links().find(|l| {
            l.link.kind == LinkKind::Result
                && l.target.thing == value
                && l.target.state.is_none()
                && self
                    .model
                    .thing(&l.source.thing)
                    .is_some_and(Thing::is_process)
        })
    }

    /// §3.1, §3.2, §3.6 (pose) and §3.7: every shown Process except an EC cycle gets an
    /// ellipse with its ports.
    fn processes(&mut self) {
        let procs: Vec<&'a Thing> = self
            .view
            .things
            .iter()
            .copied()
            .filter(|t| t.is_process() && self.role_of(&t.id) != Some("ec_cycle"))
            .collect();
        // return values first, so that connections find the port they start from
        for p in &procs {
            let mut out = None;
            let mut st = status(&p.extensions);
            for l in self.links() {
                if l.link.kind != LinkKind::Result || l.source.thing != p.id {
                    continue;
                }
                let Some(v) = self.model.thing(&l.target.thing) else {
                    continue;
                };
                match role(&v.extensions) {
                    Some("value") if out.is_none() && l.target.state.is_none() => {
                        out = Some(
                            juiz(&v.extensions, "datatype")
                                .map(|d| short_type(d).to_string())
                                .unwrap_or_default(),
                        );
                        st = st
                            .max(status(&v.extensions))
                            .max(status(&l.link.extensions));
                        self.absorbed.insert(v.id.clone());
                        self.used.insert(l.link.id.clone());
                        self.value_at
                            .insert(v.id.clone(), EndRef::Out(p.id.clone()));
                    }
                    Some("topic") => {
                        self.used.insert(l.link.id.clone());
                        self.scene.edges.push(Edge {
                            kind: EdgeKind::Publish,
                            from: EndRef::Out(p.id.clone()),
                            to: EndRef::Thing(v.id.clone()),
                            label: Some("publish".into()),
                            head: Head::Arrow,
                            tail: Head::None,
                            status: status(&l.link.extensions),
                            links: vec![l.link.id.clone()],
                        });
                        if out.is_none() {
                            out = Some(String::new());
                        }
                    }
                    _ => {}
                }
            }
            self.procs.insert(
                p.id.clone(),
                Proc {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    sub: juiz(&p.extensions, "type").map(str::to_string),
                    mark: None,
                    args: Vec::new(),
                    out,
                    physical: p.essence == Essence::Physical,
                    dashed: p.affiliation == Affiliation::Environmental,
                    status: st,
                },
            );
        }
        for p in &procs {
            self.ports(p);
        }
    }

    fn ports(&mut self, p: &'a Thing) {
        let mut args: Vec<(i64, &'v ShownLink<'a>)> = self
            .links()
            .filter(|l| {
                l.link.kind == LinkKind::Instrument
                    && l.target.thing == p.id
                    && juiz(&l.link.extensions, "arg").is_some()
            })
            .map(|l| {
                let i = juiz(&l.link.extensions, "arg_index")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(i64::MAX);
                (i, l)
            })
            .collect();
        args.sort_by_key(|(i, _)| *i);
        let mut st = Status::Released;
        let mut ports = Vec::new();
        for (index, (_, l)) in args.into_iter().enumerate() {
            let link = l.link;
            let name = juiz(&link.extensions, "arg")
                .unwrap_or_default()
                .to_string();
            let mut note = jval(&link.extensions, "applied").map(Note::Applied);
            let src = l.source.thing.as_str();
            let to = EndRef::Port(p.id.clone(), index);
            self.used.insert(link.id.clone());
            let lst = status(&link.extensions);
            let Some(x) = self.model.thing(src) else {
                continue;
            };
            let mut connected = true;
            let edge = |kind, from, label: Option<String>, head| Edge {
                kind,
                from,
                to: to.clone(),
                label,
                head,
                tail: Head::None,
                status: lst,
                links: vec![link.id.clone()],
            };
            match role(&x.extensions) {
                Some("value") => {
                    if let Some(prod) = self.producer(src).filter(|q| q.source.thing != p.id) {
                        // a connection, or a value passed between nodes (§3.2, §3.8)
                        let conn = juiz(&link.extensions, "connection");
                        let label = conn.map(|c| match juiz(&link.extensions, "connection_name") {
                            Some(n) => format!("{} ({})", c, n),
                            None => c.to_string(),
                        });
                        let kind = if conn == Some("push") {
                            EdgeKind::Push
                        } else {
                            EdgeKind::Pull
                        };
                        let from = EndRef::Out(prod.source.thing.clone());
                        self.scene.edges.push(edge(kind, from, label, Head::Arrow));
                    } else if self.only_used_by(x, &link.id) {
                        // its own input value (or a const)
                        connected = false;
                        self.absorbed.insert(x.id.clone());
                        self.value_at.insert(x.id.clone(), to.clone());
                        st = st.max(status(&x.extensions)).max(lst);
                        if note.is_none() {
                            note = jval(&x.extensions, "default")
                                .or_else(|| jval(&x.extensions, "value"))
                                .map(Note::Default);
                        }
                    } else {
                        let from = EndRef::Thing(x.id.clone());
                        self.scene
                            .edges
                            .push(edge(EdgeKind::Pull, from, None, Head::Arrow));
                    }
                }
                Some("topic") => {
                    let from = EndRef::Thing(x.id.clone());
                    let e = edge(
                        EdgeKind::Subscribe,
                        from,
                        Some("subscribe".into()),
                        Head::Arrow,
                    );
                    self.scene.edges.push(e);
                }
                Some("pose") => {
                    let owner = self
                        .links()
                        .find(|e| e.link.kind == LinkKind::Exhibition && e.target.thing == x.id);
                    let mut label = vec!["pose".to_string()];
                    if let Some(r) = juiz(&link.extensions, "reference") {
                        label.push(format!("ref: {}", r));
                    }
                    for f in ["initial", "on_update"] {
                        if juiz(&link.extensions, f) == Some("true") {
                            label.push(f.to_string());
                        }
                    }
                    let label = Some(label.join(" · "));
                    match owner {
                        Some(o) => {
                            self.used.insert(o.link.id.clone());
                            self.absorbed.insert(x.id.clone());
                            let from = EndRef::Thing(o.source.thing.clone());
                            let mut e = edge(EdgeKind::Pose, from, label, Head::Arrow);
                            e.links.push(o.link.id.clone());
                            self.scene.edges.push(e);
                        }
                        None => {
                            let from = EndRef::Thing(x.id.clone());
                            self.scene
                                .edges
                                .push(edge(EdgeKind::Pose, from, label, Head::Arrow));
                        }
                    }
                }
                _ => {
                    let from = match &l.source.state {
                        Some(s) => EndRef::State(x.id.clone(), s.clone()),
                        None => EndRef::Thing(x.id.clone()),
                    };
                    let mut e = edge(EdgeKind::Opm(LinkKind::Instrument), from, None, Head::None);
                    if let Some(m) = link.modifier {
                        e.label = Some(modifier(m).to_string());
                    }
                    self.scene.edges.push(e);
                }
            }
            ports.push(Port {
                name,
                connected,
                note,
            });
        }
        let proc = self.procs.get_mut(&p.id).expect("built above");
        proc.args = ports;
        proc.status = proc.status.max(st);
    }

    /// The value has no other link in the model than this one, so it can be drawn as a port.
    fn only_used_by(&self, x: &'a Thing, link: &str) -> bool {
        self.touching(x).iter().all(|l| {
            l.id == link
                || (l.kind == LinkKind::Aggregation && self.role_of(&l.source) == Some("system"))
        })
    }

    /// §3.4.
    fn ecs(&mut self) {
        for ec in self.view.things.clone() {
            if role(&ec.extensions) != Some("ec") {
                continue;
            }
            let state_ids: HashSet<&str> = ec.states().iter().map(|s| s.id.as_str()).collect();
            let mut detail = vec![];
            if let Some(t) = juiz(&ec.extensions, "type") {
                detail.push(t.to_string());
            }
            if let Some(r) = juiz(&ec.extensions, "rate") {
                detail.push(hz(r));
            }
            let mut st = status(&ec.extensions);
            // the cycles that run while the ec is in a state
            let cycles: Vec<(&'a Link, &'a Thing)> = self
                .model
                .links
                .iter()
                .filter(|l| {
                    l.kind == LinkKind::Instrument
                        && l.modifier == Some(Modifier::Condition)
                        && state_ids.contains(l.source.as_str())
                })
                .filter_map(|l| {
                    let c = self.model.thing(&l.target)?;
                    (role(&c.extensions) == Some("ec_cycle")).then_some((l, c))
                })
                .collect();
            for (cond, cycle) in cycles {
                self.absorbed.insert(cycle.id.clone());
                self.used.insert(cond.id.clone());
                st = st.max(status(&cycle.extensions));
                let links = self.touching(cycle);
                let periodic = links.iter().any(|l| {
                    l.kind == LinkKind::Invocation && l.source == cycle.id && l.target == cycle.id
                });
                let once = links.iter().any(|l| {
                    l.kind == LinkKind::Effect && l.source == cycle.id && l.target == ec.id
                });
                let mut binds: Vec<&&'a Link> = links
                    .iter()
                    .filter(|l| {
                        l.kind == LinkKind::Invocation
                            && l.source == cycle.id
                            && l.target != cycle.id
                    })
                    .collect();
                binds.sort_by_key(|l| {
                    juiz(&l.extensions, "order")
                        .and_then(|o| o.parse::<i64>().ok())
                        .unwrap_or(i64::MAX)
                });
                for l in &links {
                    self.used.insert(l.id.clone());
                }
                let n = binds.len();
                for b in binds {
                    if !self.shown.contains(b.target.as_str()) {
                        continue;
                    }
                    let mut label = if once {
                        "invokes once → stopped".to_string()
                    } else if periodic {
                        match juiz(&ec.extensions, "rate") {
                            Some(r) => format!("periodically invokes · {}", hz(r)),
                            None => "periodically invokes".to_string(),
                        }
                    } else {
                        "invokes".to_string()
                    };
                    if n > 1 {
                        if let Some(o) = juiz(&b.extensions, "order") {
                            label.push_str(&format!(" #{}", o));
                        }
                    }
                    let from = if self.view.hidden_states.contains(cond.source.as_str()) {
                        EndRef::Thing(ec.id.clone())
                    } else {
                        EndRef::State(ec.id.clone(), cond.source.clone())
                    };
                    self.scene.edges.push(Edge {
                        kind: EdgeKind::EcInvokes,
                        from,
                        to: EndRef::Thing(b.target.clone()),
                        label: Some(label),
                        head: Head::Arrow,
                        tail: Head::None,
                        status: status(&b.extensions),
                        links: vec![b.id.clone(), cond.id.clone()],
                    });
                }
            }
            self.scene.nodes.push(Node {
                id: ec.id.clone(),
                shape: Shape::Ec {
                    name: ec.name.clone(),
                    detail: detail.join(" · "),
                    states: states(ec, self.view),
                },
                status: st,
            });
            self.absorbed.insert(ec.id.clone());
        }
    }

    /// §3.3.
    fn containers(&mut self) {
        for c in self.view.things.clone() {
            let r = role(&c.extensions);
            if r != Some("container") && r != Some("composite_container") {
                continue;
            }
            let ty = juiz(&c.extensions, "type");
            let anchor = matches!(ty, Some("periodic_anchor" | "static_anchor"));
            let mut name = c.name.clone();
            if anchor {
                name = format!("⚓ {}", name);
            }
            let mut detail = vec![];
            if let Some(t) = ty {
                detail.push(t.to_string());
            }
            if let Some(rate) = juiz(&c.extensions, "rate") {
                detail.push(hz(rate));
            }
            if let Some(p) = jval(&c.extensions, "params") {
                detail.push(p);
            }
            let mut procs = vec![];
            let mut st = status(&c.extensions);
            let owned: Vec<&'v ShownLink<'a>> = self
                .links()
                .filter(|l| {
                    (l.link.kind == LinkKind::Effect && l.target.thing == c.id)
                        || (l.link.kind == LinkKind::Instrument && l.source.thing == c.id)
                })
                .collect();
            for l in owned {
                let p = if l.link.kind == LinkKind::Effect {
                    &l.source.thing
                } else {
                    &l.target.thing
                };
                if self.role_of(p) != Some("container_process") || self.absorbed.contains(p) {
                    continue;
                }
                let Some(mut proc) = self.procs.remove(p) else {
                    continue;
                };
                proc.mark = Some(if l.link.kind == LinkKind::Effect {
                    "✎"
                } else {
                    "👁"
                });
                proc.status = proc.status.max(status(&l.link.extensions));
                st = st.max(proc.status);
                self.used.insert(l.link.id.clone());
                self.absorbed.insert(p.clone());
                procs.push(proc);
            }
            self.scene.nodes.push(Node {
                id: c.id.clone(),
                shape: Shape::Container {
                    name,
                    detail: detail.join(" · "),
                    procs,
                },
                status: st,
            });
            self.absorbed.insert(c.id.clone());
        }
    }

    /// §3.6.
    fn transforms(&mut self) {
        for t in self.model.things.iter() {
            if role(&t.extensions) != Some("transform") {
                continue;
            }
            let tagged: Vec<&'a Link> = self
                .touching(t)
                .into_iter()
                .filter(|l| l.kind == LinkKind::Tagged && l.source == t.id)
                .collect();
            let end = |e: &str| {
                tagged
                    .iter()
                    .find(|l| juiz(&l.extensions, "end") == Some(e))
                    .map(|l| l.target.clone())
            };
            let (Some(a), Some(b)) = (end("source"), end("destination")) else {
                continue;
            };
            if !self.shown.contains(t.id.as_str()) {
                continue;
            }
            self.absorbed.insert(t.id.clone());
            for l in &tagged {
                self.used.insert(l.id.clone());
            }
            if !(self.shown.contains(a.as_str()) && self.shown.contains(b.as_str())) {
                continue;
            }
            let mut label = vec![];
            if let Some(tr) = vector(&t.extensions, "translation", &["x", "y", "z"]) {
                label.push(tr);
            }
            if let Some(r) = vector(&t.extensions, "rotation", &["x", "y", "z", "w"]) {
                label.push(format!("rot {}", r));
            }
            self.scene.edges.push(Edge {
                kind: EdgeKind::Transform,
                from: EndRef::Thing(a),
                to: EndRef::Thing(b),
                label: (!label.is_empty()).then(|| label.join(" ")),
                head: Head::None,
                tail: Head::None,
                status: status(&t.extensions),
                links: tagged.iter().map(|l| l.id.clone()).collect(),
            });
        }
    }

    /// §3.9 and the invocation of a push connection (§3.2).
    fn calls_and_push(&mut self) {
        let push: HashSet<(String, String)> = self
            .model
            .links
            .iter()
            .filter(|l| {
                l.kind == LinkKind::Instrument && juiz(&l.extensions, "connection") == Some("push")
            })
            .filter_map(|l| {
                let q = self
                    .model
                    .links
                    .iter()
                    .find(|r| r.kind == LinkKind::Result && r.target == l.source)?;
                Some((q.source.clone(), l.target.clone()))
            })
            .collect();
        for l in self.view.links.iter() {
            if l.link.kind != LinkKind::Invocation {
                continue;
            }
            if push.contains(&(l.link.source.clone(), l.link.target.clone())) {
                self.used.insert(l.link.id.clone());
            } else if self.role_of(&l.source.thing) == Some("client") {
                self.used.insert(l.link.id.clone());
                self.scene.edges.push(Edge {
                    kind: EdgeKind::Call,
                    from: EndRef::Thing(l.source.thing.clone()),
                    to: EndRef::Thing(l.target.thing.clone()),
                    label: Some("call".into()),
                    head: Head::Arrow,
                    tail: Head::None,
                    status: status(&l.link.extensions),
                    links: vec![l.link.id.clone()],
                });
            }
        }
    }

    /// §3.5 and §3.8.
    fn frames(&mut self) {
        let mut member_of: HashSet<String> = HashSet::new();
        // InZoom first: their members are closer than a System's
        for z in &self.view.zooms {
            let t = self
                .model
                .thing(&z.in_zoom.refinee)
                .expect("refinee exists");
            let members: Vec<String> = z
                .inner
                .iter()
                .filter(|i| !self.absorbed.contains(**i) || self.is_node(i))
                .map(|i| i.to_string())
                .collect();
            member_of.extend(members.iter().cloned());
            self.absorbed.insert(t.id.clone());
            self.procs.remove(&t.id);
            let steps: Vec<Vec<&String>> = z
                .in_zoom
                .steps
                .iter()
                .map(|s| s.iter().filter(|p| members.contains(p)).collect::<Vec<_>>())
                .filter(|s| !s.is_empty())
                .collect();
            for w in steps.windows(2) {
                for a in &w[0] {
                    for b in &w[1] {
                        self.scene.order.push(((*a).clone(), (*b).clone()));
                    }
                }
            }
            self.scene.frames.push(Frame {
                id: t.id.clone(),
                label: t.name.clone(),
                sub: juiz(&t.extensions, "type").map(str::to_string),
                members,
                zoom: true,
                status: status(&t.extensions),
            });
        }
        for s in self.view.things.clone() {
            if role(&s.extensions) != Some("system") {
                continue;
            }
            let mut members = vec![];
            for l in self.links() {
                if l.source.thing != s.id
                    || !matches!(l.link.kind, LinkKind::Aggregation | LinkKind::Exhibition)
                {
                    continue;
                }
                self.used.insert(l.link.id.clone());
                let m = &l.target.thing;
                if member_of.contains(m) || members.contains(m) {
                    continue;
                }
                if self.absorbed.contains(m) && !self.is_node(m) && !self.is_zoom(m) {
                    continue;
                }
                members.push(m.clone());
            }
            member_of.extend(members.iter().cloned());
            self.absorbed.insert(s.id.clone());
            self.scene.frames.push(Frame {
                id: s.id.clone(),
                label: s.name.clone(),
                sub: Some("system".into()),
                members,
                zoom: false,
                status: status(&s.extensions),
            });
        }
    }

    fn is_node(&self, id: &str) -> bool {
        self.scene.nodes.iter().any(|n| n.id == id) || self.procs.contains_key(id)
    }

    fn is_zoom(&self, id: &str) -> bool {
        self.view.zooms.iter().any(|z| z.in_zoom.refinee == id)
    }

    /// The process ellipses, and every other shown Thing in the OPM form (§3.11).
    fn nodes(&mut self) {
        for t in self.view.things.clone() {
            if let Some(p) = self.procs.remove(&t.id) {
                let status = p.status;
                self.scene.nodes.push(Node {
                    id: t.id.clone(),
                    shape: Shape::Process(p),
                    status,
                });
                continue;
            }
            if self.absorbed.contains(&t.id) {
                continue;
            }
            self.scene.nodes.push(Node {
                id: t.id.clone(),
                shape: Shape::Object {
                    name: t.name.clone(),
                    sub: (role(&t.extensions) == Some("topic")).then(|| "topic".to_string()),
                    states: states(t, self.view),
                    physical: t.essence == Essence::Physical,
                    environmental: t.affiliation == Affiliation::Environmental,
                },
                status: status(&t.extensions),
            });
        }
        // keep the model order
        let order: HashMap<&str, usize> = self
            .model
            .things
            .iter()
            .enumerate()
            .map(|(i, t)| (t.id.as_str(), i))
            .collect();
        self.scene
            .nodes
            .sort_by_key(|n| order.get(n.id.as_str()).copied().unwrap_or(usize::MAX));
    }

    /// Where a link end is drawn, or `None` when it is not drawn at all.
    fn end_ref(&self, e: &End) -> Option<EndRef> {
        if let Some(r) = self.value_at.get(&e.thing) {
            return Some(r.clone());
        }
        let drawn = self.scene.holder(&e.thing).is_some() || self.scene.frame(&e.thing).is_some();
        if !drawn {
            return None;
        }
        Some(match &e.state {
            Some(s) => EndRef::State(e.thing.clone(), s.clone()),
            None => EndRef::Thing(e.thing.clone()),
        })
    }

    /// §3.11: the links in no symbol.
    fn rest(&mut self) {
        let links: Vec<&'v ShownLink<'a>> = self
            .links()
            .filter(|l| !self.used.contains(&l.link.id))
            .collect();
        for l in links {
            let link = l.link;
            let (Some(from), Some(to)) = (self.end_ref(&l.source), self.end_ref(&l.target)) else {
                continue;
            };
            let mut label = vec![];
            let (mut head, mut tail) = (Head::None, Head::None);
            match link.kind {
                LinkKind::Consumption | LinkKind::Result => head = Head::Arrow,
                LinkKind::Instrument => head = Head::WhiteDot,
                LinkKind::Agent => head = Head::BlackDot,
                LinkKind::Effect => {
                    if l.from_state.is_some() || l.to_state.is_some() {
                        let obj = |s: &Option<String>| match s {
                            Some(s) => EndRef::State(l.target.thing.clone(), s.clone()),
                            None => to.clone(),
                        };
                        let st = status(&link.extensions);
                        for (a, b) in [
                            (obj(&l.from_state), from.clone()),
                            (from.clone(), obj(&l.to_state)),
                        ] {
                            self.scene.edges.push(Edge {
                                kind: EdgeKind::Opm(LinkKind::Effect),
                                from: a,
                                to: b,
                                label: None,
                                head: Head::Arrow,
                                tail: Head::None,
                                status: st,
                                links: vec![link.id.clone()],
                            });
                        }
                        continue;
                    }
                    head = Head::Arrow;
                    tail = Head::Arrow;
                }
                LinkKind::Invocation => {
                    head = Head::Arrow;
                    label.push("invokes".to_string());
                }
                LinkKind::OvertimeException => {
                    head = Head::Arrow;
                    label.push("overtime".to_string());
                }
                LinkKind::UndertimeException => {
                    head = Head::Arrow;
                    label.push("undertime".to_string());
                }
                LinkKind::Tagged => {
                    head = Head::OpenArrow;
                    let mut t = link.tag.clone().unwrap_or_default();
                    if let Some(b) = &link.backward_tag {
                        t = format!("{} / {}", t, b);
                        tail = Head::OpenArrow;
                    }
                    label.push(t);
                }
                k => tail = Head::Triangle(k),
            }
            if let Some(m) = link.modifier {
                label.push(modifier(m).to_string());
            }
            if let Some(p) = &link.path {
                label.push(format!("path {}", p));
            }
            if let Some(op) = self
                .model
                .fans
                .iter()
                .find(|f| f.links.contains(&link.id))
                .map(|f| f.operator)
            {
                label.push(match op {
                    opm_model::FanOperator::Xor => "XOR".into(),
                    opm_model::FanOperator::Or => "OR".into(),
                });
            }
            self.scene.edges.push(Edge {
                kind: EdgeKind::Opm(link.kind),
                from,
                to,
                label: (!label.is_empty()).then(|| label.join(" ")),
                head,
                tail,
                status: status(&link.extensions),
                links: vec![link.id.clone()],
            });
        }
    }
}

fn modifier(m: Modifier) -> &'static str {
    match m {
        Modifier::Event => "e",
        Modifier::Condition => "c",
    }
}
