// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The model as an OPCAT file (OPX), to open in OPCAT or OPCloud (spec/opx.md).
//!
//! OPX keeps the model (`LogicalStructure`) apart from the diagrams (`VisualPart`), as the
//! interchange XML does, but numbers every entity and draws every OPD with positions. This
//! module writes the format only from its description (docs/research/opcat-opx.md); no OPCAT
//! code is used.
//!
//! - Each OPD of the model is one OPD of the file; a model without OPDs gets an SD of the
//!   top-level Things and one in-zoomed OPD for each InZoom (§3).
//! - What an OPD shows is [`crate::view`]: summed-up links of higher OPDs become links of their
//!   own in the logical part, as OPCAT keeps them (§4.3).
//! - Things are placed by a simple fixed rule unless the Appearance has a layout (§5).
//! - What OPX cannot hold is left out and reported in [`Opx::notes`] (§4.4).

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;

use opm_model::{
    Affiliation, Diagram, Essence, FanOperator, Link, LinkKind, Model, Modifier, Refinement, Thing,
};

use crate::view::{self, End, View};

/// An OPX document and what was lost on the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Opx {
    pub xml: String,
    /// One line for each kind of thing OPX could not hold, sorted.
    pub notes: Vec<String>,
}

/// What goes in the header of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// `author` of the OPMSystem.
    pub author: String,
    /// `creationDate` and `lastUpdate`, as `YYYY-MM-DDTHH:MM:SS`.
    pub date: String,
}

impl Default for Options {
    /// A fixed date, so that the same model gives the same file.
    fn default() -> Self {
        Options {
            author: "juiz_opm".to_string(),
            date: "2000-01-01T00:00:00".to_string(),
        }
    }
}

impl Options {
    /// The current time (UTC) as the date.
    pub fn now() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Options {
            date: utc_date(secs),
            ..Options::default()
        }
    }
}

/// `YYYY-MM-DDTHH:MM:SS` of seconds since 1970 (UTC).
fn utc_date(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // civil from days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        y,
        m,
        d,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

// The OPCAT numbers of the link and relation types (docs/research/opcat-opx.md).
const CONSUMPTION: u32 = 301;
const EFFECT: u32 = 302;
const INSTRUMENT: u32 = 303;
const CONDITION: u32 = 304;
const AGENT: u32 = 305;
const RESULT: u32 = 306;
const INVOCATION: u32 = 307;
const INSTRUMENT_EVENT: u32 = 308;
const EXCEPTION: u32 = 309;
const CONSUMPTION_EVENT: u32 = 310;
const SPECIALIZATION: u32 = 201;
const EXHIBITION: u32 = 202;
const INSTANTIATION: u32 = 203;
const AGGREGATION: u32 = 204;
const UNI_TAGGED: u32 = 205;
const BI_TAGGED: u32 = 206;

// Colors as OPCAT stores them (Java ARGB as a signed int).
const BACKGROUND: i32 = -1_644_826;
const OBJECT_BORDER: i32 = -16_749_056;
const PROCESS_BORDER: i32 = -16_777_046;
const STATE_BORDER: i32 = -10_790_144;
const BLACK: i32 = -16_777_216;

// OPCAT's sides of a Thing for a line end.
const N_SIDE: u32 = 1;
const S_SIDE: u32 = 4;
const W_SIDE: u32 = 7;
const E_SIDE: u32 = 8;

const NO_TIME: &str = "0;0;0;0;0;0;0";

/// One logical link or relation of the file: a model link becomes one or two of these
/// (an effect with states becomes a consumption and a result, §4.2).
#[derive(Debug, Clone)]
struct Part {
    id: u64,
    kind: u32,
    /// Model ids of the ends: a Thing or a State.
    source: String,
    target: String,
}

impl Part {
    fn structural(&self) -> bool {
        self.kind < 300
    }
}

/// Write the model as OPX.
pub fn to_opx(model: &Model, opts: &Options) -> Opx {
    // A model without OPDs gets the OPDs a reader of OPCAT expects (§3).
    let owned;
    let model = if model.diagram.is_none() {
        let mut m = model.clone();
        m.diagram = Some(default_diagrams(model));
        owned = m;
        &owned
    } else {
        model
    };
    let mut w = Writer::new(model);
    w.number_entities();
    let mut opds = String::new();
    let root = model.diagram.as_ref().expect("diagram set above");
    w.opd(root, 4, &mut opds);
    w.finish(opts, &opds)
}

/// The OPX gzipped, as OPCAT saves `.opz`.
pub fn gzip(xml: &str) -> Vec<u8> {
    use std::io::Write;
    let mut e = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    e.write_all(xml.as_bytes()).expect("writing to memory");
    e.finish().expect("writing to memory")
}

/// SD with the top-level Things, and an in-zoomed OPD for each InZoom under the OPD that shows
/// its refinee (§3).
fn default_diagrams(model: &Model) -> Diagram {
    let inner: BTreeSet<&str> = model
        .in_zooms
        .iter()
        .flat_map(|z| z.inner().map(String::as_str))
        .collect();
    let mut root = Diagram::new("opx_sd", "SD", Refinement::Root);
    for t in &model.things {
        if !inner.contains(t.id.as_str()) {
            let id = format!("opx_sd_{}", t.id);
            root.appearances
                .push(opm_model::Appearance::new(id, t.id.clone()));
        }
    }
    // the InZooms of the Things shown at the top, then of those inside each one
    fn add(model: &Model, d: &mut Diagram, holder: Option<&opm_model::InZoom>, prefix: &str) {
        let mut n = 0;
        for z in &model.in_zooms {
            let here = match holder {
                None => d.appearances.iter().any(|a| a.thing == z.refinee),
                Some(h) => h.inner().any(|i| *i == z.refinee),
            };
            if !here {
                continue;
            }
            n += 1;
            let name = format!("{}{}", prefix, n);
            let mut c = Diagram::new(format!("opx_{}", name), name.clone(), Refinement::InZoom);
            c.refinee = Some(z.refinee.clone());
            let mut shown: Vec<&str> = vec![z.refinee.as_str()];
            shown.extend(z.inner().map(String::as_str));
            // the Things linked to the inside
            for l in &model.links {
                let ends = [end_thing(model, &l.source), end_thing(model, &l.target)];
                if let [Some(a), Some(b)] = ends {
                    if z.inner().any(|i| i == a) && !shown.contains(&b) {
                        shown.push(b);
                    } else if z.inner().any(|i| i == b) && !shown.contains(&a) {
                        shown.push(a);
                    }
                }
            }
            for t in &model.things {
                if shown.contains(&t.id.as_str()) {
                    let id = format!("opx_{}_{}", name, t.id);
                    c.appearances
                        .push(opm_model::Appearance::new(id, t.id.clone()));
                }
            }
            add(model, &mut c, Some(z), &format!("{}.", name));
            d.children.push(c);
        }
    }
    add(model, &mut root, None, "SD");
    root
}

/// The Thing of a link end (the owner of a State).
fn end_thing<'a>(model: &'a Model, id: &str) -> Option<&'a str> {
    match model.state(id) {
        Some((owner, _)) => Some(owner.id.as_str()),
        None => model.thing(id).map(|t| t.id.as_str()),
    }
}

