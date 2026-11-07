// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The metamodel (spec/metamodel.md): core and diagram layers.
//!
//! The types mirror the tables of the metamodel. Annotations of other layers (juiz) are kept as
//! [`Extensions`] on every element that has an id, without interpretation.

use crate::ext::Extensions;

/// Namespace of the core and diagram layers (spec/xml.md §2).
pub const OPM_NS: &str = "https://openjuiz.github.io/ns/opm/v0";
/// Namespace of the juiz extension (spec/xml.md §2). opm_model does not interpret it.
pub const JUIZ_NS: &str = "https://openjuiz.github.io/ns/juiz/v0";
/// The only `format_version` this crate reads and writes (spec/xml.md §11).
pub const FORMAT_VERSION: &str = "0.1";

/// A whole model: one interchange document.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub id: String,
    pub name: Option<String>,
    pub format_version: String,
    pub description: Option<String>,
    pub extensions: Extensions,
    pub things: Vec<Thing>,
    pub links: Vec<Link>,
    pub fans: Vec<Fan>,
    pub in_zooms: Vec<InZoom>,
    /// The root OPD (SD), if any.
    pub diagram: Option<Diagram>,
    /// Namespace prefixes to use when writing, in order (`(prefix, uri)`).
    /// The reader fills it from the declarations on the root element. The `opm` prefix is
    /// always used for the core namespace whatever this says.
    pub prefixes: Vec<(String, String)>,
}

impl Model {
    /// An empty model of the current format version.
    pub fn new(id: impl Into<String>) -> Self {
        Model {
            id: id.into(),
            name: None,
            format_version: FORMAT_VERSION.to_string(),
            description: None,
            extensions: Extensions::default(),
            things: Vec::new(),
            links: Vec::new(),
            fans: Vec::new(),
            in_zooms: Vec::new(),
            diagram: None,
            prefixes: Vec::new(),
        }
    }

    pub fn thing(&self, id: &str) -> Option<&Thing> {
        self.things.iter().find(|t| t.id == id)
    }

    pub fn thing_by_name(&self, name: &str) -> Option<&Thing> {
        self.things.iter().find(|t| t.name == name)
    }

    pub fn link(&self, id: &str) -> Option<&Link> {
        self.links.iter().find(|l| l.id == id)
    }

    /// The state with this id and its owner.
    pub fn state(&self, id: &str) -> Option<(&Thing, &State)> {
        self.things
            .iter()
            .find_map(|t| t.states().iter().find(|s| s.id == id).map(|s| (t, s)))
    }

    pub fn in_zoom_of(&self, refinee: &str) -> Option<&InZoom> {
        self.in_zooms.iter().find(|z| z.refinee == refinee)
    }

    /// All OPDs, root first, depth first.
    pub fn diagrams(&self) -> Vec<&Diagram> {
        fn walk<'a>(d: &'a Diagram, out: &mut Vec<&'a Diagram>) {
            out.push(d);
            for c in &d.children {
                walk(c, out);
            }
        }
        let mut out = Vec::new();
        if let Some(d) = &self.diagram {
            walk(d, &mut out);
        }
        out
    }
}

/// metamodel §4.1 essence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Essence {
    #[default]
    Informatical,
    Physical,
}

/// metamodel §4.1 affiliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Affiliation {
    #[default]
    Systemic,
    Environmental,
}

/// An Object or a Process (metamodel §4).
#[derive(Debug, Clone, PartialEq)]
pub struct Thing {
    pub id: String,
    pub name: String,
    pub essence: Essence,
    pub affiliation: Affiliation,
    pub description: Option<String>,
    pub extensions: Extensions,
    pub kind: ThingKind,
}

/// What only an Object or only a Process has (metamodel §4.2, §4.3).
#[derive(Debug, Clone, PartialEq)]
pub enum ThingKind {
    Object { states: Vec<State> },
    Process { duration: Option<Duration> },
}

impl Thing {
    pub fn object(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::with_kind(id, name, ThingKind::Object { states: Vec::new() })
    }

    pub fn process(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::with_kind(id, name, ThingKind::Process { duration: None })
    }

