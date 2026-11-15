// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! conf → model (spec/conf.md §4): the shapes of spec/metamodel.md §9.5.

use std::collections::{BTreeMap, HashMap, HashSet};

use opm_model::{
    Affiliation, ExtElement, InZoom, Link, LinkKind, Model, Modifier, State, Thing, ThingKind,
    XmlAttribute, JUIZ_NS,
};
use opm_opl::names::number;
use serde_json::{Map, Value as Json};

use crate::conf::{
    join, parse_identifier, split, Composite, Conf, ConnectionType, Node, Process, ProcessRef,
};

/// Build the model of a conf. The ids are those that juiz-OPL gives (the model is written as
/// OPL and read back), so the same conf always gives the same model. Errors are references that
/// do not resolve.
pub fn to_model(conf: &Conf) -> Result<Model, Vec<String>> {
    let mut b = Builder::new(conf);
    b.build();
    if !b.errors.is_empty() {
        return Err(b.errors);
    }
    let text = opm_opl::write(&b.model, opm_opl::Options::default()).text;
    let parsed = opm_opl::parse(&text);
    if !parsed.is_ok() {
        return Err(parsed
            .diagnostics
            .iter()
            .map(|d| format!("internal: the OPL of the model does not read back: {}", d))
            .collect());
    }
    Ok(parsed.model)
}

/// An argument of a Process: where its value comes from.
#[derive(Default)]
struct Arg {
    /// (link id) of the instruments that feed it, in order.
    links: Vec<String>,
}

struct Builder<'a> {
    conf: &'a Conf,
    model: Model,
    next: usize,
    used: HashSet<String>,
    errors: Vec<String>,
    system: String,
    /// full name → thing id, by kind.
    processes: HashMap<String, String>,
    containers: HashMap<String, String>,
    topics: HashMap<String, String>,
    /// process id → its output value id.
    outputs: HashMap<String, String>,
    /// process id → arguments in the order they were met.
    args: HashMap<String, Vec<(String, Arg)>>,
    poses: HashMap<String, String>,
    invocations: HashSet<(String, String)>,
}

fn attr(t: &mut opm_model::Extensions, key: &str, value: impl Into<String>) {
    t.set_attribute(JUIZ_NS, key, value);
}

fn json_elem(t: &mut opm_model::Extensions, key: &str, v: &Json) {
    t.elements.push(ExtElement::with_text(
        JUIZ_NS,
        key,
        serde_json::to_string(v).expect("JSON"),
    ));
}

fn vec_elem(t: &mut opm_model::Extensions, key: &str, names: &[&str], v: &[f64]) {
    let mut e = ExtElement::new(JUIZ_NS, key);
    for (n, x) in names.iter().zip(v) {
        e.attributes.push(XmlAttribute {
            namespace: None,
            name: n.to_string(),
            value: number(*x),
        });
    }
    t.elements.push(e);
}

impl<'a> Builder<'a> {
    fn new(conf: &'a Conf) -> Self {
        let mut model = Model::new("juiz");
        model.name = Some(conf.name.clone());
        Builder {
            conf,
            model,
            next: 0,
            used: HashSet::new(),
            errors: Vec::new(),
            system: String::new(),
            processes: HashMap::new(),
            containers: HashMap::new(),
            topics: HashMap::new(),
            outputs: HashMap::new(),
            args: HashMap::new(),
            poses: HashMap::new(),
            invocations: HashSet::new(),
        }
    }

    fn id(&mut self) -> String {
        self.next += 1;
        format!("e{}", self.next)
    }

    /// A name not used yet: `base`, else `base (what)`, else `base (what) 2`...
    fn name(&mut self, base: &str, what: &str) -> String {
        let mut n = base.to_string();
        if self.used.contains(&n) {
            n = format!("{} ({})", base, what);
            let mut k = 2;
            while self.used.contains(&n) {
                n = format!("{} ({}) {}", base, what, k);
                k += 1;
            }
        }
        self.used.insert(n.clone());
        n
    }