/// The aggregations (and the like) of one kind from one source on an OPD: the kind, the
/// source, and each part with its in-OPD id.
type Fundamental = (u32, String, Vec<(Part, u64)>);

/// A Thing placed on one OPD.
#[derive(Debug, Clone)]
struct Node<'a> {
    thing: &'a Thing,
    in_opd: u64,
    /// Relative to the parent Node (or the OPD).
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    /// Absolute top left.
    ax: f64,
    ay: f64,
    children: Vec<usize>,
    /// (state id, in-OPD id, x, y, w, h relative to the Node, shown)
    states: Vec<(String, u64, f64, f64, f64, f64, bool)>,
    /// Has a zoom drawn inside.
    zoomed: bool,
}

struct Writer<'a> {
    model: &'a Model,
    next_entity: u64,
    next_opd: u64,
    /// model id (Thing, State) -> entity id
    entity: HashMap<String, u64>,
    /// model link id -> its parts
    parts: HashMap<String, Vec<Part>>,
    /// summed-up links: (kind, modifier, source, target) -> parts
    derived: HashMap<(LinkKind, Option<Modifier>, String, String), Vec<Part>>,
    /// in the order they were made
    all_parts: Vec<(Part, Option<&'a Link>)>,
    notes: BTreeSet<String>,
}

impl<'a> Writer<'a> {
    fn new(model: &'a Model) -> Self {
        Writer {
            model,
            next_entity: 1,
            next_opd: 1,
            entity: HashMap::new(),
            parts: HashMap::new(),
            derived: HashMap::new(),
            all_parts: Vec::new(),
            notes: BTreeSet::new(),
        }
    }

    fn new_entity(&mut self) -> u64 {
        let n = self.next_entity;
        self.next_entity += 1;
        n
    }

    /// Things, States, then the links, in model order.
    fn number_entities(&mut self) {
        let model = self.model;
        for t in &model.things {
            let n = self.new_entity();
            self.entity.insert(t.id.clone(), n);
            for s in t.states() {
                let n = self.new_entity();
                self.entity.insert(s.id.clone(), n);
            }
            if let opm_model::ThingKind::Process { duration: Some(_) } = &t.kind {
                self.notes
                    .insert("the duration of a Process is not written".to_string());
            }
        }
        for l in &model.links {
            let parts = self.map_link(
                l,
                &l.source,
                &l.target,
                l.from_state.as_deref(),
                l.to_state.as_deref(),
            );
            for p in &parts {
                self.all_parts.push((p.clone(), Some(l)));
            }
            self.parts.insert(l.id.clone(), parts);
        }
    }

    /// The parts of a link with these ends (§4.2).
    fn map_link(
        &mut self,
        l: &Link,
        source: &str,
        target: &str,
        from_state: Option<&str>,
        to_state: Option<&str>,
    ) -> Vec<Part> {
        let mut kinds: Vec<(u32, String, String)> = Vec::new();
        let s = source.to_string();
        let t = target.to_string();
        let modifier = l.modifier;
        match l.kind {
            LinkKind::Consumption => {
                let k = match modifier {
                    Some(Modifier::Event) => CONSUMPTION_EVENT,
                    Some(Modifier::Condition) => {
                        self.note("a consumption condition link is written as a consumption link");
                        CONSUMPTION
                    }
                    None => CONSUMPTION,
                };
                kinds.push((k, s, t));
            }
            LinkKind::Result => kinds.push((RESULT, s, t)),
            LinkKind::Effect => {
                if modifier.is_some() {
                    self.note("the event or condition of an effect link is not written");
                }
                if from_state.is_some() || to_state.is_some() {
                    // an input link from the old state and an output link to the new one
                    kinds.push((
                        CONSUMPTION,
                        from_state.map(str::to_string).unwrap_or_else(|| t.clone()),
                        s.clone(),
                    ));
                    kinds.push((RESULT, s, to_state.map(str::to_string).unwrap_or(t)));
                } else {
                    // OPCAT draws an effect from the Object to the Process
                    kinds.push((EFFECT, t, s));
                }
            }
            LinkKind::Instrument => {
                let k = match modifier {
                    Some(Modifier::Event) => INSTRUMENT_EVENT,
                    Some(Modifier::Condition) => CONDITION,
                    None => INSTRUMENT,
                };
                kinds.push((k, s, t));
            }
            LinkKind::Agent => {
                if modifier.is_some() {
                    self.note("the event or condition of an agent link is not written");
                }
                kinds.push((AGENT, s, t));
            }
            LinkKind::Invocation => kinds.push((INVOCATION, s, t)),
            LinkKind::OvertimeException | LinkKind::UndertimeException => {
                self.note("overtime and undertime exception links are both OPCAT's exception link");
                kinds.push((EXCEPTION, s, t));
            }
            LinkKind::Aggregation => {
                if l.ordered {
                    self.note("the order of an aggregation is not written");
                }
                kinds.push((AGGREGATION, s, t));
            }
            LinkKind::Exhibition => kinds.push((EXHIBITION, s, t)),
            LinkKind::Generalization => kinds.push((SPECIALIZATION, s, t)),
            LinkKind::Classification => kinds.push((INSTANTIATION, s, t)),
            LinkKind::Tagged => {
                let k = if l.backward_tag.is_some() {
                    BI_TAGGED
                } else {
                    UNI_TAGGED
                };
                kinds.push((k, s, t));
            }
        }
        kinds
            .into_iter()
            .map(|(kind, source, target)| Part {
                id: self.new_entity(),
                kind,
                source,
                target,
            })
            .collect()
    }