    fn with_kind(id: impl Into<String>, name: impl Into<String>, kind: ThingKind) -> Self {
        Thing {
            id: id.into(),
            name: name.into(),
            essence: Essence::default(),
            affiliation: Affiliation::default(),
            description: None,
            extensions: Extensions::default(),
            kind,
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self.kind, ThingKind::Object { .. })
    }

    pub fn is_process(&self) -> bool {
        matches!(self.kind, ThingKind::Process { .. })
    }

    /// The states of an Object; empty for a Process.
    pub fn states(&self) -> &[State] {
        match &self.kind {
            ThingKind::Object { states } => states,
            ThingKind::Process { .. } => &[],
        }
    }
}

/// A State of an Object (metamodel §5). The owner is the Object that holds it.
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub id: String,
    pub name: String,
    pub initial: bool,
    pub final_: bool,
    pub default: bool,
    pub description: Option<String>,
    pub extensions: Extensions,
}

impl State {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        State {
            id: id.into(),
            name: name.into(),
            initial: false,
            final_: false,
            default: false,
            description: None,
            extensions: Extensions::default(),
        }
    }
}

/// The ISO 19450 duration of a Process (metamodel §4.3).
#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    pub min: Option<f64>,
    pub expected: Option<f64>,
    pub max: Option<f64>,
    pub unit: String,
}

/// The kind of a link (metamodel §6.2, §6.3). The XML element name is [`LinkKind::name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkKind {
    // procedural
    Consumption,
    Result,
    Effect,
    Instrument,
    Agent,
    Invocation,
    OvertimeException,
    UndertimeException,
    // structural
    Aggregation,
    Exhibition,
    Generalization,
    Classification,
    Tagged,
}

impl LinkKind {
    pub const ALL: [LinkKind; 13] = [
        LinkKind::Consumption,
        LinkKind::Result,
        LinkKind::Effect,
        LinkKind::Instrument,
        LinkKind::Agent,
        LinkKind::Invocation,
        LinkKind::OvertimeException,
        LinkKind::UndertimeException,
        LinkKind::Aggregation,
        LinkKind::Exhibition,
        LinkKind::Generalization,
        LinkKind::Classification,
        LinkKind::Tagged,
    ];

    /// The metamodel name, which is also the XML element name.
    pub fn name(self) -> &'static str {
        match self {
            LinkKind::Consumption => "consumption",
            LinkKind::Result => "result",
            LinkKind::Effect => "effect",
            LinkKind::Instrument => "instrument",
            LinkKind::Agent => "agent",
            LinkKind::Invocation => "invocation",
            LinkKind::OvertimeException => "overtime_exception",
            LinkKind::UndertimeException => "undertime_exception",
            LinkKind::Aggregation => "aggregation",
            LinkKind::Exhibition => "exhibition",
            LinkKind::Generalization => "generalization",
            LinkKind::Classification => "classification",
            LinkKind::Tagged => "tagged",
        }
    }

    pub fn from_name(name: &str) -> Option<LinkKind> {
        LinkKind::ALL.into_iter().find(|k| k.name() == name)
    }

    pub fn is_procedural(self) -> bool {
        !self.is_structural()
    }

    pub fn is_structural(self) -> bool {
        matches!(
            self,
            LinkKind::Aggregation
                | LinkKind::Exhibition
                | LinkKind::Generalization
                | LinkKind::Classification
                | LinkKind::Tagged
        )
    }

    /// Whether the kind takes a `modifier` (the ○ column of metamodel §6.2, rule C4).
    pub fn takes_modifier(self) -> bool {
        matches!(
            self,
            LinkKind::Consumption | LinkKind::Effect | LinkKind::Instrument | LinkKind::Agent
        )
    }

    /// Procedural links from an Object (or State) into a Process.
    pub fn is_enabling_or_consuming(self) -> bool {
        matches!(
            self,
            LinkKind::Consumption | LinkKind::Instrument | LinkKind::Agent
        )
    }

    /// Procedural links between two Processes.
    pub fn is_process_to_process(self) -> bool {
        matches!(
            self,
            LinkKind::Invocation | LinkKind::OvertimeException | LinkKind::UndertimeException
        )
    }
}