    fn thing(&mut self, mut t: Thing) -> String {
        let id = self.id();
        t.id = id.clone();
        self.model.things.push(t);
        id
    }

    fn object(&mut self, base: &str, role: &str) -> (String, &mut Thing) {
        let name = self.name(base, role);
        let mut t = Thing::object("", name);
        attr(&mut t.extensions, "role", role);
        let id = self.thing(t);
        (id.clone(), self.model.things.last_mut().unwrap())
    }

    fn process(&mut self, base: &str, role: &str) -> (String, &mut Thing) {
        let name = self.name(base, role);
        let mut t = Thing::process("", name);
        attr(&mut t.extensions, "role", role);
        let id = self.thing(t);
        (id.clone(), self.model.things.last_mut().unwrap())
    }

    fn thing_mut(&mut self, id: &str) -> &mut Thing {
        self.model.things.iter_mut().find(|t| t.id == id).unwrap()
    }

    fn link(&mut self, kind: LinkKind, source: &str, target: &str) -> &mut Link {
        let id = self.id();
        self.model.links.push(Link::new(id, kind, source, target));
        self.model.links.last_mut().unwrap()
    }

    fn link_by_id(&mut self, id: &str) -> &mut Link {
        self.model.links.iter_mut().find(|l| l.id == id).unwrap()
    }

    fn build(&mut self) {
        let conf = self.conf;
        let (sys, t) = self.object(&conf.name, "system");
        attr(&mut t.extensions, "name", conf.name.clone());
        self.system = sys.clone();

        for s in &conf.subsystems {
            let (id, t) = self.object(&s.name, "system");
            attr(&mut t.extensions, "name", s.name.clone());
            attr(&mut t.extensions, "type", s.type_name.clone());
            params(t, &s.params);
            self.link(LinkKind::Aggregation, &sys, &id);
        }

        for c in &conf.composites {
            self.composite(c);
        }

        for p in &conf.processes {
            let id = self.instance(p, "process");
            self.link(LinkKind::Exhibition, &sys, &id);
        }

        for c in &conf.containers {
            let full = join(&c.namespace, &c.name);
            let (id, t) = self.object(&full, "container");
            attr(&mut t.extensions, "name", c.name.clone());
            if !c.namespace.is_empty() {
                attr(&mut t.extensions, "namespace", c.namespace.clone());
            }
            attr(&mut t.extensions, "type", c.type_name.clone());
            if let Some(r) = c.rate {
                attr(&mut t.extensions, "rate", number(r));
            }
            params(t, &c.params);
            t.description = c.description.clone();
            self.containers.insert(full, id.clone());
            self.link(LinkKind::Aggregation, &sys, &id);
            for p in &c.processes {
                let pid = self.instance(p, "container_process");
                self.link(LinkKind::Effect, &pid, &id);
                self.link(LinkKind::Exhibition, &sys, &pid);
            }
        }

        for c in &conf.composite_containers {
            let full = join(&c.namespace, &c.name);
            let (id, t) = self.object(&full, "composite_container");
            attr(&mut t.extensions, "name", c.name.clone());
            if !c.namespace.is_empty() {
                attr(&mut t.extensions, "namespace", c.namespace.clone());
            }
            params(t, &c.params);
            t.description = c.description.clone();
            self.containers.insert(full.clone(), id.clone());
            self.link(LinkKind::Aggregation, &sys, &id);
            for (slot, target) in &c.slots {
                match self.containers.get(target).cloned() {
                    Some(tid) => {
                        let l = self.link(LinkKind::Aggregation, &id, &tid);
                        attr(&mut l.extensions, "slot_name", slot.clone());
                    }
                    None => self.errors.push(format!(
                        "composite_containers {}: slot {} names no container {:?}",
                        full, slot, target
                    )),
                }
            }
            for p in &c.processes {
                let pid = self.instance(p, "container_process");
                self.link(LinkKind::Effect, &pid, &id);
                self.link(LinkKind::Exhibition, &sys, &pid);
            }
        }

        // Topics, connections and pose listeners feed the arguments; p_apply and the
        // arguments without a source are settled at the end.
        self.topics_and_subscribes();
        for c in &conf.connections {
            let (Some(src), Some(dst)) = (
                self.resolve(&c.source, "connection source"),
                self.resolve(&c.destination, "connection destination"),
            ) else {
                continue;
            };
            let out = self.outputs[&src].clone();
            let l = self.link(LinkKind::Instrument, &out, &dst);
            attr(&mut l.extensions, "connection", c.connection_type.word());
            if let Some(n) = &c.name {
                attr(&mut l.extensions, "connection_name", n.clone());
            }
            let lid = l.id.clone();
            self.add_arg(&dst, &c.arg_name, lid);
            if c.connection_type == ConnectionType::Push
                && self.invocations.insert((src.clone(), dst.clone()))
            {
                self.link(LinkKind::Invocation, &src, &dst);
            }
        }

        for e in &conf.ecs {
            self.ec(e);
        }
        for t in &conf.transforms {
            self.transform(t);
        }
        for p in &conf.pose_listeners {
            self.pose_listener(p);
        }
        self.settle_args();
    }