    fn note(&mut self, s: &str) {
        self.notes.insert(s.to_string());
    }

    /// The parts that draw one shown link: the link's own, or those of a summed-up link.
    fn shown_parts(&mut self, sl: &view::ShownLink<'a>) -> Vec<Part> {
        if !sl.derived {
            return self.parts[&sl.link.id].clone();
        }
        let end = |e: &End| e.state.clone().unwrap_or_else(|| e.thing.clone());
        let (s, t) = (end(&sl.source), end(&sl.target));
        let key = (sl.link.kind, sl.link.modifier, s.clone(), t.clone());
        if let Some(p) = self.derived.get(&key) {
            return p.clone();
        }
        let parts = self.map_link(sl.link, &s, &t, None, None);
        for p in &parts {
            self.all_parts.push((p.clone(), Some(sl.link)));
        }
        self.derived.insert(key, parts.clone());
        parts
    }

    /// Write one OPD and the OPDs under it.
    fn opd(&mut self, d: &'a Diagram, indent: usize, out: &mut String) {
        let model = self.model;
        let opd_id = self.next_opd;
        self.next_opd += 1;
        let v = view::view(model, Some(d));
        let mut in_opd = 0u64;
        let mut next = || {
            in_opd += 1;
            in_opd
        };

        // nodes, and which node holds which
        let mut nodes: Vec<Node> = Vec::new();
        let mut index: HashMap<&str, usize> = HashMap::new();
        for t in &v.things {
            let hidden = &v.hidden_states;
            let n = Node {
                thing: t,
                in_opd: next(),
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                ax: 0.0,
                ay: 0.0,
                children: Vec::new(),
                states: t
                    .states()
                    .iter()
                    .map(|s| {
                        (
                            s.id.clone(),
                            0,
                            0.0,
                            0.0,
                            0.0,
                            0.0,
                            !hidden.contains(s.id.as_str()),
                        )
                    })
                    .collect(),
                zoomed: false,
            };
            index.insert(t.id.as_str(), nodes.len());
            nodes.push(n);
        }
        for n in nodes.iter_mut() {
            for s in n.states.iter_mut() {
                s.1 = next();
            }
        }
        let mut top: Vec<usize> = Vec::new();
        for (i, t) in v.things.iter().enumerate() {
            match v.zoom_holding(&t.id) {
                Some(z) => {
                    let p = index[z.in_zoom.refinee.as_str()];
                    nodes[p].children.push(i);
                    nodes[p].zoomed = true;
                }
                None => top.push(i),
            }
        }
        // children in step order, then the parts
        for z in &v.zooms {
            let p = index[z.in_zoom.refinee.as_str()];
            nodes[p].children = z.inner.iter().map(|i| index[i]).collect();
        }
        let main = match d.refinement {
            Refinement::Root => None,
            _ => d
                .refinee
                .as_deref()
                .and_then(|r| index.get(r).copied())
                .or_else(|| top.first().copied()),
        };
        if let Some(m) = main {
            top.retain(|&i| i != m);
        }

        place(&mut nodes, &top, main, &v, d);

        // the XML of the OPD
        let pad = " ".repeat(indent);
        let name = opd_name(model, d);
        let mut body = String::new();
        let ind = indent + 2;
        if let Some(m) = main {
            let _ = writeln!(body, "{}<MainEntity>", " ".repeat(ind));
            self.visual_thing(&nodes, m, true, ind + 2, &mut body);
            let _ = writeln!(body, "{}</MainEntity>", " ".repeat(ind));
        }
        let _ = writeln!(body, "{}<ThingSection>", " ".repeat(ind));
        for &i in &top {
            self.visual_thing(&nodes, i, false, ind + 2, &mut body);
        }
        let _ = writeln!(body, "{}</ThingSection>", " ".repeat(ind));

        // links: (part, source in-OPD id, target in-OPD id, visual in-OPD id)
        let instance = |id: &str| -> Option<(u64, f64, f64, f64, f64)> {
            if let Some(&i) = index.get(id) {
                let n = &nodes[i];
                return Some((n.in_opd, n.ax, n.ay, n.w, n.h));
            }
            let (owner, _) = model.state(id)?;
            let n = &nodes[*index.get(owner.id.as_str())?];
            let s = n.states.iter().find(|s| s.0 == id)?;
            Some((s.1, n.ax + s.2, n.ay + s.3, s.4, s.5))
        };
        let mut fundamental: Vec<Fundamental> = Vec::new();
        let mut general = String::new();
        let mut links = String::new();
        // model link id -> visual in-OPD ids of its parts (for fans)
        let mut visual_of: HashMap<&str, Vec<(u64, u64)>> = HashMap::new();
        let shown: Vec<view::ShownLink> = v.links.clone();
        for sl in &shown {
            let parts = self.shown_parts(sl);
            for p in parts {
                let (Some(src), Some(dst)) = (instance(&p.source), instance(&p.target)) else {
                    continue;
                };
                let vid = next();
                if !sl.derived {
                    visual_of
                        .entry(sl.link.id.as_str())
                        .or_default()
                        .push((p.id, vid));
                }
                if matches!(
                    p.kind,
                    AGGREGATION | EXHIBITION | SPECIALIZATION | INSTANTIATION
                ) {
                    match fundamental
                        .iter_mut()
                        .find(|f| f.0 == p.kind && f.1 == p.source)
                    {
                        Some(f) => f.2.push((p.clone(), vid)),
                        None => {
                            fundamental.push((p.kind, p.source.clone(), vec![(p.clone(), vid)]))
                        }
                    }
                    continue;
                }
                let (ss, sp) = side(src, dst);
                let (ds, dp) = side(dst, src);
                let line = format!(
                    "<LineAttr autoArranged=\"true\" sourceId=\"{}\" sourceInOpdId=\"{}\" sourceConnectionSide=\"{}\" sourceConnectionParameter=\"{}\" destinationId=\"{}\" destinationInOpdId=\"{}\" destinationConnectionSide=\"{}\" destinationConnectionParameter=\"{}\" />",
                    self.entity_of(&p.source), src.0, ss, sp, self.entity_of(&p.target), dst.0, ds, dp
                );
                let inst = instance_attr(p.id, vid, BACKGROUND, BLACK);
                let i2 = " ".repeat(ind + 2);
                let i4 = " ".repeat(ind + 4);
                if p.structural() {
                    let (lx, ly) = mid(src, dst);
                    let _ = writeln!(
                        general,
                        "{i2}<VisualGeneralRelation sourceLabelX=\"{}\" sourceLabelY=\"{}\" destinationLabelX=\"{}\" destinationLabelY=\"{}\">\n{i4}{}\n{i4}{}\n{i2}</VisualGeneralRelation>",
                        lx as i64, ly as i64 - 12, lx as i64, ly as i64 + 12, inst, line
                    );
                } else {
                    let _ = writeln!(
                        links,
                        "{i2}<VisualLink>\n{i4}{}\n{i4}{}\n{i2}</VisualLink>",
                        inst, line
                    );
                }
            }
        }
        // XOR / OR fans whose links are all drawn here as they are
        for f in &model.fans {
            let members: Option<Vec<(u64, u64)>> = f
                .links
                .iter()
                .map(|l| visual_of.get(l.as_str()).and_then(|v| v.first().copied()))
                .collect();
            let Some(members) = members else { continue };
            let Some(first) = model.link(&f.links[0]) else {
                continue;
            };
            let source_group = !first.kind.is_enabling_or_consuming();
            let i2 = " ".repeat(ind + 2);
            let i4 = " ".repeat(ind + 4);
            let _ = writeln!(
                links,
                "{i2}<OrXorGroup isSourceGroup=\"{}\" type=\"{}\">",
                source_group,
                match f.operator {
                    FanOperator::Or => 0,
                    FanOperator::Xor => 1,
                }
            );
            for (e, vid) in members {
                let _ = writeln!(
                    links,
                    "{i4}<Member memberId=\"{}\" memberInOpdId=\"{}\" />",
                    e, vid
                );
            }
            let _ = writeln!(links, "{i2}</OrXorGroup>");
        }

        let i2 = " ".repeat(ind + 2);
        let i4 = " ".repeat(ind + 4);
        let _ = writeln!(body, "{}<FundamentalRelationSection>", " ".repeat(ind));
        for (_, source, members) in &fundamental {
            let Some(src) = instance(source) else {
                continue;
            };
            let dsts: Vec<_> = members
                .iter()
                .filter_map(|(p, _)| instance(&p.target))
                .collect();
            let cx = dsts.iter().map(|d| d.1 + d.3 / 2.0).sum::<f64>() / dsts.len().max(1) as f64;
            let cy = dsts.iter().map(|d| d.2 + d.4 / 2.0).sum::<f64>() / dsts.len().max(1) as f64;
            let (tx, ty) = (
                (src.1 + src.3 / 2.0 + cx) / 2.0,
                (src.2 + src.4 / 2.0 + cy) / 2.0,
            );
            let tri = (0, tx - 14.0, ty - 12.0, 28.0, 24.0);
            let (ss, sp) = side(src, tri);
            let _ = writeln!(
                body,
                "{i2}<CommonPart sourceId=\"{}\" sourceInOpdId=\"{}\" sourceConnectionSide=\"{}\" sourceConnectionParameter=\"{}\" x=\"{}\" y=\"{}\" width=\"28\" height=\"24\" backgroundColor=\"{}\" borderColor=\"{}\">",
                self.entity_of(source), src.0, ss, sp, (tx - 14.0) as i64, (ty - 12.0) as i64, BACKGROUND, BLACK
            );
            for (p, vid) in members {
                let Some(dst) = instance(&p.target) else {
                    continue;
                };
                let (ds, dp) = side(dst, tri);
                let _ = writeln!(
                    body,
                    "{i4}<VisualFundamentalRelation destinationId=\"{}\" destinationInOpdId=\"{}\" destinationSide=\"{}\" destinationParameter=\"{}\" labelX=\"{}\" labelY=\"{}\">\n{i4}  {}\n{i4}</VisualFundamentalRelation>",
                    self.entity_of(&p.target), dst.0, ds, dp, tx as i64, ty as i64 + 16,
                    instance_attr(p.id, *vid, BACKGROUND, BLACK)
                );
            }
            let _ = writeln!(body, "{i2}</CommonPart>");
        }
        let _ = writeln!(body, "{}</FundamentalRelationSection>", " ".repeat(ind));
        let _ = write!(
            body,
            "{}<GeneralRelationSection>\n{}{}</GeneralRelationSection>\n",
            " ".repeat(ind),
            general,
            " ".repeat(ind)
        );
        let _ = write!(
            body,
            "{}<VisualLinkSection>\n{}{}</VisualLinkSection>\n",
            " ".repeat(ind),
            links,
            " ".repeat(ind)
        );

        // the OPDs under this one
        let here: HashMap<String, u64> = nodes
            .iter()
            .map(|n| (n.thing.id.clone(), n.in_opd))
            .collect();
        let mut unfolded = String::new();
        let mut views = String::new();
        let mut zoomed = String::new();
        for c in &d.children {
            match c.refinement {
                Refinement::InZoom => self.opd(c, ind + 2, &mut zoomed),
                Refinement::Unfold => {
                    let r = c.refinee.as_deref().unwrap_or_default();
                    let _ = writeln!(
                        unfolded,
                        "{i2}<UnfoldingProperties entityId=\"{}\" entityInOpdId=\"{}\">",
                        self.entity.get(r).copied().unwrap_or(0),
                        here.get(r).copied().unwrap_or(0)
                    );
                    self.opd(c, ind + 4, &mut unfolded);
                    let _ = writeln!(unfolded, "{i2}</UnfoldingProperties>");
                }
                Refinement::View | Refinement::Root => {
                    let r = c
                        .refinee
                        .clone()
                        .or_else(|| c.appearances.first().map(|a| a.thing.clone()))
                        .unwrap_or_default();
                    let _ = writeln!(
                        views,
                        "{i2}<ViewProperties entityId=\"{}\" entityInOpdId=\"{}\">",
                        self.entity.get(&r).copied().unwrap_or(0),
                        here.get(&r).copied().unwrap_or(0)
                    );
                    self.opd(c, ind + 4, &mut views);
                    let _ = writeln!(views, "{i2}</ViewProperties>");
                }
            }
        }
        let _ = write!(
            body,
            "{}<Unfolded>\n{}{}</Unfolded>\n",
            " ".repeat(ind),
            unfolded,
            " ".repeat(ind)
        );
        let _ = write!(
            body,
            "{}<View>\n{}{}</View>\n",
            " ".repeat(ind),
            views,
            " ".repeat(ind)
        );
        let _ = write!(
            body,
            "{}<InZoomed>\n{}{}</InZoomed>\n",
            " ".repeat(ind),
            zoomed,
            " ".repeat(ind)
        );
        let _ = writeln!(body, "{}<openReuseRelations />", " ".repeat(ind));

        let _ = writeln!(
            out,
            "{pad}<OPD id=\"{}\" name=\"{}\" maxEntityEntry=\"{}\" locked=\"false\">",
            opd_id,
            esc(&name),
            in_opd
        );
        out.push_str(&body);
        let _ = writeln!(out, "{pad}</OPD>");
    }

