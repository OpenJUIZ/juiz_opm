// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The juiz conf (YAML or JSON) in the range that round-trips (spec/conf.md).
//!
//! [`Conf::read`] reads a conf as juiz does (namespaces resolved, the old connection forms
//! accepted) and [`Conf::write`] writes the canonical form. A conf and the conf made back from
//! its model are compared in this form.

use serde_json::{Map, Value as Json};

/// A conf in the round-trip range. Names are resolved: `namespace` is the absolute namespace
/// without a leading `/` (empty for the root), topic names are full names.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Conf {
    pub name: String,
    pub composites: Vec<Composite>,
    pub processes: Vec<Process>,
    pub containers: Vec<Container>,
    pub composite_containers: Vec<CompositeContainer>,
    pub connections: Vec<Connection>,
    pub ecs: Vec<Ec>,
    pub transforms: Vec<Transform>,
    pub pose_listeners: Vec<PoseListener>,
    pub subsystems: Vec<Subsystem>,
}

/// A Process, or a ContainerProcess inside a container.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Process {
    pub type_name: String,
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub use_memo: Option<bool>,
    /// Full topic names.
    pub publishes: Vec<String>,
    /// (argument, full topic name).
    pub subscribes: Vec<(String, String)>,
    pub p_apply: Vec<(String, Json)>,
    /// The other keys, in their order.
    pub params: Map<String, Json>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Container {
    pub type_name: String,
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    /// Only for the anchors (`periodic_anchor`, `static_anchor`); elsewhere it is a param.
    pub rate: Option<f64>,
    pub params: Map<String, Json>,
    pub processes: Vec<Process>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CompositeContainer {
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    /// (slot, full name of the container).
    pub slots: Vec<(String, String)>,
    pub params: Map<String, Json>,
    pub processes: Vec<Process>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Composite {
    pub type_name: String,
    pub description: Option<String>,
    /// (slot, container type).
    pub containers: Vec<(String, String)>,
    pub body: Node,
    pub params: Map<String, Json>,
}

/// A node of the expression tree of a Composite (juiz composition.md §3).
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Call {
        process: String,
        container: Option<String>,
        /// In the order of the conf (the manifest's order is not known from the conf).
        args: Vec<(String, Node)>,
    },
    Const(Json),
    Expose {
        name: String,
        default: Option<Json>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Push,
    Pull,
}

impl ConnectionType {
    pub fn word(self) -> &'static str {
        match self {
            ConnectionType::Push => "push",
            ConnectionType::Pull => "pull",
        }
    }
}

/// A reference to a Process or ContainerProcess by name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct ProcessRef {
    pub namespace: String,
    pub name: String,
    pub type_name: String,
    pub container_name: Option<String>,
}

impl ProcessRef {
    pub fn full_name(&self) -> String {
        join(&self.namespace, &self.name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub name: Option<String>,
    pub connection_type: ConnectionType,
    pub source: ProcessRef,
    pub destination: ProcessRef,
    pub arg_name: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ec {
    pub type_name: String,
    pub name: String,
    pub namespace: String,
    pub description: Option<String>,
    pub rate: Option<f64>,
    pub auto_start: bool,
    /// In the order of `bind`.
    pub bind: Vec<ProcessRef>,
    pub params: Map<String, Json>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transform {
    /// Full name of a container of this System, or the ID of a container of another one.
    pub source: String,
    pub destination: String,
    pub translation: Option<[f64; 3]>,
    pub rotation: Option<[f64; 4]>,
    pub writers: Vec<String>,
    pub params: Map<String, Json>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoseListener {
    /// Full names.
    pub container: String,
    pub process: String,
    pub arg: String,
    pub reference: Option<String>,
    pub initial: Option<bool>,
    pub on_update: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Subsystem {
    pub type_name: String,
    pub name: String,
    pub params: Map<String, Json>,
}

/// Top-level keys that are outside the model (spec/metamodel.md §10).
pub const OUT_OF_RANGE: &[&str] = &[
    "plugins",
    "option",
    "brokers",
    "broker_proxies",
    "mastersystem",
];

/// `a/b` from a namespace and a name; the name alone in the root namespace.
pub fn join(namespace: &str, name: &str) -> String {
    if namespace.is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", namespace, name)
    }
}

/// Split `a/b/c` into (`a/b`, `c`).
pub fn split(full: &str) -> (String, String) {
    match full.rsplit_once('/') {
        Some((ns, n)) => (ns.to_string(), n.to_string()),
        None => (String::new(), full.to_string()),
    }
}

/// Resolve a namespace written in the conf: `None` is `base`, `/x` is absolute, `x` is under
/// `base` (juiz system_builder/namespaces.rs).
fn resolve_namespace(base: &str, written: Option<&str>) -> String {
    match written {
        None | Some("") => base.to_string(),
        Some(ns) => match ns.strip_prefix('/') {
            Some(abs) => abs.trim_end_matches('/').to_string(),
            None => join(base, ns.trim_end_matches('/')),
        },
    }
}

/// Resolve a name that may hold a namespace (topic names, slot targets, transform ends):
/// `/a/b` is absolute, `a/b` and `b` are under `base`.
pub fn resolve_name(base: &str, written: &str) -> String {
    match written.strip_prefix('/') {
        Some(abs) => abs.to_string(),
        None => join(base, written),
    }
}

/// Write a full name relative to `base`: without the namespace when it is `base`, absolute
/// (with a leading `/`) when `base` is not the root and the name is elsewhere.
pub fn relative_name(base: &str, full: &str) -> String {
    if base.is_empty() {
        return full.to_string();
    }
    match full.strip_prefix(base).and_then(|r| r.strip_prefix('/')) {
        Some(rest) if !rest.contains('/') => rest.to_string(),
        _ => format!("/{}", full),
    }
}

/// What reading found that is not carried: warnings, or errors that stop the conversion.
#[derive(Debug, Clone, Default)]
pub struct Notes {
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

type R<T> = Result<T, String>;

fn as_obj<'a>(v: &'a Json, what: &str) -> R<&'a Map<String, Json>> {
    v.as_object()
        .ok_or_else(|| format!("{} must be a mapping, but is {}", what, v))
}

fn as_arr<'a>(v: &'a Json, what: &str) -> R<&'a Vec<Json>> {
    v.as_array()
        .ok_or_else(|| format!("{} must be a list, but is {}", what, v))
}

fn get_str(o: &Map<String, Json>, key: &str, what: &str) -> R<String> {
    match o.get(key) {
        Some(Json::String(s)) => Ok(s.clone()),
        Some(v) => Err(format!(
            "{}: `{}` must be a string, but is {}",
            what, key, v
        )),
        None => Err(format!("{}: `{}` is missing", what, key)),
    }
}

fn opt_str(o: &Map<String, Json>, key: &str, what: &str) -> R<Option<String>> {
    match o.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::String(s)) => Ok(Some(s.clone())),
        Some(v) => Err(format!(
            "{}: `{}` must be a string, but is {}",
            what, key, v
        )),
    }
}