    /// A Process or ContainerProcess and its output value.
    fn instance(&mut self, p: &Process, role: &str) -> String {
        let full = join(&p.namespace, &p.name);
        let (id, t) = self.process(&full, role);
        attr(&mut t.extensions, "name", p.name.clone());
        if !p.namespace.is_empty() {
            attr(&mut t.extensions, "namespace", p.namespace.clone());
        }
        attr(&mut t.extensions, "type", p.type_name.clone());
        if let Some(b) = p.use_memo {
            attr(&mut t.extensions, "use_memo", b.to_string());
        }
        params(t, &p.params);
        t.description = p.description.clone();
        let name = t.name.clone();
        if self.processes.insert(full.clone(), id.clone()).is_some() {
            self.errors
                .push(format!("two processes are named {:?}", full));
        }
        let (out, t) = self.object(&format!("{} out", name), "value");
        attr(&mut t.extensions, "value_kind", "output");
        self.link(LinkKind::Result, &id, &out);
        self.outputs.insert(id.clone(), out);
        self.args.insert(id.clone(), Vec::new());
        id
    }

    fn add_arg(&mut self, process: &str, arg: &str, link: String) {
        let args = self.args.get_mut(process).expect("a process");
        match args.iter_mut().find(|(a, _)| a == arg) {
            Some((_, a)) => a.links.push(link),
            None => args.push((arg.to_string(), Arg { links: vec![link] })),
        }
    }

    /// Every Process and ContainerProcess of the conf with its process id.
    fn all_instances(&self) -> Vec<(&'a Process, String)> {
        let conf = self.conf;
        let mut out = Vec::new();
        for p in &conf.processes {
            out.push((p, self.processes[&join(&p.namespace, &p.name)].clone()));
        }
        for c in &conf.containers {
            for p in &c.processes {
                out.push((p, self.processes[&join(&p.namespace, &p.name)].clone()));
            }
        }
        for c in &conf.composite_containers {
            for p in &c.processes {
                out.push((p, self.processes[&join(&p.namespace, &p.name)].clone()));
            }
        }
        out
    }

    fn topic(&mut self, full: &str) -> String {
        if let Some(id) = self.topics.get(full) {
            return id.clone();
        }
        let (ns, name) = split(full);
        let (id, t) = self.object(full, "topic");
        attr(&mut t.extensions, "name", name);
        if !ns.is_empty() {
            attr(&mut t.extensions, "namespace", ns);
        }
        self.topics.insert(full.to_string(), id.clone());
        let sys = self.system.clone();
        self.link(LinkKind::Aggregation, &sys, &id);
        id
    }