    fn entity_of(&self, id: &str) -> u64 {
        self.entity.get(id).copied().unwrap_or(0)
    }

    fn visual_thing(&self, nodes: &[Node], i: usize, main: bool, indent: usize, out: &mut String) {
        let n = &nodes[i];
        let p0 = " ".repeat(indent);
        let p1 = " ".repeat(indent + 2);
        let p2 = " ".repeat(indent + 4);
        let p3 = " ".repeat(indent + 6);
        let id = self.entity_of(&n.thing.id);
        let edge = format!(
            "<ConnectionEdgeAttr x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />",
            n.x as i64, n.y as i64, n.w as i64, n.h as i64
        );
        let text_pos = if main || n.zoomed { "N" } else { "C" };
        let _ = writeln!(out, "{p0}<VisualThing>\n{p1}<ThingData>");
        if n.thing.is_process() {
            let _ = writeln!(
                out,
                "{p2}<VisualProcess>\n{p3}{}\n{p3}{}\n{p3}<ThingInstanceAttr layer=\"0\" textPosition=\"{}\" />\n{p2}</VisualProcess>",
                instance_attr(id, n.in_opd, BACKGROUND, PROCESS_BORDER),
                edge,
                text_pos
            );
        } else {
            let _ = writeln!(
                out,
                "{p2}<VisualObject statesAutoArranged=\"false\">\n{p3}{}\n{p3}{}\n{p3}<ThingInstanceAttr layer=\"0\" textPosition=\"{}\" />",
                instance_attr(id, n.in_opd, BACKGROUND, OBJECT_BORDER),
                edge,
                text_pos
            );
            let p4 = " ".repeat(indent + 8);
            for (sid, sin, x, y, w, h, shown) in &n.states {
                let _ = writeln!(
                    out,
                    "{p3}<VisualState visible=\"{}\">\n{p4}{}\n{p4}<ConnectionEdgeAttr x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" />\n{p3}</VisualState>",
                    shown,
                    instance_attr(self.entity_of(sid), *sin, BACKGROUND, STATE_BORDER),
                    *x as i64, *y as i64, *w as i64, *h as i64
                );
            }
            let _ = writeln!(out, "{p2}</VisualObject>");
        }
        let _ = writeln!(out, "{p1}</ThingData>");
        if n.children.is_empty() {
            let _ = writeln!(out, "{p1}<ChildrenContainer />");
        } else {
            let _ = writeln!(out, "{p1}<ChildrenContainer>");
            for &c in &n.children {
                self.visual_thing(nodes, c, false, indent + 4, out);
            }
            let _ = writeln!(out, "{p1}</ChildrenContainer>");
        }
        let _ = writeln!(out, "{p0}</VisualThing>");
    }

