// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! A running System (spec/system.md): the snapshot of its profiles, the conf it amounts to, and
//! what its manifests add to a model (argument order, defaults, readonly, data types, ids).

use std::collections::{BTreeSet, HashMap};

use serde_json::{Map, Value as Json};

use crate::conf::{
    join, parse_identifier, split, Composite, CompositeContainer, Conf, Connection, ConnectionType,
    Container, Ec, Node, Notes, PoseListener, Process, ProcessRef, Transform, ANCHOR_TYPES,
};

/// The `format` of a snapshot file.
pub const SNAPSHOT_FORMAT: &str = "juiz_opm/system/0";

/// The lists of the HTTP API whose profiles a snapshot holds, with the key in the snapshot.
const LISTS: &[(&str, &str)] = &[
    ("connection", "connections"),
    ("composite", "composites"),
    ("transform", "transforms"),
    ("pose_listener", "pose_listeners"),
];

/// Take a snapshot of the System that the HTTP broker at `base` (`http://127.0.0.1:8000`)
/// serves: `system/profile_full` and the profiles of every connection, composite, transform and
/// pose listener (they are not in `system/profile_full`).
pub fn fetch(base: &str) -> Result<Json, String> {
    let base = base.trim_end_matches('/');
    let get = |path: &str, id: Option<&str>| -> Result<Json, String> {
        let url = format!("{}/api/{}", base, path);
        let mut req = ureq::get(&url);
        if let Some(id) = id {
            req = req.query("identifier", id);
        }
        let resp = req.call().map_err(|e| format!("GET {}: {}", url, e))?;
        resp.into_json::<Json>()
            .map_err(|e| format!("GET {}: {}", url, e))
    };
    let mut out = Map::new();
    out.insert("format".into(), SNAPSHOT_FORMAT.into());
    out.insert("system".into(), get("system/profile_full", None)?);
    for (kind, key) in LISTS {
        let ids = get(&format!("{}/list", kind), None)?;
        let mut profiles = Vec::new();
        for id in ids.as_array().into_iter().flatten() {
            let id = id
                .as_str()
                .ok_or_else(|| format!("{}/list: an identifier is not a string", kind))?;
            profiles.push(get(&format!("{}/profile_full", kind), Some(id))?);
        }
        out.insert(key.to_string(), Json::Array(profiles));
    }
    Ok(Json::Object(out))
}

/// One argument as a manifest declares it.
#[derive(Debug, Clone, PartialEq)]
pub struct ArgInfo {
    pub name: String,
    pub default: Option<Json>,
    pub datatype: Option<String>,
}

/// What the profile of one Process or ContainerProcess says beyond the conf.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessInfo {
    pub id: Option<String>,
    /// In the order of the manifest.
    pub args: Vec<ArgInfo>,
    pub readonly: bool,
    pub output_datatype: Option<String>,
}

/// What a running System adds to the model of its conf (spec/system.md §4).
#[derive(Debug, Clone, Default)]
pub struct SystemInfo {
    /// Full name → the instance.
    pub processes: HashMap<String, ProcessInfo>,
    /// Type name → the arguments of the factory (for the nodes of a composite).
    pub process_types: HashMap<String, Vec<ArgInfo>>,
    /// The ids of containers, ECs and transforms, by full name (transforms by `src|dst`).
    pub ids: HashMap<String, String>,
    /// Every type name the System can make: process, container process and composite.
    pub types: BTreeSet<String>,
    /// The type names of the composites (J10).
    pub composite_types: BTreeSet<String>,
}

fn s<'a>(v: &'a Json, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Json::as_str)
}

fn arr<'a>(v: &'a Json, key: &str) -> impl Iterator<Item = &'a Json> {
    v.get(key).and_then(Json::as_array).into_iter().flatten()
}

/// `examples.geometry.Pose2D@sha256:...` from a `schema` (juiz datatype.md §5).
fn datatype(schema: Option<&Json>) -> Option<String> {
    let schema = schema?;
    let name = s(schema, "name")?;
    Some(match s(schema, "fingerprint") {
        Some(f) => format!("{}@{}", name, f),
        None => name.to_string(),
    })
}

fn args_of(v: &Json) -> Vec<ArgInfo> {
    arr(v, "arguments")
        .filter_map(|a| {
            Some(ArgInfo {
                name: s(a, "name")?.to_string(),
                default: a.get("default").filter(|d| !d.is_null()).cloned(),
                datatype: datatype(a.get("schema")),
            })
        })
        .collect()
}

fn full(v: &Json) -> String {
    join(s(v, "namespace").unwrap_or(""), s(v, "name").unwrap_or(""))
}

/// The core store of the snapshot.
fn store(snapshot: &Json) -> Result<&Json, String> {
    snapshot
        .get("system")
        .and_then(|s| s.get("core_store"))
        .ok_or_else(|| "the snapshot has no system.core_store".to_string())
}