    fn topics_and_subscribes(&mut self) {
        for (p, pid) in self.all_instances() {
            for t in &p.publishes {
                let tid = self.topic(t);
                self.link(LinkKind::Result, &pid, &tid);
            }
            for (arg, t) in &p.subscribes {
                let tid = self.topic(t);
                let l = self.link(LinkKind::Instrument, &tid, &pid);
                l.modifier = Some(Modifier::Event);
                let lid = l.id.clone();
                self.add_arg(&pid, arg, lid);
            }
        }
    }

    /// Give every argument its index, its value(input) when nothing feeds it, and the p_apply
    /// values.
    fn settle_args(&mut self) {
        for (p, pid) in self.all_instances() {
            for (arg, _) in &p.p_apply {
                let known = self.args[&pid].iter().any(|(a, _)| a == arg);
                if !known {
                    let pname = self.thing_mut(&pid).name.clone();
                    let (vid, t) = self.object(&format!("{} {}", pname, arg), "value");
                    attr(&mut t.extensions, "value_kind", "input");
                    let lid = self.link(LinkKind::Instrument, &vid, &pid).id.clone();
                    self.add_arg(&pid, arg, lid);
                }
            }
            let args = std::mem::take(self.args.get_mut(&pid).unwrap());
            for (k, (arg, a)) in args.iter().enumerate() {
                let applied = p.p_apply.iter().find(|(x, _)| x == arg).map(|(_, v)| v);
                for (i, lid) in a.links.iter().enumerate() {
                    let l = self.link_by_id(lid);
                    attr(&mut l.extensions, "arg", arg.clone());
                    attr(&mut l.extensions, "arg_index", k.to_string());
                    if i == 0 {
                        if let Some(v) = applied {
                            json_elem(&mut l.extensions, "applied", v);
                        }
                    }
                }
            }
            self.args.insert(pid, args);
        }
    }

    /// The process a reference names, or an error.
    fn resolve(&mut self, r: &ProcessRef, what: &str) -> Option<String> {
        let full = r.full_name();
        match self.processes.get(&full) {
            Some(id) => {
                let ty = self
                    .model
                    .thing(id)
                    .and_then(|t| t.extensions.attribute(JUIZ_NS, "type"))
                    .unwrap_or_default();
                if !r.type_name.is_empty() && ty != r.type_name {
                    self.errors.push(format!(
                        "{} {:?} is of type {:?}, not {:?}",
                        what, full, ty, r.type_name
                    ));
                }
                Some(id.clone())
            }
            None => {
                self.errors
                    .push(format!("{} {:?} names no process of the conf", what, full));
                None
            }
        }
    }

    fn ec(&mut self, e: &crate::conf::Ec) {
        let full = join(&e.namespace, &e.name);
        let (id, t) = self.object(&full, "ec");
        attr(&mut t.extensions, "name", e.name.clone());
        if !e.namespace.is_empty() {
            attr(&mut t.extensions, "namespace", e.namespace.clone());
        }
        attr(&mut t.extensions, "type", e.type_name.clone());
        if let Some(r) = e.rate {
            attr(&mut t.extensions, "rate", number(r));
        }
        params(t, &e.params);
        t.description = e.description.clone();
        let name = t.name.clone();
        let mut stopped = State::new(format!("{}-stopped", id), "stopped");
        let mut started = State::new(format!("{}-started", id), "started");
        if e.auto_start {
            started.initial = true;
        } else {
            stopped.initial = true;
        }
        let started_id = started.id.clone();
        let stopped_id = stopped.id.clone();
        t.kind = ThingKind::Object {
            states: vec![stopped, started],
        };
        let sys = self.system.clone();
        self.link(LinkKind::Aggregation, &sys, &id);

        let (cycle, _) = self.process(&format!("{} cycle", name), "ec_cycle");
        self.link(LinkKind::Instrument, &started_id, &cycle)
            .modifier = Some(Modifier::Condition);
        for (k, b) in e.bind.iter().enumerate() {
            if let Some(pid) = self.resolve(b, &format!("ecs {} bind", full)) {
                let l = self.link(LinkKind::Invocation, &cycle, &pid);
                attr(&mut l.extensions, "order", k.to_string());
            }
        }
        match e.type_name.as_str() {
            "main_loop_ec" => {
                self.link(LinkKind::Invocation, &cycle, &cycle);
            }
            "one_shot_ec" => {
                let l = self.link(LinkKind::Effect, &cycle, &id);
                l.from_state = Some(started_id);
                l.to_state = Some(stopped_id);
            }
            _ => {}
        }
    }