/// metamodel §6.1 modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Event,
    Condition,
}

/// A link (metamodel §6). Attributes that only some kinds take are optional here and are
/// checked by [`crate::check`] (C4 to C6), so that a model built in code can be checked too.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub id: String,
    pub kind: LinkKind,
    /// A Thing id, or a State id for procedural links that name a state.
    pub source: String,
    pub target: String,
    /// consumption, effect, instrument, agent.
    pub modifier: Option<Modifier>,
    /// effect only.
    pub from_state: Option<String>,
    /// effect only.
    pub to_state: Option<String>,
    /// aggregation only.
    pub ordered: bool,
    /// tagged only; required there.
    pub tag: Option<String>,
    /// tagged only.
    pub backward_tag: Option<String>,
    pub path: Option<String>,
    pub source_multiplicity: Option<String>,
    pub target_multiplicity: Option<String>,
    pub description: Option<String>,
    pub extensions: Extensions,
}

impl Link {
    pub fn new(
        id: impl Into<String>,
        kind: LinkKind,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Link {
            id: id.into(),
            kind,
            source: source.into(),
            target: target.into(),
            modifier: None,
            from_state: None,
            to_state: None,
            ordered: false,
            tag: None,
            backward_tag: None,
            path: None,
            source_multiplicity: None,
            target_multiplicity: None,
            description: None,
            extensions: Extensions::default(),
        }
    }

    /// The end that is on the Process side, used to group links in a Fan (C11):
    /// the target for links into a Process, the source otherwise.
    pub fn process_end(&self) -> &str {
        if self.kind.is_enabling_or_consuming() {
            &self.target
        } else {
            &self.source
        }
    }
}

/// metamodel §6.4 operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanOperator {
    Xor,
    Or,
}

/// A XOR / OR fan of links (metamodel §6.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Fan {
    pub id: String,
    pub operator: FanOperator,
    pub links: Vec<String>,
    pub description: Option<String>,
    pub extensions: Extensions,
}

/// The inside of a Thing (metamodel §7.1).
#[derive(Debug, Clone, PartialEq)]
pub struct InZoom {
    pub id: String,
    pub refinee: String,
    /// Process ids by step, in order. Members of one step run in parallel.
    pub steps: Vec<Vec<String>>,
    pub parts: Vec<String>,
    pub description: Option<String>,
    pub extensions: Extensions,
}

impl InZoom {
    /// The ids of all inner things, steps first.
    pub fn inner(&self) -> impl Iterator<Item = &String> {
        self.steps.iter().flatten().chain(self.parts.iter())
    }
}

/// metamodel §7.2 refinement. The root OPD is `Root`; every other one is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refinement {
    Root,
    InZoom,
    Unfold,
    View,
}

/// An OPD (metamodel §7.2). Children are nested, so the parent is the enclosing diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagram {
    pub id: String,
    pub name: String,
    pub refinement: Refinement,
    pub refinee: Option<String>,
    pub appearances: Vec<Appearance>,
    pub children: Vec<Diagram>,
    pub description: Option<String>,
    pub extensions: Extensions,
}

impl Diagram {
    pub fn new(id: impl Into<String>, name: impl Into<String>, refinement: Refinement) -> Self {
        Diagram {
            id: id.into(),
            name: name.into(),
            refinement,
            refinee: None,
            appearances: Vec::new(),
            children: Vec::new(),
            description: None,
            extensions: Extensions::default(),
        }
    }
}

/// A Thing shown on an OPD (metamodel §7.2).
#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    pub id: String,
    pub thing: String,
    pub suppressed_states: Vec<String>,
    pub layout: Option<Layout>,
    pub description: Option<String>,
    pub extensions: Extensions,
}

impl Appearance {
    pub fn new(id: impl Into<String>, thing: impl Into<String>) -> Self {
        Appearance {
            id: id.into(),
            thing: thing.into(),
            suppressed_states: Vec::new(),
            layout: None,
            description: None,
            extensions: Extensions::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub x: f64,
    pub y: f64,
    pub width: Option<f64>,
    pub height: Option<f64>,
}
