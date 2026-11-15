// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! What one picture shows (spec/diagram.md §2): the Things, States and links of one OPD, or of
//! the whole model when it has no OPD.
//!
//! A link is shown when both its ends are shown and the OPD does not suppress it. An end that
//! is a State shows on its owner; a suppressed State moves the end to the owner. An end that is
//! not shown but lies inside an in-zoomed Thing that is shown moves to that Thing (the summed-up
//! link of the higher OPD, metamodel §7.1). Such a link is marked [`ShownLink::derived`], and
//! links that end up the same are drawn once.

use std::collections::{HashMap, HashSet};

use opm_model::{Diagram, InZoom, Link, Model, Refinement, Thing};

/// One end of a shown link.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct End {
    /// The Thing drawn at this end.
    pub thing: String,
    /// The State of that Thing, when the link names one that is shown.
    pub state: Option<String>,
}

/// A link as shown on one picture.
#[derive(Debug, Clone)]
pub struct ShownLink<'a> {
    pub link: &'a Link,
    pub source: End,
    pub target: End,
    /// Effect only: the shown `from_state` / `to_state`.
    pub from_state: Option<String>,
    pub to_state: Option<String>,
    /// An end was moved from an inner Thing to the in-zoomed Thing that holds it.
    pub derived: bool,
}

/// An in-zoomed Thing drawn with its inside (spec/diagram.md §3.8 for juiz, OPM otherwise).
#[derive(Debug, Clone)]
pub struct Zoom<'a> {
    pub in_zoom: &'a InZoom,
    /// The inner Things that are shown, steps first, in order.
    pub inner: Vec<&'a str>,
}

/// What one picture shows.
#[derive(Debug, Clone)]
pub struct View<'a> {
    pub model: &'a Model,
    /// The OPD, or `None` for the whole model.
    pub diagram: Option<&'a Diagram>,
    /// Things shown, in model order.
    pub things: Vec<&'a Thing>,
    /// States not shown (suppressed on the Appearance).
    pub hidden_states: HashSet<&'a str>,
    pub links: Vec<ShownLink<'a>>,
    /// In-zoomed Things shown with at least one inner Thing.
    pub zooms: Vec<Zoom<'a>>,
}

impl<'a> View<'a> {
    /// The picture name: the OPD name, or the model name.
    pub fn name(&self) -> String {
        match self.diagram {
            Some(d) => d.name.clone(),
            None => self
                .model
                .name
                .clone()
                .unwrap_or_else(|| self.model.id.clone()),
        }
    }

    pub fn shows(&self, thing: &str) -> bool {
        self.things.iter().any(|t| t.id == thing)
    }

    /// The zoom whose inside holds this Thing directly.
    pub fn zoom_holding(&self, thing: &str) -> Option<&Zoom<'a>> {
        self.zooms.iter().find(|z| z.inner.contains(&thing))
    }

    pub fn zoom_of(&self, refinee: &str) -> Option<&Zoom<'a>> {
        self.zooms.iter().find(|z| z.in_zoom.refinee == refinee)
    }
}

/// One view for each OPD, root first, or one view of the whole model when there is no OPD.
pub fn views(model: &Model) -> Vec<View<'_>> {
    if model.diagram.is_none() {
        return vec![view(model, None)];
    }
    model
        .diagrams()
        .into_iter()
        .map(|d| view(model, Some(d)))
        .collect()
}

/// The view of one OPD, or of the whole model.
pub fn view<'a>(model: &'a Model, diagram: Option<&'a Diagram>) -> View<'a> {
    let shown: HashSet<&str> = match diagram {
        None => model.things.iter().map(|t| t.id.as_str()).collect(),
        Some(d) => {
            let mut s: HashSet<&str> = d.appearances.iter().map(|a| a.thing.as_str()).collect();
            // An in-zoom OPD shows its refinee even when it forgot the Appearance.
            if d.refinement == Refinement::InZoom {
                if let Some(r) = &d.refinee {
                    s.insert(r.as_str());
                }
            }
            s
        }
    };
    let things: Vec<&Thing> = model
        .things
        .iter()
        .filter(|t| shown.contains(t.id.as_str()))
        .collect();
    let hidden_states: HashSet<&str> = diagram
        .map(|d| {
            d.appearances
                .iter()
                .flat_map(|a| a.suppressed_states.iter().map(String::as_str))
                .collect()
        })
        .unwrap_or_default();
    let suppressed: HashSet<&str> = diagram
        .map(|d| d.suppressed_links.iter().map(String::as_str).collect())
        .unwrap_or_default();

    // inner Thing -> the Thing it is inside of
    let mut parent: HashMap<&str, &str> = HashMap::new();
    for z in &model.in_zooms {
        for i in z.inner() {
            parent.insert(i.as_str(), z.refinee.as_str());
        }
    }

    let zooms: Vec<Zoom> = model
        .in_zooms
        .iter()
        .filter(|z| shown.contains(z.refinee.as_str()))
        .map(|z| Zoom {
            in_zoom: z,
            inner: z
                .inner()
                .map(String::as_str)
                .filter(|i| shown.contains(i))
                .collect(),
        })
        .filter(|z| !z.inner.is_empty())
        .collect();

    // Where an end is drawn: (end, moved to an enclosing Thing).
    let resolve = |id: &str| -> Option<(End, bool)> {
        let (thing, state) = match model.state(id) {
            Some((owner, s)) => (owner.id.as_str(), Some(s.id.as_str())),
            None => (model.thing(id)?.id.as_str(), None),
        };
        if shown.contains(thing) {
            let state = state.filter(|s| !hidden_states.contains(s));
            return Some((
                End {
                    thing: thing.to_string(),
                    state: state.map(str::to_string),
                },
                false,
            ));
        }
        let mut cur = thing;
        while let Some(&up) = parent.get(cur) {
            if shown.contains(up) {
                return Some((
                    End {
                        thing: up.to_string(),
                        state: None,
                    },
                    true,
                ));
            }
            cur = up;
        }
        None
    };

    let mut links = Vec::new();
    let mut seen_derived = HashSet::new();
    for l in &model.links {
        if suppressed.contains(l.id.as_str()) {
            continue;
        }
        let (Some((source, ds)), Some((target, dt))) = (resolve(&l.source), resolve(&l.target))
        else {
            continue;
        };
        let derived = ds || dt;
        if derived {
            // A link inside one in-zoomed Thing does not show outside it.
            if source.thing == target.thing && l.source != l.target {
                continue;
            }
            let key = (l.kind, source.clone(), target.clone(), l.modifier.is_some());
            if !seen_derived.insert(key) {
                continue;
            }
        }
        let state_end = |s: &Option<String>| {
            s.as_deref()
                .filter(|s| !derived && !hidden_states.contains(s))
                .map(str::to_string)
        };
        links.push(ShownLink {
            link: l,
            source,
            target,
            from_state: state_end(&l.from_state),
            to_state: state_end(&l.to_state),
            derived,
        });
    }

    View {
        model,
        diagram,
        things,
        hidden_states,
        links,
        zooms,
    }
}