    fn finish(self, opts: &Options, opds: &str) -> Opx {
        let model = self.model;
        let mut x = String::new();
        let name = model.name.clone().unwrap_or_else(|| model.id.clone());
        let _ = writeln!(x, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
        let _ = writeln!(x, "<OPX version=\"3\">");
        let _ = writeln!(
            x,
            "  <OPMSystem creationDate=\"{d}\" lastUpdate=\"{d}\" author=\"{}\" modeltype=\"System\" name=\"{}\" maxEntityEntry=\"{}\" maxOpdEntry=\"{}\" global-id=\"{}\">",
            esc(&opts.author),
            esc(&name),
            self.next_entity - 1,
            self.next_opd - 1,
            esc(&model.id),
            d = esc(&opts.date)
        );
        x.push_str(&system_properties());
        let _ = writeln!(x, "    <LogicalStructure>\n      <ObjectSection>");
        for t in model.things.iter().filter(|t| t.is_object()) {
            let _ = writeln!(
                x,
                "        <LogicalObject indexName=\"\" indexOrder=\"0\" initialValue=\"\" key=\"false\" persistent=\"true\" type=\"\" typeOriginId=\"-1\" locked=\"false\">"
            );
            x.push_str(&entity_attr(
                self.entity_of(&t.id),
                &t.name,
                t.description.as_deref(),
                t.affiliation == Affiliation::Environmental,
                10,
            ));
            let _ = writeln!(x, "          {}", thing_attr(t));
            for s in t.states() {
                let _ = writeln!(
                    x,
                    "          <LogicalState default=\"{}\" final=\"{}\" initial=\"{}\" locked=\"false\" maxTime=\"infinity\" minTime=\"{}\">",
                    s.default, s.final_, s.initial, NO_TIME
                );
                x.push_str(&entity_attr(
                    self.entity_of(&s.id),
                    &s.name,
                    s.description.as_deref(),
                    false,
                    12,
                ));
                let _ = writeln!(x, "          </LogicalState>");
            }
            let _ = writeln!(x, "        </LogicalObject>");
        }
        let _ = writeln!(x, "      </ObjectSection>\n      <ProcessSection>");
        for t in model.things.iter().filter(|t| t.is_process()) {
            let _ = writeln!(
                x,
                "        <LogicalProcess maxTimeActivation=\"infinity\" minTimeActivation=\"{}\" locked=\"false\">",
                NO_TIME
            );
            x.push_str(&entity_attr(
                self.entity_of(&t.id),
                &t.name,
                t.description.as_deref(),
                t.affiliation == Affiliation::Environmental,
                10,
            ));
            let _ = writeln!(x, "          {}", thing_attr(t));
            let _ = writeln!(
                x,
                "          <processBody>none</processBody>\n        </LogicalProcess>"
            );
        }
        let _ = writeln!(x, "      </ProcessSection>\n      <RelationSection>");
        for (p, l) in self.all_parts.iter().filter(|(p, _)| p.structural()) {
            let l = l.expect("a part has its link");
            let _ = writeln!(
                x,
                "        <LogicalRelation relationType=\"{}\" sourceId=\"{}\" destinationId=\"{}\" sourceCardinality=\"{}\" destinationCardinality=\"{}\" forwardRelationMeaning=\"{}\" backwardRelationMeaning=\"{}\" locked=\"false\">",
                p.kind,
                self.entity_of(&p.source),
                self.entity_of(&p.target),
                esc(l.source_multiplicity.as_deref().unwrap_or("1")),
                esc(l.target_multiplicity.as_deref().unwrap_or("1")),
                esc(l.tag.as_deref().unwrap_or("")),
                esc(l.backward_tag.as_deref().unwrap_or(""))
            );
            x.push_str(&entity_attr(
                p.id,
                &format!("{} {}", l.kind.name(), p.id),
                l.description.as_deref(),
                false,
                10,
            ));
            let _ = writeln!(x, "        </LogicalRelation>");
        }
        let _ = writeln!(x, "      </RelationSection>\n      <LinkSection>");
        for (p, l) in self.all_parts.iter().filter(|(p, _)| !p.structural()) {
            let l = l.expect("a part has its link");
            let _ = writeln!(
                x,
                "        <LogicalLink linkType=\"{}\" sourceId=\"{}\" destinationId=\"{}\" condition=\"\" path=\"{}\" maxReactionTime=\"infinity\" minReactionTime=\"{}\" locked=\"false\">",
                p.kind,
                self.entity_of(&p.source),
                self.entity_of(&p.target),
                esc(l.path.as_deref().unwrap_or("")),
                NO_TIME
            );
            x.push_str(&entity_attr(
                p.id,
                &format!("{} {}", l.kind.name(), p.id),
                l.description.as_deref(),
                false,
                10,
            ));
            let _ = writeln!(x, "        </LogicalLink>");
        }
        let _ = writeln!(
            x,
            "      </LinkSection>\n    </LogicalStructure>\n    <VisualPart>"
        );
        x.push_str(opds);
        let _ = writeln!(x, "    </VisualPart>\n  </OPMSystem>\n</OPX>");
        Opx {
            xml: x,
            notes: self.notes.into_iter().collect(),
        }
    }
}

/// The OPD name as OPCAT writes it: `SD1 - Making in-zoomed` (§3).
fn opd_name(model: &Model, d: &Diagram) -> String {
    let refinee = d
        .refinee
        .as_deref()
        .and_then(|r| model.thing(r))
        .map(|t| t.name.as_str());
    match (d.refinement, refinee) {
        (Refinement::Root, _) => d.name.clone(),
        (Refinement::InZoom, Some(r)) => format!("{} - {} in-zoomed", d.name, r),
        (Refinement::Unfold, Some(r)) => format!("{} - {} unfolded", d.name, r),
        // OPCAT takes an OPD whose name has "View" before " - " as a view
        (Refinement::View, _) => format!(
            "{} View - {}",
            d.name,
            d.description.as_deref().unwrap_or(&d.name)
        ),
        _ => d.name.clone(),
    }
}

fn instance_attr(entity: u64, in_opd: u64, background: i32, border: i32) -> String {
    format!(
        "<InstanceAttr entityId=\"{}\" entityInOpdId=\"{}\" backgroundColor=\"{}\" borderColor=\"{}\" textColor=\"{}\" />",
        entity, in_opd, background, border, BLACK
    )
}

fn entity_attr(
    id: u64,
    name: &str,
    description: Option<&str>,
    environmental: bool,
    indent: usize,
) -> String {
    let p = " ".repeat(indent);
    format!(
        "{p}<EntityAttr id=\"{}\" environmental=\"{}\">\n{p}  <name>{}</name>\n{p}  <description>{}</description>\n{p}  <url>none</url>\n{p}</EntityAttr>\n",
        id,
        environmental,
        esc(name),
        esc(description.unwrap_or("none"))
    )
}

fn thing_attr(t: &Thing) -> String {
    format!(
        "<ThingAttr numberOfInstances=\"1\" physical=\"{}\" role=\"\" scope=\"0\" />",
        t.essence == Essence::Physical
    )
}

/// The side of `a` that faces `b`, and the place on it (0 to 1). Boxes are
/// (in-OPD id, x, y, w, h), absolute.
fn side(a: (u64, f64, f64, f64, f64), b: (u64, f64, f64, f64, f64)) -> (u32, f64) {
    let (ax, ay) = (a.1 + a.3 / 2.0, a.2 + a.4 / 2.0);
    let (bx, by) = (b.1 + b.3 / 2.0, b.2 + b.4 / 2.0);
    let (dx, dy) = (bx - ax, by - ay);
    // inside: b holds a (a Thing in an in-zoomed one), or the other way
    if dx.abs() * a.4 > dy.abs() * a.3 {
        (if dx > 0.0 { E_SIDE } else { W_SIDE }, 0.5)
    } else {
        (if dy > 0.0 { S_SIDE } else { N_SIDE }, 0.5)
    }
}

fn mid(a: (u64, f64, f64, f64, f64), b: (u64, f64, f64, f64, f64)) -> (f64, f64) {
    (
        (a.1 + a.3 / 2.0 + b.1 + b.3 / 2.0) / 2.0,
        (a.2 + a.4 / 2.0 + b.2 + b.4 / 2.0) / 2.0,
    )
}

// ---- placing the Things (§5) ----

const GAP: f64 = 40.0;
const COLUMN_GAP: f64 = 140.0;
const ZOOM_PAD: f64 = 30.0;
const ZOOM_HEAD: f64 = 36.0;
const STATE_H: f64 = 28.0;

fn text_w(s: &str) -> f64 {
    s.chars().count() as f64 * 8.0
}

/// Size the Node and its inside, and place the inside (relative to the Node).
fn size(nodes: &mut [Node], i: usize, v: &View, d: &Diagram) {
    if let Some(l) = layout_of(d, &nodes[i].thing.id) {
        if let (Some(w), Some(h)) = (l.width, l.height) {
            nodes[i].w = w;
            nodes[i].h = h;
        }
    }
    let children = nodes[i].children.clone();
    if !children.is_empty() {
        for &c in &children {
            size(nodes, c, v, d);
        }
        // the steps of the InZoom one under another (members of a step side by side),
        // the Objects inside in a column on the left
        let zoom = v.zoom_of(&nodes[i].thing.id).map(|z| z.in_zoom);
        let mut rows: Vec<Vec<usize>> = Vec::new();
        let mut parts: Vec<usize> = Vec::new();
        if let Some(z) = zoom {
            for step in &z.steps {
                let row: Vec<usize> = children
                    .iter()
                    .copied()
                    .filter(|&c| step.contains(&nodes[c].thing.id))
                    .collect();
                if !row.is_empty() {
                    rows.push(row);
                }
            }
            parts = children
                .iter()
                .copied()
                .filter(|&c| !rows.iter().flatten().any(|&r| r == c))
                .collect();
        }
        let parts_w = parts.iter().map(|&c| nodes[c].w).fold(0.0, f64::max);
        let left = ZOOM_PAD + if parts.is_empty() { 0.0 } else { parts_w + GAP };
        let mut y = ZOOM_HEAD;
        let mut width: f64 = 0.0;
        for row in &rows {
            let row_w: f64 =
                row.iter().map(|&c| nodes[c].w).sum::<f64>() + GAP * (row.len() - 1) as f64;
            width = width.max(row_w);
        }
        for row in &rows {
            let row_w: f64 =
                row.iter().map(|&c| nodes[c].w).sum::<f64>() + GAP * (row.len() - 1) as f64;
            let mut x = left + (width - row_w) / 2.0;
            let h = row.iter().map(|&c| nodes[c].h).fold(0.0, f64::max);
            for &c in row {
                nodes[c].x = x;
                nodes[c].y = y + (h - nodes[c].h) / 2.0;
                x += nodes[c].w + GAP;
            }
            y += h + GAP;
        }
        let mut py = ZOOM_HEAD;
        for &c in &parts {
            nodes[c].x = ZOOM_PAD;
            nodes[c].y = py;
            py += nodes[c].h + GAP;
        }
        for &c in &children {
            if let Some(l) = layout_of(d, &nodes[c].thing.id) {
                nodes[c].x = l.x;
                nodes[c].y = l.y;
            }
        }
        let inner_w = children
            .iter()
            .map(|&c| nodes[c].x + nodes[c].w)
            .fold(0.0, f64::max)
            + ZOOM_PAD;
        let inner_h = children
            .iter()
            .map(|&c| nodes[c].y + nodes[c].h)
            .fold(0.0, f64::max)
            + ZOOM_PAD;
        if nodes[i].thing.is_process() && layout_of(d, &nodes[i].thing.id).is_none() {
            // an in-zoomed Process is an ellipse: the box of the inside fits in it when the
            // ellipse is √2 times as wide and as high; move the inside to its middle
            let (w, h) = (
                inner_w * std::f64::consts::SQRT_2,
                inner_h * std::f64::consts::SQRT_2,
            );
            let (dx, dy) = ((w - inner_w) / 2.0, (h - inner_h) / 2.0);
            for &c in &children {
                nodes[c].x += dx;
                nodes[c].y += dy;
            }
            nodes[i].w = w;
            nodes[i].h = h;
        } else {
            nodes[i].w = nodes[i].w.max(inner_w);
            nodes[i].h = nodes[i].h.max(inner_h);
        }
        nodes[i].w = nodes[i].w.max(text_w(&nodes[i].thing.name) + 60.0);
        return;
    }
    let name_w = text_w(&nodes[i].thing.name);
    let mut x = 10.0;
    let state_count = nodes[i].states.len();
    for s in nodes[i].states.iter_mut() {
        let name = v
            .model
            .state(&s.0)
            .map(|(_, st)| st.name.clone())
            .unwrap_or_default();
        let w = (text_w(&name) + 16.0).max(50.0);
        s.2 = x;
        s.4 = w;
        s.5 = STATE_H;
        x += w + 10.0;
    }
    let (min_w, h) = if nodes[i].thing.is_process() {
        ((name_w + 50.0).max(120.0), 60.0)
    } else if state_count > 0 {
        ((name_w + 30.0).max(x).max(100.0), 40.0 + STATE_H + 10.0)
    } else {
        ((name_w + 30.0).max(100.0), 50.0)
    };
    if nodes[i].w == 0.0 {
        nodes[i].w = min_w;
        nodes[i].h = h;
    }
    let h = nodes[i].h;
    for s in nodes[i].states.iter_mut() {
        s.3 = h - STATE_H - 8.0;
    }
}

fn layout_of<'d>(d: &'d Diagram, thing: &str) -> Option<&'d opm_model::Layout> {
    d.appearances
        .iter()
        .find(|a| a.thing == thing)
        .and_then(|a| a.layout.as_ref())
}