    /// The container at one end of a Transform: one of this System, or one of another System
    /// named by its ID (an environmental Object).
    fn transform_end(&mut self, end: &str) -> Option<String> {
        if let Some(id) = self.containers.get(end) {
            return Some(id.clone());
        }
        if end.contains("://") {
            let (ns, name, ty) = match parse_identifier(end) {
                Some((_, r)) => (r.namespace, r.name, r.type_name),
                None => (String::new(), end.to_string(), String::new()),
            };
            let base = join(&ns, &name);
            let (id, t) = self.object(&base, "container");
            t.affiliation = Affiliation::Environmental;
            attr(&mut t.extensions, "id", end.to_string());
            attr(&mut t.extensions, "name", name);
            if !ns.is_empty() {
                attr(&mut t.extensions, "namespace", ns);
            }
            if !ty.is_empty() {
                attr(&mut t.extensions, "type", ty);
            }
            self.containers.insert(end.to_string(), id.clone());
            return Some(id);
        }
        self.errors
            .push(format!("transforms: {:?} names no container", end));
        None
    }

    fn transform(&mut self, tr: &crate::conf::Transform) {
        let (Some(src), Some(dst)) = (
            self.transform_end(&tr.source),
            self.transform_end(&tr.destination),
        ) else {
            return;
        };
        let sname = self.thing_mut(&src).name.clone();
        let dname = self.thing_mut(&dst).name.clone();
        let (id, t) = self.object(&format!("{}-{} transform", sname, dname), "transform");
        if let Some(v) = &tr.translation {
            vec_elem(&mut t.extensions, "translation", &["x", "y", "z"], v);
        }
        if let Some(v) = &tr.rotation {
            vec_elem(&mut t.extensions, "rotation", &["x", "y", "z", "w"], v);
        }
        if !tr.writers.is_empty() {
            attr(&mut t.extensions, "writers", tr.writers.join(" "));
        }
        params(t, &tr.params);
        let sys = self.system.clone();
        self.link(LinkKind::Aggregation, &sys, &id);
        for (end, tag) in [(src, "source"), (dst, "destination")] {
            let l = self.link(LinkKind::Tagged, &id, &end);
            l.tag = Some(tag.to_string());
            attr(&mut l.extensions, "end", tag);
        }
    }

    fn pose_listener(&mut self, p: &crate::conf::PoseListener) {
        let Some(cid) = self.containers.get(&p.container).cloned() else {
            self.errors.push(format!(
                "pose_listeners: {:?} names no container",
                p.container
            ));
            return;
        };
        let Some(pid) = self.processes.get(&p.process).cloned() else {
            self.errors
                .push(format!("pose_listeners: {:?} names no process", p.process));
            return;
        };
        let pose = match self.poses.get(&cid) {
            Some(id) => id.clone(),
            None => {
                let cname = self.thing_mut(&cid).name.clone();
                let (id, _) = self.object(&format!("{} pose", cname), "pose");
                self.link(LinkKind::Exhibition, &cid, &id);
                self.poses.insert(cid.clone(), id.clone());
                id
            }
        };
        let l = self.link(LinkKind::Instrument, &pose, &pid);
        l.modifier = Some(Modifier::Event);
        if let Some(r) = &p.reference {
            attr(&mut l.extensions, "reference", r.clone());
        }
        if let Some(b) = p.initial {
            attr(&mut l.extensions, "initial", b.to_string());
        }
        if let Some(b) = p.on_update {
            attr(&mut l.extensions, "on_update", b.to_string());
        }
        let lid = l.id.clone();
        self.add_arg(&pid, &p.arg, lid);
    }