/// Every instance profile: processes and container processes (also those listed under their
/// containers), once each by identifier.
fn instances(cs: &Json) -> Vec<&Json> {
    let mut out: Vec<&Json> = Vec::new();
    let mut seen = BTreeSet::new();
    let all = arr(cs, "processes")
        .chain(arr(cs, "container_processes"))
        .chain(arr(cs, "containers").flat_map(|c| arr(c, "processes")));
    for p in all {
        let id = s(p, "identifier").unwrap_or("").to_string();
        if seen.insert(id) {
            out.push(p);
        }
    }
    out
}

/// The factories, with the manifest inside the wrapper when there is one.
fn factories(cs: &Json) -> Vec<&Json> {
    let mut out = Vec::new();
    for f in arr(cs, "process_factories").chain(arr(cs, "container_process_factories")) {
        let inner = ["given_process_factory", "container_process_factory"]
            .iter()
            .find_map(|k| f.get(*k));
        out.push(match inner {
            Some(i) if f.get("arguments").is_none() => i,
            _ => f,
        });
    }
    out
}

impl SystemInfo {
    pub fn from_snapshot(snapshot: &Json) -> Result<SystemInfo, String> {
        let cs = store(snapshot)?;
        let mut info = SystemInfo::default();
        for p in instances(cs) {
            info.processes.insert(
                full(p),
                ProcessInfo {
                    id: s(p, "identifier").map(str::to_string),
                    args: args_of(p),
                    readonly: p.get("readonly").and_then(Json::as_bool).unwrap_or(false),
                    output_datatype: datatype(p.get("output_schema")),
                },
            );
        }
        for f in factories(cs) {
            if let Some(t) = s(f, "type_name") {
                info.types.insert(t.to_string());
                info.process_types.insert(t.to_string(), args_of(f));
                if f.get("composition").is_some() {
                    info.composite_types.insert(t.to_string());
                }
            }
        }
        for c in arr(snapshot, "composites") {
            if let Some(t) = s(c, "type_name") {
                info.types.insert(t.to_string());
                info.composite_types.insert(t.to_string());
            }
        }
        for c in arr(cs, "containers") {
            if let Some(id) = s(c, "identifier") {
                info.ids.insert(full(c), id.to_string());
            }
        }
        if let Some(ecs) = cs.get("ecs").and_then(Json::as_object) {
            for e in ecs.values() {
                if let Some(id) = s(e, "identifier") {
                    info.ids.insert(full(e), id.to_string());
                }
            }
        }
        for t in arr(snapshot, "transforms") {
            if let (Some(id), Some(a), Some(b)) =
                (s(t, "identifier"), s(t, "source"), s(t, "destination"))
            {
                info.ids
                    .insert(format!("{}|{}", end_name(a), end_name(b)), id.to_string());
            }
        }
        Ok(info)
    }
}

/// A transform end: the full name of a container of this System, or the ID of another one.
fn end_name(id: &str) -> String {
    if id.starts_with("core://") {
        if let Some((_, r)) = parse_identifier(id) {
            return r.full_name();
        }
    }
    id.to_string()
}

fn process_ref(v: &Json) -> Option<ProcessRef> {
    if let Some(id) = v.as_str() {
        return parse_identifier(id).map(|(_, r)| r);
    }
    let ty = s(v, "type_name")?;
    // An ID object of a container process writes `type:container`.
    let (type_name, from_type) = match ty.split_once(':') {
        Some((t, c)) => (t.to_string(), Some(c.to_string())),
        None => (ty.to_string(), None),
    };
    Some(ProcessRef {
        namespace: s(v, "namespace").unwrap_or("").to_string(),
        name: s(v, "name")?.to_string(),
        type_name,
        container_name: s(v, "container_name").map(str::to_string).or(from_type),
    })
}

fn node(v: &Json) -> Result<Node, String> {
    if let Some(c) = v.get("const") {
        return Ok(Node::Const(c.clone()));
    }
    if let Some(e) = v.get("expose") {
        return Ok(match e {
            Json::String(n) => Node::Expose {
                name: n.clone(),
                default: None,
            },
            _ => Node::Expose {
                name: s(e, "name").unwrap_or("").to_string(),
                default: e.get("default").cloned(),
            },
        });
    }
    let mut args = Vec::new();
    if let Some(a) = v.get("args").and_then(Json::as_object) {
        for (k, n) in a {
            args.push((k.clone(), node(n)?));
        }
    }
    Ok(Node::Call {
        process: s(v, "process")
            .ok_or_else(|| format!("a composite node has no process: {}", v))?
            .to_string(),
        container: s(v, "container").map(str::to_string),
        args,
    })
}

