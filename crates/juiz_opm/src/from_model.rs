// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! model → conf (spec/conf.md §5). Reads the shapes of spec/metamodel.md §9.5 back.

use opm_model::{Link, LinkKind, Model, Modifier, Thing, JUIZ_NS};
use opm_opl::names::parse_number;
use serde_json::{Map, Value as Json};

use crate::conf::{
    join, Composite, CompositeContainer, Conf, Connection, ConnectionType, Container, Ec, Node,
    PoseListener, Process, ProcessRef, Subsystem, Transform, ANCHOR_TYPES,
};

/// The conf of a model. Errors are shapes that do not map to a conf.
pub fn to_conf(model: &Model) -> Result<Conf, Vec<String>> {
    let mut r = Reader {
        m: model,
        errors: Vec::new(),
    };
    let conf = r.conf();
    if r.errors.is_empty() {
        Ok(conf)
    } else {
        Err(r.errors)
    }
}

fn ann<'a>(t: &'a Thing, key: &str) -> Option<&'a str> {
    t.extensions.attribute(JUIZ_NS, key)
}

fn link_ann<'a>(l: &'a Link, key: &str) -> Option<&'a str> {
    l.extensions.attribute(JUIZ_NS, key)
}

fn role(t: &Thing) -> &str {
    ann(t, "role").unwrap_or("")
}

fn json_of(ext: &opm_model::Extensions, key: &str) -> Result<Option<Json>, String> {
    match ext.element(JUIZ_NS, key) {
        None => Ok(None),
        Some(e) => serde_json::from_str(&e.text())
            .map(Some)
            .map_err(|err| format!("juiz:{} is not JSON: {}", key, err)),
    }
}

fn vec_of<const N: usize>(
    ext: &opm_model::Extensions,
    key: &str,
    names: [&str; N],
    defaults: [f64; N],
) -> Option<[f64; N]> {
    let e = ext.element(JUIZ_NS, key)?;
    let mut out = defaults;
    for (i, n) in names.iter().enumerate() {
        if let Some(x) = e.attribute(None, n).and_then(|v| parse_number(v.trim())) {
            out[i] = x;
        }
    }
    Some(out)
}

struct Reader<'a> {
    m: &'a Model,
    errors: Vec<String>,
}