    /// A Composite type: the process, its InZoom, the nodes and values of the tree.
    fn composite(&mut self, c: &Composite) {
        let (id, t) = self.process(&c.type_name, "composite");
        attr(&mut t.extensions, "type", c.type_name.clone());
        params(t, &c.params);
        t.description = c.description.clone();
        let cname = t.name.clone();
        let mut zoom = InZoom {
            id: String::new(),
            refinee: id.clone(),
            steps: Vec::new(),
            parts: Vec::new(),
            description: None,
            extensions: Default::default(),
        };
        let mut slots = BTreeMap::new();
        for (slot, ty) in &c.containers {
            let (sid, t) = self.object(&format!("{} {}", cname, slot), "slot");
            attr(&mut t.extensions, "name", slot.clone());
            attr(&mut t.extensions, "type", ty.clone());
            slots.insert(slot.clone(), sid);
        }
        let (out, t) = self.object(&format!("{} out", cname), "value");
        attr(&mut t.extensions, "value_kind", "output");
        let mut parts = Vec::new();
        self.node(&cname, &c.body, "body", &out, &slots, &mut zoom, &mut parts);
        parts.extend(slots.into_values());
        zoom.parts = parts;
        zoom.id = self.id();
        self.model.in_zooms.push(zoom);
    }

    /// A Call node yielding `out`. Its children are added to the steps first (depth first, the
    /// arguments before the node: juiz composition.md §5.1).
    #[allow(clippy::too_many_arguments)]
    fn node(
        &mut self,
        cname: &str,
        n: &Node,
        path: &str,
        out: &str,
        slots: &BTreeMap<String, String>,
        zoom: &mut InZoom,
        parts: &mut Vec<String>,
    ) {
        let Node::Call {
            process,
            container,
            args,
        } = n
        else {
            self.errors.push(format!(
                "composite {}: the body must be a process call",
                cname
            ));
            return;
        };
        let (id, t) = self.process(&format!("{} {}", cname, path), "node");
        attr(&mut t.extensions, "type", process.clone());
        attr(&mut t.extensions, "path", path.to_string());
        for (k, (arg, a)) in args.iter().enumerate() {
            let apath = format!("{}.{}", path, arg);
            let src = match a {
                Node::Call { .. } => {
                    let (v, t) = self.object(&format!("{} {} out", cname, apath), "value");
                    attr(&mut t.extensions, "value_kind", "intermediate");
                    parts.push(v.clone());
                    self.node(cname, a, &apath, &v, slots, zoom, parts);
                    v
                }
                Node::Const(x) => {
                    let (v, t) = self.object(&format!("{} {}", cname, apath), "value");
                    attr(&mut t.extensions, "value_kind", "const");
                    json_elem(&mut t.extensions, "value", x);
                    parts.push(v.clone());
                    v
                }
                Node::Expose { name, default } => {
                    let (v, t) = self.object(&format!("{} {}", cname, name), "value");
                    attr(&mut t.extensions, "name", name.clone());
                    attr(&mut t.extensions, "value_kind", "input");
                    if let Some(d) = default {
                        json_elem(&mut t.extensions, "default", d);
                    }
                    v
                }
            };
            let l = self.link(LinkKind::Instrument, &src, &id);
            attr(&mut l.extensions, "arg", arg.clone());
            attr(&mut l.extensions, "arg_index", k.to_string());
        }
        if let Some(slot) = container {
            match slots.get(slot) {
                Some(sid) => {
                    let sid = sid.clone();
                    self.link(LinkKind::Effect, &id, &sid);
                }
                None => self.errors.push(format!(
                    "composite {}: {} uses no slot {:?}",
                    cname, path, slot
                )),
            }
        }
        self.link(LinkKind::Result, &id, out);
        zoom.steps.push(vec![id]);
    }
}

fn params(t: &mut Thing, p: &Map<String, Json>) {
    if !p.is_empty() {
        json_elem(&mut t.extensions, "params", &Json::Object(p.clone()));
    }
}