fn topic_name(v: &Json) -> Option<String> {
    v.as_str()
        .or_else(|| s(v, "name"))
        .map(|n| n.trim_start_matches('/').to_string())
}

fn instance(p: &Json) -> Process {
    Process {
        type_name: s(p, "type_name").unwrap_or("").to_string(),
        name: s(p, "name").unwrap_or("").to_string(),
        namespace: s(p, "namespace").unwrap_or("").to_string(),
        use_memo: p.get("use_memo").and_then(Json::as_bool),
        publishes: arr(p, "publishes").filter_map(topic_name).collect(),
        subscribes: p
            .get("subscribes")
            .and_then(Json::as_object)
            .into_iter()
            .flatten()
            .filter_map(|(a, t)| Some((a.clone(), topic_name(t)?)))
            .collect(),
        ..Default::default()
    }
}

fn vec3(v: Option<&Json>, names: [&str; 3]) -> Option<[f64; 3]> {
    let v = v?;
    let mut out = [0.0; 3];
    for (i, n) in names.iter().enumerate() {
        out[i] = v.get(*n).and_then(Json::as_f64).unwrap_or(0.0);
    }
    Some(out)
}

fn vec4(v: Option<&Json>) -> Option<[f64; 4]> {
    let v = v?;
    let mut out = [0.0, 0.0, 0.0, 1.0];
    for (i, n) in ["x", "y", "z", "w"].iter().enumerate() {
        if let Some(x) = v.get(*n).and_then(Json::as_f64) {
            out[i] = x;
        }
    }
    Some(out)
}