fn opt_bool(o: &Map<String, Json>, key: &str, what: &str) -> R<Option<bool>> {
    match o.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(Json::Bool(b)) => Ok(Some(*b)),
        Some(v) => Err(format!(
            "{}: `{}` must be true or false, but is {}",
            what, key, v
        )),
    }
}

fn opt_f64(o: &Map<String, Json>, key: &str, what: &str) -> R<Option<f64>> {
    match o.get(key) {
        None | Some(Json::Null) => Ok(None),
        Some(v) => v
            .as_f64()
            .map(Some)
            .ok_or_else(|| format!("{}: `{}` must be a number, but is {}", what, key, v)),
    }
}

fn rest(o: &Map<String, Json>, known: &[&str]) -> Map<String, Json> {
    o.iter()
        .filter(|(k, _)| !known.contains(&k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

/// A juiz identifier (`core://core/container_process/ns/read0::type:container`) as a reference.
pub fn parse_identifier(id: &str) -> Option<(String, ProcessRef)> {
    let (_, after) = id.split_once("://")?;
    let mut parts = after.splitn(3, '/');
    let _broker = parts.next()?;
    let class = parts.next()?.to_string();
    let rest = parts.next()?;
    let (full, ty) = rest.rsplit_once("::")?;
    let (namespace, name) = split(full);
    let (type_name, container_name) = match ty.split_once(':') {
        Some((t, c)) => (t.to_string(), Some(c.to_string())),
        None => (ty.to_string(), None),
    };
    Some((
        class,
        ProcessRef {
            namespace,
            name,
            type_name,
            container_name,
        },
    ))
}

fn process_ref(v: &Json, base: &str, what: &str) -> R<ProcessRef> {
    let id = match v {
        Json::String(s) => Some(s.clone()),
        Json::Object(o) => match o.get("identifier").or_else(|| o.get("id")) {
            Some(Json::String(s)) => Some(s.clone()),
            Some(v) => return Err(format!("{}: the identifier must be a string: {}", what, v)),
            None => None,
        },
        _ => {
            return Err(format!(
                "{}: a process reference must be a mapping: {}",
                what, v
            ))
        }
    };
    if let Some(id) = id {
        // An ID is absolute (juiz does not resolve it against a namespace).
        let (_, r) = parse_identifier(&id)
            .ok_or_else(|| format!("{}: cannot read the ID {:?}", what, id))?;
        return Ok(r);
    }
    let o = as_obj(v, what)?;
    let ns = resolve_namespace(base, opt_str(o, "namespace", what)?.as_deref());
    Ok(ProcessRef {
        namespace: ns,
        name: get_str(o, "name", what)?,
        type_name: get_str(o, "type_name", what)?,
        container_name: opt_str(o, "container_name", what)?,
    })
}

/// Numbers that juiz reads as f64 (rates, vectors) are written as floats.
fn json_number(x: f64) -> Json {
    Json::from(x)
}

impl Conf {
    /// Read a conf written in YAML (JSON is YAML too).
    pub fn read(text: &str) -> (Option<Conf>, Notes) {
        let mut notes = Notes::default();
        let value: Json = match serde_yaml::from_str(text) {
            Ok(v) => v,
            Err(e) => {
                notes.errors.push(format!("YAML: {}", e));
                return (None, notes);
            }
        };
        match Conf::from_json(&value, &mut notes) {
            Ok(c) => (Some(c), notes),
            Err(e) => {
                notes.errors.push(e);
                (None, notes)
            }
        }
    }

    pub fn from_json(value: &Json, notes: &mut Notes) -> R<Conf> {
        let top = as_obj(value, "the conf")?;
        let base = resolve_namespace("", opt_str(top, "namespace", "the conf")?.as_deref());
        let mut conf = Conf {
            name: get_str(top, "name", "the conf")?,
            ..Default::default()
        };
        for (k, v) in top {
            match k.as_str() {
                "name" | "namespace" => {}
                "processes" => {
                    for (i, p) in as_arr(v, "processes")?.iter().enumerate() {
                        let what = format!("processes[{}]", i);
                        let o = as_obj(p, &what)?;
                        let ns =
                            resolve_namespace(&base, opt_str(o, "namespace", &what)?.as_deref());
                        conf.processes.push(read_process(o, ns, &what)?);
                    }
                }
                "containers" => {
                    for (i, c) in as_arr(v, "containers")?.iter().enumerate() {
                        let what = format!("containers[{}]", i);
                        conf.containers.push(read_container(c, &base, &what)?);
                    }
                }
                "composite_containers" => {
                    for (i, c) in as_arr(v, "composite_containers")?.iter().enumerate() {
                        let what = format!("composite_containers[{}]", i);
                        conf.composite_containers
                            .push(read_composite_container(c, &base, &what)?);
                    }
                }
                "composites" => {
                    for (i, c) in as_arr(v, "composites")?.iter().enumerate() {
                        let what = format!("composites[{}]", i);
                        conf.composites.push(read_composite(c, &what)?);
                    }
                }
                "connections" => {
                    for (i, c) in as_arr(v, "connections")?.iter().enumerate() {
                        let what = format!("connections[{}]", i);
                        conf.connections
                            .push(read_connection(c, &base, &what, notes)?);
                    }
                }
                "ecs" => {
                    for (i, c) in as_arr(v, "ecs")?.iter().enumerate() {
                        let what = format!("ecs[{}]", i);
                        conf.ecs.push(read_ec(c, &base, &what)?);
                    }
                }
                "transforms" => {
                    for (i, c) in as_arr(v, "transforms")?.iter().enumerate() {
                        let what = format!("transforms[{}]", i);
                        conf.transforms.push(read_transform(c, &base, &what)?);
                    }
                }
                "pose_listeners" => {
                    for (i, c) in as_arr(v, "pose_listeners")?.iter().enumerate() {
                        let what = format!("pose_listeners[{}]", i);
                        conf.pose_listeners
                            .push(read_pose_listener(c, &base, &what)?);
                    }
                }
                "subsystems" => {
                    for (i, c) in as_arr(v, "subsystems")?.iter().enumerate() {
                        let what = format!("subsystems[{}]", i);
                        let o = as_obj(c, &what)?;
                        conf.subsystems.push(Subsystem {
                            type_name: get_str(o, "type_name", &what)?,
                            name: get_str(o, "name", &what)?,
                            params: rest(o, &["type_name", "name"]),
                        });
                    }
                }
                k if OUT_OF_RANGE.contains(&k) => {
                    notes
                        .warnings
                        .push(format!("`{}` is outside the model and is not carried", k));
                }
                k => notes
                    .warnings
                    .push(format!("unknown top-level key `{}` is not carried", k)),
            }
        }
        conf.canonicalize();
        Ok(conf)
    }

    /// Sort the lists into the canonical order (spec/conf.md §3). Orders that juiz uses (`bind`,
    /// the arguments of a Composite node, `params`) are kept.
    pub fn canonicalize(&mut self) {
        // A reference to a ContainerProcess names its container (as `bind` asks for).
        let mut owners = std::collections::HashMap::new();
        for c in &self.containers {
            for p in &c.processes {
                owners.insert(join(&p.namespace, &p.name), c.name.clone());
            }
        }
        for c in &self.composite_containers {
            for p in &c.processes {
                owners.insert(join(&p.namespace, &p.name), c.name.clone());
            }
        }
        let fill = |r: &mut ProcessRef| {
            if let Some(o) = owners.get(&r.full_name()) {
                r.container_name = Some(o.clone());
            }
        };
        for c in &mut self.connections {
            fill(&mut c.source);
            fill(&mut c.destination);
        }
        for e in &mut self.ecs {
            for b in &mut e.bind {
                fill(b);
            }
        }
        fn sort_procs(ps: &mut [Process]) {
            for p in ps.iter_mut() {
                p.publishes.sort();
                p.publishes.dedup();
                p.subscribes.sort();
                p.p_apply.sort_by(|a, b| a.0.cmp(&b.0));
            }
            ps.sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
        }
        sort_procs(&mut self.processes);
        for c in &mut self.containers {
            sort_procs(&mut c.processes);
        }
        self.containers
            .sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
        for c in &mut self.composite_containers {
            sort_procs(&mut c.processes);
            c.slots.sort();
        }
        self.composite_containers
            .sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
        for c in &mut self.composites {
            c.containers.sort();
        }
        self.composites = order_composites(std::mem::take(&mut self.composites));
        self.connections.sort_by(|a, b| {
            (&a.destination, &a.arg_name, &a.source, &a.name).cmp(&(
                &b.destination,
                &b.arg_name,
                &b.source,
                &b.name,
            ))
        });
        self.ecs
            .sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
        for t in &mut self.transforms {
            t.writers.sort();
        }
        self.transforms
            .sort_by(|a, b| (&a.source, &a.destination).cmp(&(&b.source, &b.destination)));
        self.pose_listeners.sort_by(|a, b| {
            (&a.container, &a.process, &a.arg).cmp(&(&b.container, &b.process, &b.arg))
        });
        self.subsystems
            .sort_by(|a, b| (&a.type_name, &a.name).cmp(&(&b.type_name, &b.name)));
    }

    /// The canonical conf as a JSON value (keys in the canonical order).
    pub fn to_json(&self) -> Json {
        let mut top = Map::new();
        top.insert("name".into(), self.name.clone().into());
        if !self.composites.is_empty() {
            top.insert(
                "composites".into(),
                self.composites.iter().map(write_composite).collect(),
            );
        }
        if !self.processes.is_empty() {
            top.insert(
                "processes".into(),
                self.processes
                    .iter()
                    .map(|p| write_process(p, false))
                    .collect(),
            );
        }
        if !self.containers.is_empty() {
            top.insert(
                "containers".into(),
                self.containers.iter().map(write_container).collect(),
            );
        }
        if !self.composite_containers.is_empty() {
            top.insert(
                "composite_containers".into(),
                self.composite_containers
                    .iter()
                    .map(write_composite_container)
                    .collect(),
            );
        }
        if !self.connections.is_empty() {
            top.insert(
                "connections".into(),
                self.connections.iter().map(write_connection).collect(),
            );
        }
        if !self.ecs.is_empty() {
            top.insert("ecs".into(), self.ecs.iter().map(write_ec).collect());
        }
        if !self.transforms.is_empty() {
            top.insert(
                "transforms".into(),
                self.transforms.iter().map(write_transform).collect(),
            );
        }
        if !self.pose_listeners.is_empty() {
            top.insert(
                "pose_listeners".into(),
                self.pose_listeners
                    .iter()
                    .map(write_pose_listener)
                    .collect(),
            );
        }
        if !self.subsystems.is_empty() {
            top.insert(
                "subsystems".into(),
                self.subsystems
                    .iter()
                    .map(|s| {
                        let mut o = Map::new();
                        o.insert("type_name".into(), s.type_name.clone().into());
                        o.insert("name".into(), s.name.clone().into());
                        extend(&mut o, &s.params);
                        Json::Object(o)
                    })
                    .collect(),
            );
        }
        Json::Object(top)
    }

    /// The canonical conf in YAML.
    pub fn write(&self) -> String {
        serde_yaml::to_string(&self.to_json()).expect("a JSON value is always YAML")
    }
}

/// Composites in an order where a Composite comes after those it uses, by type name otherwise.
fn order_composites(mut cs: Vec<Composite>) -> Vec<Composite> {
    cs.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    let names: Vec<String> = cs.iter().map(|c| c.type_name.clone()).collect();
    let mut out: Vec<Composite> = Vec::new();
    while !cs.is_empty() {
        let pos = cs
            .iter()
            .position(|c| {
                let mut uses = Vec::new();
                node_types(&c.body, &mut uses);
                uses.iter().all(|u| {
                    *u == c.type_name || !names.contains(u) || out.iter().any(|o| &o.type_name == u)
                })
            })
            .unwrap_or(0); // a cycle: keep the name order
        out.push(cs.remove(pos));
    }
    out
}

fn node_types(n: &Node, out: &mut Vec<String>) {
    if let Node::Call { process, args, .. } = n {
        out.push(process.clone());
        for (_, a) in args {
            node_types(a, out);
        }
    }
}

const PROCESS_KEYS: &[&str] = &[
    "type_name",
    "name",
    "namespace",
    "description",
    "use_memo",
    "publishes",
    "subscribes",
    "p_apply",
];

fn read_process(o: &Map<String, Json>, ns: String, what: &str) -> R<Process> {
    let mut p = Process {
        type_name: get_str(o, "type_name", what)?,
        name: get_str(o, "name", what)?,
        description: opt_str(o, "description", what)?,
        use_memo: opt_bool(o, "use_memo", what)?,
        params: rest(o, PROCESS_KEYS),
        namespace: ns,
        ..Default::default()
    };
    if let Some(v) = o.get("publishes") {
        for t in as_arr(v, &format!("{}.publishes", what))? {
            let t = t
                .as_str()
                .ok_or_else(|| format!("{}.publishes: a topic name must be a string", what))?;
            p.publishes.push(resolve_name(&p.namespace, t));
        }
    }
    if let Some(v) = o.get("subscribes") {
        for (arg, t) in as_obj(v, &format!("{}.subscribes", what))? {
            let t = t
                .as_str()
                .ok_or_else(|| format!("{}.subscribes: a topic name must be a string", what))?;
            p.subscribes
                .push((arg.clone(), resolve_name(&p.namespace, t)));
        }
    }
    if let Some(v) = o.get("p_apply") {
        for (arg, x) in as_obj(v, &format!("{}.p_apply", what))? {
            p.p_apply.push((arg.clone(), x.clone()));
        }
    }
    Ok(p)
}

fn write_process(p: &Process, in_container: bool) -> Json {
    let mut o = Map::new();
    o.insert("type_name".into(), p.type_name.clone().into());
    o.insert("name".into(), p.name.clone().into());
    if !in_container && !p.namespace.is_empty() {
        o.insert("namespace".into(), p.namespace.clone().into());
    }
    if let Some(d) = &p.description {
        o.insert("description".into(), d.clone().into());
    }
    if let Some(b) = p.use_memo {
        o.insert("use_memo".into(), b.into());
    }
    if !p.publishes.is_empty() {
        o.insert(
            "publishes".into(),
            p.publishes
                .iter()
                .map(|t| Json::from(relative_name(&p.namespace, t)))
                .collect(),
        );
    }
    if !p.subscribes.is_empty() {
        o.insert(
            "subscribes".into(),
            Json::Object(
                p.subscribes
                    .iter()
                    .map(|(a, t)| (a.clone(), Json::from(relative_name(&p.namespace, t))))
                    .collect(),
            ),
        );
    }
    if !p.p_apply.is_empty() {
        o.insert(
            "p_apply".into(),
            Json::Object(p.p_apply.iter().cloned().collect()),
        );
    }
    extend(&mut o, &p.params);
    Json::Object(o)
}

fn extend(o: &mut Map<String, Json>, params: &Map<String, Json>) {
    for (k, v) in params {
        o.insert(k.clone(), v.clone());
    }
}

/// The type names of the anchors (juiz transformations.md §4).
pub const ANCHOR_TYPES: &[&str] = &["periodic_anchor", "static_anchor"];

fn read_container(v: &Json, base: &str, what: &str) -> R<Container> {
    let o = as_obj(v, what)?;
    let ns = resolve_namespace(base, opt_str(o, "namespace", what)?.as_deref());
    let type_name = get_str(o, "type_name", what)?;
    let anchor = ANCHOR_TYPES.contains(&type_name.as_str());
    let mut known = vec!["type_name", "name", "namespace", "description", "processes"];
    if anchor {
        known.push("rate");
    }
    let mut c = Container {
        name: get_str(o, "name", what)?,
        description: opt_str(o, "description", what)?,
        rate: if anchor {
            opt_f64(o, "rate", what)?
        } else {
            None
        },
        params: rest(o, &known),
        processes: Vec::new(),
        namespace: ns,
        type_name,
    };
    if let Some(ps) = o.get("processes") {
        for (i, p) in as_arr(ps, &format!("{}.processes", what))?
            .iter()
            .enumerate()
        {
            let w = format!("{}.processes[{}]", what, i);
            c.processes
                .push(read_process(as_obj(p, &w)?, c.namespace.clone(), &w)?);
        }
    }
    Ok(c)
}

fn write_container(c: &Container) -> Json {
    let mut o = Map::new();
    o.insert("type_name".into(), c.type_name.clone().into());
    o.insert("name".into(), c.name.clone().into());
    if !c.namespace.is_empty() {
        o.insert("namespace".into(), c.namespace.clone().into());
    }
    if let Some(d) = &c.description {
        o.insert("description".into(), d.clone().into());
    }
    if let Some(r) = c.rate {
        o.insert("rate".into(), json_number(r));
    }
    extend(&mut o, &c.params);
    if !c.processes.is_empty() {
        o.insert(
            "processes".into(),
            c.processes.iter().map(|p| write_process(p, true)).collect(),
        );
    }
    Json::Object(o)
}

fn read_composite_container(v: &Json, base: &str, what: &str) -> R<CompositeContainer> {
    let o = as_obj(v, what)?;
    let ns = resolve_namespace(base, opt_str(o, "namespace", what)?.as_deref());
    let mut c = CompositeContainer {
        name: get_str(o, "name", what)?,
        description: opt_str(o, "description", what)?,
        params: rest(
            o,
            &["name", "namespace", "description", "slots", "processes"],
        ),
        namespace: ns,
        ..Default::default()
    };
    if let Some(s) = o.get("slots") {
        for (slot, target) in as_obj(s, &format!("{}.slots", what))? {
            let t = target
                .as_str()
                .ok_or_else(|| format!("{}.slots.{}: must be a container name", what, slot))?;
            c.slots.push((slot.clone(), resolve_name(&c.namespace, t)));
        }
    }
    if let Some(ps) = o.get("processes") {
        for (i, p) in as_arr(ps, &format!("{}.processes", what))?
            .iter()
            .enumerate()
        {
            let w = format!("{}.processes[{}]", what, i);
            c.processes
                .push(read_process(as_obj(p, &w)?, c.namespace.clone(), &w)?);
        }
    }
    Ok(c)
}

fn write_composite_container(c: &CompositeContainer) -> Json {
    let mut o = Map::new();
    o.insert("name".into(), c.name.clone().into());
    if !c.namespace.is_empty() {
        o.insert("namespace".into(), c.namespace.clone().into());
    }
    if let Some(d) = &c.description {
        o.insert("description".into(), d.clone().into());
    }
    o.insert(
        "slots".into(),
        Json::Object(
            c.slots
                .iter()
                .map(|(s, t)| (s.clone(), Json::from(relative_name(&c.namespace, t))))
                .collect(),
        ),
    );
    extend(&mut o, &c.params);
    if !c.processes.is_empty() {
        o.insert(
            "processes".into(),
            c.processes.iter().map(|p| write_process(p, true)).collect(),
        );
    }
    Json::Object(o)
}

fn read_composite(v: &Json, what: &str) -> R<Composite> {
    let o = as_obj(v, what)?;
    let mut containers = Vec::new();
    if let Some(s) = o.get("containers") {
        for (slot, ty) in as_obj(s, &format!("{}.containers", what))? {
            let t = ty
                .as_str()
                .ok_or_else(|| format!("{}.containers.{}: must be a type name", what, slot))?;
            containers.push((slot.clone(), t.to_string()));
        }
    }
    let body = o
        .get("body")
        .ok_or_else(|| format!("{}: `body` is missing", what))?;
    Ok(Composite {
        type_name: get_str(o, "type_name", what)?,
        description: opt_str(o, "description", what)?,
        containers,
        body: read_node(body, &format!("{}.body", what))?,
        params: rest(o, &["type_name", "description", "containers", "body"]),
    })
}

fn read_node(v: &Json, what: &str) -> R<Node> {
    let o = as_obj(v, what)?;
    if let Some(c) = o.get("const") {
        return Ok(Node::Const(c.clone()));
    }
    if let Some(e) = o.get("expose") {
        return match e {
            Json::String(s) => Ok(Node::Expose {
                name: s.clone(),
                default: None,
            }),
            Json::Object(eo) => Ok(Node::Expose {
                name: get_str(eo, "name", what)?,
                default: eo.get("default").cloned(),
            }),
            _ => Err(format!("{}: `expose` must be a name or a mapping", what)),
        };
    }
    let mut args = Vec::new();
    if let Some(a) = o.get("args") {
        for (k, n) in as_obj(a, &format!("{}.args", what))? {
            args.push((k.clone(), read_node(n, &format!("{}.{}", what, k))?));
        }
    }
    Ok(Node::Call {
        process: get_str(o, "process", what)?,
        container: opt_str(o, "container", what)?,
        args,
    })
}

fn write_node(n: &Node) -> Json {
    let mut o = Map::new();
    match n {
        Node::Const(v) => {
            o.insert("const".into(), v.clone());
        }
        Node::Expose {
            name,
            default: None,
        } => {
            o.insert("expose".into(), name.clone().into());
        }
        Node::Expose {
            name,
            default: Some(d),
        } => {
            let mut e = Map::new();
            e.insert("name".into(), name.clone().into());
            e.insert("default".into(), d.clone());
            o.insert("expose".into(), Json::Object(e));
        }
        Node::Call {
            process,
            container,
            args,
        } => {
            o.insert("process".into(), process.clone().into());
            if let Some(c) = container {
                o.insert("container".into(), c.clone().into());
            }
            if !args.is_empty() {
                o.insert(
                    "args".into(),
                    Json::Object(
                        args.iter()
                            .map(|(k, a)| (k.clone(), write_node(a)))
                            .collect(),
                    ),
                );
            }
        }
    }
    Json::Object(o)
}

fn write_composite(c: &Composite) -> Json {
    let mut o = Map::new();
    o.insert("type_name".into(), c.type_name.clone().into());
    if let Some(d) = &c.description {
        o.insert("description".into(), d.clone().into());
    }
    if !c.containers.is_empty() {
        o.insert(
            "containers".into(),
            Json::Object(
                c.containers
                    .iter()
                    .map(|(s, t)| (s.clone(), Json::from(t.clone())))
                    .collect(),
            ),
        );
    }
    o.insert("body".into(), write_node(&c.body));
    extend(&mut o, &c.params);
    Json::Object(o)
}

fn read_connection(v: &Json, base: &str, what: &str, notes: &mut Notes) -> R<Connection> {
    let o = as_obj(v, what)?;
    let ns = resolve_namespace(base, opt_str(o, "namespace", what)?.as_deref());
    let ty = match (
        opt_str(o, "connection_type", what)?,
        opt_str(o, "type", what)?,
    ) {
        (Some(t), _) => t,
        (None, Some(t)) => {
            notes.warnings.push(format!(
                "{}: `type` is read as `connection_type` (the old form)",
                what
            ));
            t
        }
        (None, None) => "push".to_string(),
    };
    let connection_type = match ty.to_ascii_lowercase().as_str() {
        "push" => ConnectionType::Push,
        "pull" => ConnectionType::Pull,
        _ => return Err(format!("{}: unknown connection_type {:?}", what, ty)),
    };
    let end = |key: &str| -> R<ProcessRef> {
        let v = o
            .get(key)
            .or_else(|| o.get(&format!("{}_process_id", key)))
            .ok_or_else(|| format!("{}: `{}` is missing", what, key))?;
        process_ref(v, &ns, &format!("{}.{}", what, key))
    };
    for k in o.keys() {
        if !matches!(
            k.as_str(),
            "name"
                | "namespace"
                | "connection_type"
                | "type"
                | "source"
                | "destination"
                | "source_process_id"
                | "destination_process_id"
                | "arg_name"
        ) {
            notes
                .warnings
                .push(format!("{}: `{}` is not carried", what, k));
        }
    }
    Ok(Connection {
        name: opt_str(o, "name", what)?,
        connection_type,
        source: end("source")?,
        destination: end("destination")?,
        arg_name: get_str(o, "arg_name", what)?,
    })
}

fn write_ref(r: &ProcessRef) -> Json {
    let mut o = Map::new();
    o.insert("type_name".into(), r.type_name.clone().into());
    o.insert("name".into(), r.name.clone().into());
    if !r.namespace.is_empty() {
        o.insert("namespace".into(), r.namespace.clone().into());
    }
    if let Some(c) = &r.container_name {
        o.insert("container_name".into(), c.clone().into());
    }
    Json::Object(o)
}

fn write_connection(c: &Connection) -> Json {
    let mut o = Map::new();
    if let Some(n) = &c.name {
        o.insert("name".into(), n.clone().into());
    }
    o.insert(
        "connection_type".into(),
        match c.connection_type {
            ConnectionType::Push => "Push",
            ConnectionType::Pull => "Pull",
        }
        .into(),
    );
    o.insert("source".into(), write_ref(&c.source));
    o.insert("destination".into(), write_ref(&c.destination));
    o.insert("arg_name".into(), c.arg_name.clone().into());
    Json::Object(o)
}

fn read_ec(v: &Json, base: &str, what: &str) -> R<Ec> {
    let o = as_obj(v, what)?;
    let ns = resolve_namespace(base, opt_str(o, "namespace", what)?.as_deref());
    let mut ec = Ec {
        type_name: get_str(o, "type_name", what)?,
        name: get_str(o, "name", what)?,
        description: opt_str(o, "description", what)?,
        rate: opt_f64(o, "rate", what)?,
        auto_start: opt_bool(o, "auto_start", what)?.unwrap_or(false),
        bind: Vec::new(),
        params: rest(
            o,
            &[
                "type_name",
                "name",
                "namespace",
                "description",
                "rate",
                "auto_start",
                "bind",
            ],
        ),
        namespace: ns,
    };
    if let Some(b) = o.get("bind") {
        for (i, r) in as_arr(b, &format!("{}.bind", what))?.iter().enumerate() {
            ec.bind.push(process_ref(
                r,
                &ec.namespace,
                &format!("{}.bind[{}]", what, i),
            )?);
        }
    }
    Ok(ec)
}

fn write_ec(e: &Ec) -> Json {
    let mut o = Map::new();
    o.insert("type_name".into(), e.type_name.clone().into());
    o.insert("name".into(), e.name.clone().into());
    if !e.namespace.is_empty() {
        o.insert("namespace".into(), e.namespace.clone().into());
    }
    if let Some(d) = &e.description {
        o.insert("description".into(), d.clone().into());
    }
    if let Some(r) = e.rate {
        o.insert("rate".into(), json_number(r));
    }
    o.insert("auto_start".into(), e.auto_start.into());
    o.insert("bind".into(), e.bind.iter().map(write_ref).collect());
    extend(&mut o, &e.params);
    Json::Object(o)
}

fn is_identifier(s: &str) -> bool {
    s.contains("://")
}

fn read_vec<const N: usize>(
    v: Option<&Json>,
    names: [&str; N],
    defaults: [f64; N],
    what: &str,
) -> R<Option<[f64; N]>> {
    let Some(v) = v else { return Ok(None) };
    let o = as_obj(v, what)?;
    let mut out = defaults;
    for (i, n) in names.iter().enumerate() {
        if let Some(x) = opt_f64(o, n, what)? {
            out[i] = x;
        }
    }
    Ok(Some(out))
}

fn write_vec<const N: usize>(v: &[f64; N], names: [&str; N]) -> Json {
    Json::Object(
        names
            .iter()
            .zip(v)
            .map(|(n, x)| (n.to_string(), json_number(*x)))
            .collect(),
    )
}

fn read_transform(v: &Json, base: &str, what: &str) -> R<Transform> {
    let o = as_obj(v, what)?;
    let end = |key: &str| -> R<String> {
        let s = get_str(o, key, what)?;
        Ok(if is_identifier(&s) {
            s
        } else {
            resolve_name(base, &s)
        })
    };
    let mut writers = Vec::new();
    if let Some(w) = o.get("writers") {
        for x in as_arr(w, &format!("{}.writers", what))? {
            writers.push(
                x.as_str()
                    .ok_or_else(|| format!("{}.writers: must be names", what))?
                    .to_string(),
            );
        }
    }
    Ok(Transform {
        source: end("source")?,
        destination: end("destination")?,
        translation: read_vec(
            o.get("translation"),
            ["x", "y", "z"],
            [0.0; 3],
            &format!("{}.translation", what),
        )?,
        rotation: read_vec(
            o.get("rotation"),
            ["x", "y", "z", "w"],
            [0.0, 0.0, 0.0, 1.0],
            &format!("{}.rotation", what),
        )?,
        writers,
        params: rest(
            o,
            &[
                "source",
                "destination",
                "translation",
                "rotation",
                "writers",
            ],
        ),
    })
}

fn write_transform(t: &Transform) -> Json {
    let mut o = Map::new();
    o.insert("source".into(), t.source.clone().into());
    o.insert("destination".into(), t.destination.clone().into());
    if let Some(v) = &t.translation {
        o.insert("translation".into(), write_vec(v, ["x", "y", "z"]));
    }
    if let Some(v) = &t.rotation {
        o.insert("rotation".into(), write_vec(v, ["x", "y", "z", "w"]));
    }
    if !t.writers.is_empty() {
        o.insert(
            "writers".into(),
            t.writers.iter().map(|w| Json::from(w.clone())).collect(),
        );
    }
    extend(&mut o, &t.params);
    Json::Object(o)
}

fn read_pose_listener(v: &Json, base: &str, what: &str) -> R<PoseListener> {
    let o = as_obj(v, what)?;
    for k in o.keys() {
        if !matches!(
            k.as_str(),
            "container" | "process" | "arg" | "reference" | "initial" | "on_update"
        ) {
            return Err(format!("{}: unknown key `{}`", what, k));
        }
    }
    Ok(PoseListener {
        container: resolve_name(base, &get_str(o, "container", what)?),
        process: resolve_name(base, &get_str(o, "process", what)?),
        arg: get_str(o, "arg", what)?,
        reference: opt_str(o, "reference", what)?.map(|r| resolve_name(base, &r)),
        initial: opt_bool(o, "initial", what)?,
        on_update: opt_bool(o, "on_update", what)?,
    })
}

fn write_pose_listener(p: &PoseListener) -> Json {
    let mut o = Map::new();
    o.insert("container".into(), p.container.clone().into());
    o.insert("process".into(), p.process.clone().into());
    o.insert("arg".into(), p.arg.clone().into());
    if let Some(r) = &p.reference {
        o.insert("reference".into(), r.clone().into());
    }
    if let Some(b) = p.initial {
        o.insert("initial".into(), b.into());
    }
    if let Some(b) = p.on_update {
        o.insert("on_update".into(), b.into());
    }
    Json::Object(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(resolve_name("robot1", "chatter"), "robot1/chatter");
        assert_eq!(resolve_name("robot1", "/chatter"), "chatter");
        assert_eq!(relative_name("robot1", "robot1/chatter"), "chatter");
        assert_eq!(relative_name("robot1", "chatter"), "/chatter");
        assert_eq!(relative_name("", "a/b"), "a/b");
        assert_eq!(resolve_namespace("a", Some("b")), "a/b");
        assert_eq!(resolve_namespace("a", Some("/b")), "b");
    }

    #[test]
    fn identifiers() {
        let (class, r) =
            parse_identifier("core://core/ContainerProcess/read0::cv_video_capture_read").unwrap();
        assert_eq!(class, "ContainerProcess");
        assert_eq!(r.name, "read0");
        assert_eq!(r.type_name, "cv_video_capture_read");
        let (_, r) = parse_identifier("core://core/container_process/ns/a::t:c0").unwrap();
        assert_eq!(
            (r.namespace.as_str(), r.container_name.as_deref()),
            ("ns", Some("c0"))
        );
    }
}