/// Place the top-level Things in three columns: the Objects that only go into Processes,
/// the Processes (and the main Thing), the other Objects.
fn place(nodes: &mut [Node], top: &[usize], main: Option<usize>, v: &View, d: &Diagram) {
    for &i in top.iter().chain(main.iter()) {
        size(nodes, i, v, d);
    }
    let mut cols: [Vec<usize>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    if let Some(m) = main {
        cols[1].push(m);
    }
    for &i in top {
        let t = nodes[i].thing;
        if t.is_process() {
            cols[1].push(i);
            continue;
        }
        let ends: Vec<&view::ShownLink> = v
            .links
            .iter()
            .filter(|l| l.source.thing == t.id || l.target.thing == t.id)
            .collect();
        let outgoing_only = ends.iter().all(|l| {
            l.link.kind.is_structural()
                || (l.source.thing == t.id && l.link.kind.is_enabling_or_consuming())
        });
        cols[if outgoing_only { 0 } else { 2 }].push(i);
    }
    let mut x = 20.0;
    for col in &cols {
        if col.is_empty() {
            continue;
        }
        let w = col.iter().map(|&i| nodes[i].w).fold(0.0, f64::max);
        let mut y = 20.0;
        for &i in col {
            nodes[i].x = x + (w - nodes[i].w) / 2.0;
            nodes[i].y = y;
            y += nodes[i].h + GAP;
        }
        x += w + COLUMN_GAP;
    }
    for &i in top.iter().chain(main.iter()) {
        if let Some(l) = layout_of(d, &nodes[i].thing.id) {
            nodes[i].x = l.x;
            nodes[i].y = l.y;
        }
    }
    fn absolute(nodes: &mut [Node], i: usize, ox: f64, oy: f64) {
        nodes[i].ax = ox + nodes[i].x;
        nodes[i].ay = oy + nodes[i].y;
        let (ax, ay) = (nodes[i].ax, nodes[i].ay);
        for c in nodes[i].children.clone() {
            absolute(nodes, c, ax, ay);
        }
    }
    for &i in top.iter().chain(main.iter()) {
        absolute(nodes, i, 0.0, 0.0);
    }
}

fn system_properties() -> String {
    // The display settings OPCAT writes in every file, with its default values.
    const PROPS: [(&str, &str); 26] = [
        ("StateColor", "-10790144"),
        ("MinimalThingHeight", "35"),
        ("StateHeight", "35"),
        ("ThingHeight", "70"),
        ("SmallFont", "11 0"),
        ("ThingWidth", "120"),
        ("ObjectColor", "-16749056"),
        ("LineFont", "11 0"),
        ("TextColor", "-16777216"),
        ("BackgroundColor", "-1644826"),
        ("OPDWidth", "1300"),
        ("LabelFont", "11 0"),
        ("NormalSize", "5"),
        ("StateFont", "12 0"),
        ("DraggerSize", "36"),
        ("OPDHeight", "860"),
        ("URLFont", "16 3"),
        ("CurrentSize", "4"),
        ("FundamentalRelationHeight", "31"),
        ("FundamentalRelationWidth", "36"),
        ("UrlColor", "-65536"),
        ("ProcessColor", "-16777046"),
        ("ThingFont", "16 0"),
        ("MinimalThingWidth", "60"),
        ("StateWidth", "60"),
        ("LinkFont", "12 0"),
    ];
    let mut s = String::from("    <SystemProperties>\n");
    for section in ["SystemConfiguration", "GeneralInfo"] {
        let _ = writeln!(s, "      <{}>", section);
        for (k, v) in PROPS {
            let _ = writeln!(
                s,
                "        <Property key=\"{}\">\n          <value>{}</value>\n        </Property>",
                k, v
            );
        }
        let _ = writeln!(s, "      </{}>", section);
    }
    s.push_str("    </SystemProperties>\n");
    s
}

/// Escape text for XML text and attribute values.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\n' => o.push_str("&#10;"),
            _ => o.push(c),
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date() {
        assert_eq!(utc_date(0), "1970-01-01T00:00:00");
        assert_eq!(utc_date(1_763_250_000), "2025-11-15T23:40:00");
    }
}