/// The conf a running System amounts to (spec/system.md §3). `name` names the System (the
/// profiles do not hold the conf's `name`).
pub fn conf_of_snapshot(snapshot: &Json, name: &str) -> Result<(Conf, Notes), String> {
    let mut notes = Notes::default();
    match s(snapshot, "format") {
        Some(SNAPSHOT_FORMAT) => {}
        f => notes.warnings.push(format!(
            "the snapshot format is {:?}, not {:?}",
            f, SNAPSHOT_FORMAT
        )),
    }
    let cs = store(snapshot)?;
    let mut conf = Conf {
        name: name.to_string(),
        ..Default::default()
    };

    let all = instances(cs);
    for p in &all {
        if s(p, "container_name").is_none_or(str::is_empty) {
            conf.processes.push(instance(p));
        }
    }
    for c in arr(cs, "containers") {
        let ty = s(c, "type_name").unwrap_or("");
        let cname = s(c, "name").unwrap_or("").to_string();
        let ns = s(c, "namespace").unwrap_or("").to_string();
        let owned: Vec<Process> = all
            .iter()
            .filter(|p| {
                s(p, "container_name") == Some(cname.as_str())
                    && s(p, "namespace").unwrap_or("") == ns
            })
            .map(|p| instance(p))
            .collect();
        if ty == "composite_container" {
            let slots = c
                .get("slots")
                .and_then(Json::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(k, v)| Some((k.clone(), end_name(v.as_str()?))))
                .collect();
            conf.composite_containers.push(CompositeContainer {
                name: cname,
                namespace: ns,
                slots,
                processes: owned,
                ..Default::default()
            });
        } else {
            let rate = c
                .get("anchor")
                .and_then(|a| a.get("rate"))
                .and_then(Json::as_f64)
                .filter(|_| ANCHOR_TYPES.contains(&ty));
            conf.containers.push(Container {
                type_name: ty.to_string(),
                name: cname,
                namespace: ns,
                rate,
                processes: owned,
                ..Default::default()
            });
        }
    }

    // Composite types: the factories with a composition (the composite list names them too).
    let mut seen = BTreeSet::new();
    for f in factories(cs).into_iter().chain(arr(snapshot, "composites")) {
        let (Some(t), Some(comp)) = (s(f, "type_name"), f.get("composition")) else {
            continue;
        };
        if !seen.insert(t.to_string()) {
            continue;
        }
        let body = node(
            comp.get("body")
                .ok_or_else(|| format!("composite {} has no body", t))?,
        )?;
        conf.composites.push(Composite {
            type_name: t.to_string(),
            description: f
                .get("description")
                .and_then(|d| s(d, "text"))
                .filter(|d| !d.is_empty())
                .map(str::to_string),
            containers: comp
                .get("containers")
                .and_then(Json::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                .collect(),
            body,
            params: Map::new(),
        });
    }

    for c in arr(snapshot, "connections") {
        // juiz lists the wiring of publishes and subscribes as connections to a topic too.
        let topic_end = ["source_process_id", "destination_process_id"]
            .iter()
            .any(|k| c.get(*k).and_then(|e| s(e, "type_name")) == Some("topic"));
        if topic_end {
            continue;
        }
        let (Some(src), Some(dst)) = (
            c.get("source_process_id").and_then(process_ref),
            c.get("destination_process_id").and_then(process_ref),
        ) else {
            notes
                .warnings
                .push(format!("a connection is not understood: {}", c));
            continue;
        };
        conf.connections.push(Connection {
            name: None,
            connection_type: match s(c, "connection_type").map(str::to_ascii_lowercase) {
                Some(t) if t == "pull" => ConnectionType::Pull,
                _ => ConnectionType::Push,
            },
            source: src,
            destination: dst,
            arg_name: s(c, "arg_name").unwrap_or("").to_string(),
        });
    }

    if let Some(ecs) = cs.get("ecs").and_then(Json::as_object) {
        for e in ecs.values() {
            conf.ecs.push(Ec {
                type_name: s(e, "type_name").unwrap_or("").to_string(),
                name: s(e, "name").unwrap_or("").to_string(),
                namespace: s(e, "namespace").unwrap_or("").to_string(),
                rate: e.get("rate").and_then(Json::as_f64),
                auto_start: e.get("auto_start").and_then(Json::as_bool).unwrap_or(false),
                bind: arr(e, "targets").filter_map(process_ref).collect(),
                ..Default::default()
            });
        }
    }

    for t in arr(snapshot, "transforms") {
        let (Some(a), Some(b)) = (s(t, "source"), s(t, "destination")) else {
            continue;
        };
        let tr = t.get("transform");
        let writers = arr(t, "writers")
            .filter_map(|w| w.as_str().map(end_name))
            .collect();
        // juiz fills in the identity for what the conf leaves out.
        conf.transforms.push(Transform {
            source: end_name(a),
            destination: end_name(b),
            translation: vec3(tr.and_then(|x| x.get("translation")), ["x", "y", "z"])
                .filter(|v| *v != [0.0; 3]),
            rotation: vec4(tr.and_then(|x| x.get("rotation")))
                .filter(|v| *v != [0.0, 0.0, 0.0, 1.0]),
            writers,
            params: Map::new(),
        });
    }

    for p in arr(snapshot, "pose_listeners") {
        let (Some(c), Some(pr)) = (s(p, "container"), s(p, "process")) else {
            continue;
        };
        conf.pose_listeners.push(PoseListener {
            container: end_name(c),
            process: parse_identifier(pr)
                .map(|(_, r)| r.full_name())
                .unwrap_or_else(|| pr.to_string()),
            arg: s(p, "arg").unwrap_or("").to_string(),
            reference: s(p, "reference").map(end_name),
            // false is what juiz takes when the conf says nothing.
            initial: p.get("initial").and_then(Json::as_bool).filter(|b| *b),
            on_update: p.get("on_update").and_then(Json::as_bool).filter(|b| *b),
        });
    }

    for sub in arr(snapshot.get("system").unwrap_or(&Json::Null), "subsystems") {
        notes.warnings.push(format!(
            "subsystem {} is not carried (take its own snapshot)",
            s(sub, "identifier").or(s(sub, "name")).unwrap_or("?")
        ));
    }
    conf.canonicalize();
    Ok((conf, notes))
}

/// The namespace and name of a full name (for messages).
pub fn split_full(full: &str) -> (String, String) {
    split(full)
}

/// The part of a conf that the profiles of a running System hold (spec/system.md §3.2): without
/// params, p_apply, descriptions, use_memo, connection names, the identity transform and false
/// `initial` / `on_update` of a pose listener (juiz fills them in), in the canonical form.
pub fn profile_part(conf: &Conf) -> Conf {
    let mut c = conf.clone();
    fn process(p: &mut Process) {
        p.params.clear();
        p.p_apply.clear();
        p.description = None;
        p.use_memo = None;
    }
    c.processes.iter_mut().for_each(process);
    for k in &mut c.containers {
        k.params.clear();
        k.description = None;
        k.processes.iter_mut().for_each(process);
    }
    for k in &mut c.composite_containers {
        k.params.clear();
        k.description = None;
        k.processes.iter_mut().for_each(process);
    }
    for k in &mut c.composites {
        k.params.clear();
        k.description = None;
    }
    for k in &mut c.connections {
        k.name = None;
    }
    for k in &mut c.ecs {
        k.params.clear();
        k.description = None;
    }
    for k in &mut c.transforms {
        k.params.clear();
        k.translation = k.translation.filter(|v| *v != [0.0; 3]);
        k.rotation = k.rotation.filter(|v| *v != [0.0, 0.0, 0.0, 1.0]);
    }
    for k in &mut c.pose_listeners {
        k.initial = k.initial.filter(|b| *b);
        k.on_update = k.on_update.filter(|b| *b);
    }
    c.subsystems.clear();
    c.canonicalize();
    c
}