impl<'a> Reader<'a> {
    fn thing(&self, id: &str) -> Option<&'a Thing> {
        self.m.thing(id)
    }

    fn links_from(&self, id: &str, kind: LinkKind) -> impl Iterator<Item = &'a Link> + 'a {
        let id = id.to_string();
        self.m
            .links
            .iter()
            .filter(move |l| l.kind == kind && l.source == id)
    }

    fn links_to(&self, id: &str, kind: LinkKind) -> impl Iterator<Item = &'a Link> + 'a {
        let id = id.to_string();
        self.m
            .links
            .iter()
            .filter(move |l| l.kind == kind && l.target == id)
    }

    fn name_of(&mut self, t: &Thing) -> String {
        match ann(t, "name") {
            Some(n) => n.to_string(),
            None => {
                self.errors.push(format!("{:?} has no juiz:name", t.name));
                t.name.clone()
            }
        }
    }

    fn type_of(&mut self, t: &Thing) -> String {
        match ann(t, "type") {
            Some(n) => n.to_string(),
            None => {
                self.errors.push(format!("{:?} has no juiz:type", t.name));
                String::new()
            }
        }
    }

    fn full_name(&mut self, t: &Thing) -> String {
        let n = self.name_of(t);
        join(ann(t, "namespace").unwrap_or(""), &n)
    }

    fn params(&mut self, t: &Thing) -> Map<String, Json> {
        match json_of(&t.extensions, "params") {
            Ok(Some(Json::Object(o))) => o,
            Ok(Some(v)) => {
                self.errors.push(format!(
                    "{:?}: juiz:params must be an object: {}",
                    t.name, v
                ));
                Map::new()
            }
            Ok(None) => Map::new(),
            Err(e) => {
                self.errors.push(format!("{:?}: {}", t.name, e));
                Map::new()
            }
        }
    }

    /// The root system: a Thing of role system that no other system aggregates.
    fn root(&mut self) -> Option<&'a Thing> {
        let systems: Vec<&Thing> = self
            .m
            .things
            .iter()
            .filter(|t| role(t) == "system")
            .collect();
        let roots: Vec<&Thing> = systems
            .iter()
            .copied()
            .filter(|s| {
                !self
                    .links_to(&s.id, LinkKind::Aggregation)
                    .any(|l| self.thing(&l.source).is_some_and(|t| role(t) == "system"))
            })
            .collect();
        match roots.as_slice() {
            [r] => Some(*r),
            [] => {
                self.errors.push("the model has no System".to_string());
                None
            }
            _ => {
                self.errors.push(format!(
                    "the model has {} root Systems; a conf holds one",
                    roots.len()
                ));
                None
            }
        }
    }

    fn conf(&mut self) -> Conf {
        let Some(sys) = self.root() else {
            return Conf::default();
        };
        let mut conf = Conf {
            name: self.name_of(sys),
            ..Default::default()
        };
        let members: Vec<&Thing> = self
            .m
            .links
            .iter()
            .filter(|l| {
                l.source == sys.id && matches!(l.kind, LinkKind::Aggregation | LinkKind::Exhibition)
            })
            .filter_map(|l| self.thing(&l.target))
            .collect();
        for t in &members {
            match role(t) {
                "system" => {
                    let s = Subsystem {
                        type_name: self.type_of(t),
                        name: self.name_of(t),
                        params: self.params(t),
                    };
                    conf.subsystems.push(s);
                }
                "process" => {
                    let p = self.process(t);
                    conf.processes.push(p);
                }
                "container" => {
                    let c = self.container(t);
                    conf.containers.push(c);
                }
                "composite_container" => {
                    let c = self.composite_container(t);
                    conf.composite_containers.push(c);
                }
                "ec" => {
                    let e = self.ec(t);
                    conf.ecs.push(e);
                }
                "transform" => {
                    if let Some(tr) = self.transform(t) {
                        conf.transforms.push(tr);
                    }
                }
                // Read from the processes; the owners are read from their containers.
                "topic" | "container_process" => {}
                r => self.errors.push(format!(
                    "{:?} (role {:?}) is not a member a System holds",
                    t.name, r
                )),
            }
        }
        for t in &self.m.things {
            if role(t) == "composite" {
                if let Some(c) = self.composite(t) {
                    conf.composites.push(c);
                }
            }
        }
        conf.connections = self.connections();
        conf.pose_listeners = self.pose_listeners();
        conf.canonicalize();
        conf
    }

    fn reference(&mut self, t: &Thing) -> ProcessRef {
        let container_name = if role(t) == "container_process" {
            self.owner(t).map(|o| self.name_of(o))
        } else {
            None
        };
        ProcessRef {
            namespace: ann(t, "namespace").unwrap_or("").to_string(),
            name: self.name_of(t),
            type_name: self.type_of(t),
            container_name,
        }
    }

    fn owner(&mut self, t: &Thing) -> Option<&'a Thing> {
        let owners: Vec<&Thing> = self
            .m
            .links
            .iter()
            .filter(|l| {
                l.source == t.id && matches!(l.kind, LinkKind::Effect | LinkKind::Instrument)
                    || l.target == t.id && l.kind == LinkKind::Instrument && l.modifier.is_none()
            })
            .filter_map(|l| {
                let other = if l.source == t.id {
                    &l.target
                } else {
                    &l.source
                };
                self.thing(other)
            })
            .filter(|o| matches!(role(o), "container" | "composite_container"))
            .collect();
        match owners.as_slice() {
            [o] => Some(*o),
            _ => {
                self.errors.push(format!(
                    "container process {:?} needs exactly one owner container (J5), has {}",
                    t.name,
                    owners.len()
                ));
                None
            }
        }
    }

    fn process(&mut self, t: &Thing) -> Process {
        let mut p = Process {
            type_name: self.type_of(t),
            name: self.name_of(t),
            namespace: ann(t, "namespace").unwrap_or("").to_string(),
            description: t.description.clone(),
            use_memo: ann(t, "use_memo").map(|v| v == "true"),
            params: self.params(t),
            ..Default::default()
        };
        for l in self.links_from(&t.id, LinkKind::Result) {
            if let Some(o) = self.thing(&l.target) {
                if role(o) == "topic" {
                    p.publishes.push(self.full_name(o));
                }
            }
        }
        let mut ins: Vec<&Link> = self.links_to(&t.id, LinkKind::Instrument).collect();
        ins.sort_by_key(|l| link_ann(l, "arg_index").and_then(|v| v.parse::<usize>().ok()));
        for l in ins {
            let Some(arg) = link_ann(l, "arg") else {
                continue;
            };
            let src = self
                .thing(&l.source)
                .or_else(|| self.m.state(&l.source).map(|(o, _)| o));
            if let Some(s) = src {
                if role(s) == "topic" && l.modifier == Some(Modifier::Event) {
                    p.subscribes.push((arg.to_string(), self.full_name(s)));
                }
            }
            match json_of(&l.extensions, "applied") {
                Ok(Some(v)) => p.p_apply.push((arg.to_string(), v)),
                Ok(None) => {}
                Err(e) => self.errors.push(format!("{:?}: {}", t.name, e)),
            }
        }
        p
    }

    fn owned_processes(&mut self, owner: &Thing) -> Vec<Process> {
        let ids: Vec<String> = self
            .m
            .links
            .iter()
            .filter(|l| {
                l.target == owner.id && matches!(l.kind, LinkKind::Effect)
                    || l.source == owner.id
                        && l.kind == LinkKind::Instrument
                        && l.modifier.is_none()
            })
            .map(|l| {
                if l.target == owner.id {
                    l.source.clone()
                } else {
                    l.target.clone()
                }
            })
            .collect();
        let mut out = Vec::new();
        for id in ids {
            if let Some(t) = self.thing(&id) {
                if role(t) == "container_process" {
                    let mut p = self.process(t);
                    let ns = ann(owner, "namespace").unwrap_or("");
                    if p.namespace != ns {
                        self.errors.push(format!(
                            "{:?} is not in the namespace of its container",
                            t.name
                        ));
                    }
                    p.namespace = ns.to_string();
                    out.push(p);
                }
            }
        }
        out
    }

    fn container(&mut self, t: &Thing) -> Container {
        let type_name = self.type_of(t);
        let rate = ann(t, "rate").and_then(parse_number);
        let mut params = self.params(t);
        if let Some(r) = rate.filter(|_| !ANCHOR_TYPES.contains(&type_name.as_str())) {
            // juiz:rate on another container is the conf key `rate`.
            params.insert("rate".into(), Json::from(r));
        }
        Container {
            rate: rate.filter(|_| ANCHOR_TYPES.contains(&type_name.as_str())),
            type_name,
            name: self.name_of(t),
            namespace: ann(t, "namespace").unwrap_or("").to_string(),
            description: t.description.clone(),
            params,
            processes: self.owned_processes(t),
        }
    }

    fn composite_container(&mut self, t: &Thing) -> CompositeContainer {
        let mut slots = Vec::new();
        for l in self.links_from(&t.id, LinkKind::Aggregation) {
            let Some(slot) = link_ann(l, "slot_name") else {
                continue;
            };
            if let Some(c) = self.thing(&l.target) {
                slots.push((slot.to_string(), self.full_name(c)));
            }
        }
        CompositeContainer {
            name: self.name_of(t),
            namespace: ann(t, "namespace").unwrap_or("").to_string(),
            description: t.description.clone(),
            slots,
            params: self.params(t),
            processes: self.owned_processes(t),
        }
    }

    fn ec(&mut self, t: &Thing) -> Ec {
        let started = t.states().iter().find(|s| s.name == "started");
        let auto_start = started.is_some_and(|s| s.initial);
        // The cycle: the process the `started` state conditions.
        let cycle = started.and_then(|s| {
            self.m
                .links
                .iter()
                .find(|l| {
                    l.kind == LinkKind::Instrument
                        && l.source == s.id
                        && l.modifier == Some(Modifier::Condition)
                })
                .map(|l| l.target.clone())
        });
        let mut bind = Vec::new();
        if let Some(cycle) = &cycle {
            let mut inv: Vec<&Link> = self
                .links_from(cycle, LinkKind::Invocation)
                .filter(|l| &l.target != cycle)
                .collect();
            inv.sort_by_key(|l| link_ann(l, "order").and_then(|v| v.parse::<usize>().ok()));
            for l in inv {
                if let Some(p) = self.thing(&l.target) {
                    bind.push(self.reference(p));
                }
            }
        } else {
            self.errors
                .push(format!("EC {:?} has no cycle (J7)", t.name));
        }
        Ec {
            type_name: self.type_of(t),
            name: self.name_of(t),
            namespace: ann(t, "namespace").unwrap_or("").to_string(),
            description: t.description.clone(),
            rate: ann(t, "rate").and_then(parse_number),
            auto_start,
            bind,
            params: self.params(t),
        }
    }

    fn container_ref(&mut self, t: &Thing) -> String {
        match ann(t, "id") {
            Some(id) if t.affiliation == opm_model::Affiliation::Environmental => id.to_string(),
            _ => self.full_name(t),
        }
    }

    fn transform(&mut self, t: &Thing) -> Option<Transform> {
        let mut source = None;
        let mut destination = None;
        for l in self.links_from(&t.id, LinkKind::Tagged) {
            let end = link_ann(l, "end").or(l.tag.as_deref());
            let Some(c) = self.thing(&l.target) else {
                continue;
            };
            match end {
                Some("source") => source = Some(self.container_ref(c)),
                Some("destination") => destination = Some(self.container_ref(c)),
                _ => {}
            }
        }
        let (Some(source), Some(destination)) = (source, destination) else {
            self.errors.push(format!(
                "transform {:?} needs a source and a destination (J11)",
                t.name
            ));
            return None;
        };
        Some(Transform {
            source,
            destination,
            translation: vec_of(&t.extensions, "translation", ["x", "y", "z"], [0.0; 3]),
            rotation: vec_of(
                &t.extensions,
                "rotation",
                ["x", "y", "z", "w"],
                [0.0, 0.0, 0.0, 1.0],
            ),
            writers: ann(t, "writers")
                .map(|w| w.split_whitespace().map(str::to_string).collect())
                .unwrap_or_default(),
            params: self.params(t),
        })
    }

    /// The process that yields a value.
    fn producer(&self, value: &str) -> Option<&'a Thing> {
        self.links_to(value, LinkKind::Result)
            .find_map(|l| self.thing(&l.source))
    }

    fn connections(&mut self) -> Vec<Connection> {
        let mut out = Vec::new();
        for l in &self.m.links {
            if l.kind != LinkKind::Instrument {
                continue;
            }
            let Some(kind) = link_ann(l, "connection") else {
                continue;
            };
            let connection_type = match kind {
                "push" => ConnectionType::Push,
                "pull" => ConnectionType::Pull,
                k => {
                    self.errors.push(format!("unknown juiz:connection {:?}", k));
                    continue;
                }
            };
            let (Some(src), Some(dst)) = (self.producer(&l.source), self.thing(&l.target)) else {
                self.errors.push(format!(
                    "connection {} needs a value yielded by a process (J3)",
                    l.id
                ));
                continue;
            };
            let Some(arg) = link_ann(l, "arg") else {
                self.errors
                    .push(format!("connection {} has no juiz:arg", l.id));
                continue;
            };
            out.push(Connection {
                name: link_ann(l, "connection_name").map(str::to_string),
                connection_type,
                source: self.reference(src),
                destination: self.reference(dst),
                arg_name: arg.to_string(),
            });
        }
        out
    }

    fn pose_listeners(&mut self) -> Vec<PoseListener> {
        let mut out = Vec::new();
        for l in &self.m.links {
            if l.kind != LinkKind::Instrument {
                continue;
            }
            let Some(pose) = self.thing(&l.source) else {
                continue;
            };
            if role(pose) != "pose" {
                continue;
            }
            let Some(container) = self
                .links_to(&pose.id, LinkKind::Exhibition)
                .find_map(|x| self.thing(&x.source))
            else {
                self.errors
                    .push(format!("pose {:?} has no container (J12)", pose.name));
                continue;
            };
            let Some(process) = self.thing(&l.target) else {
                continue;
            };
            out.push(PoseListener {
                container: self.full_name(container),
                process: self.full_name(process),
                arg: link_ann(l, "arg").unwrap_or("").to_string(),
                reference: link_ann(l, "reference").map(str::to_string),
                initial: link_ann(l, "initial").map(|v| v == "true"),
                on_update: link_ann(l, "on_update").map(|v| v == "true"),
            });
        }
        out
    }

    fn composite(&mut self, t: &Thing) -> Option<Composite> {
        let Some(zoom) = self.m.in_zoom_of(&t.id) else {
            self.errors
                .push(format!("composite {:?} has no InZoom", t.name));
            return None;
        };
        let nodes: Vec<&String> = zoom.steps.iter().flatten().collect();
        // The root node yields a value(output); the others an intermediate value.
        let root = nodes.iter().find(|n| {
            self.links_from(n, LinkKind::Result).any(|l| {
                self.thing(&l.target)
                    .is_some_and(|v| ann(v, "value_kind") == Some("output"))
            })
        });
        let Some(root) = root else {
            self.errors.push(format!(
                "composite {:?} has no node that yields its output (J1)",
                t.name
            ));
            return None;
        };
        let mut containers = Vec::new();
        for p in &zoom.parts {
            if let Some(s) = self.thing(p) {
                if role(s) == "slot" {
                    let slot = (self.name_of(s), self.type_of(s));
                    containers.push(slot);
                }
            }
        }
        let body = self.node(root, 0)?;
        Some(Composite {
            type_name: self.type_of(t),
            description: t.description.clone(),
            containers,
            body,
            params: self.params(t),
        })
    }

    fn node(&mut self, id: &str, depth: usize) -> Option<Node> {
        if depth > 64 {
            self.errors
                .push("the expression tree of a composite is too deep".to_string());
            return None;
        }
        let t = self.thing(id)?;
        let mut ins: Vec<&Link> = self.links_to(id, LinkKind::Instrument).collect();
        ins.sort_by_key(|l| link_ann(l, "arg_index").and_then(|v| v.parse::<usize>().ok()));
        let mut args = Vec::new();
        for l in ins {
            let arg = link_ann(l, "arg").unwrap_or("").to_string();
            let Some(v) = self.thing(&l.source) else {
                continue;
            };
            let n = match ann(v, "value_kind") {
                Some("intermediate") => {
                    let p = self.producer(&v.id)?;
                    self.node(&p.id.clone(), depth + 1)?
                }
                Some("const") => match json_of(&v.extensions, "value") {
                    Ok(Some(x)) => Node::Const(x),
                    _ => {
                        self.errors
                            .push(format!("const {:?} has no juiz:value", v.name));
                        continue;
                    }
                },
                Some("input") => Node::Expose {
                    name: ann(v, "name").unwrap_or(&arg).to_string(),
                    default: json_of(&v.extensions, "default").ok().flatten(),
                },
                _ => {
                    self.errors.push(format!(
                        "{:?} feeds a composite node but is not a value",
                        v.name
                    ));
                    continue;
                }
            };
            args.push((arg, n));
        }
        let container = self
            .m
            .links
            .iter()
            .filter(|l| {
                l.source == id && l.kind == LinkKind::Effect
                    || l.target == id && l.kind == LinkKind::Instrument
            })
            .filter_map(|l| {
                let other = if l.source == id { &l.target } else { &l.source };
                self.thing(other)
            })
            .find(|s| role(s) == "slot")
            .and_then(|s| ann(s, "name").map(str::to_string));
        Some(Node::Call {
            process: self.type_of(t),
            container,
            args,
        })
    }
}
