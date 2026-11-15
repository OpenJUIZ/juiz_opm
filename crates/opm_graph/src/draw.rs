// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The SVG of a placed [`Scene`] (spec/diagram.md §3).

use std::collections::HashMap;
use std::fmt::Write;

use opm_model::LinkKind;

use crate::abbrev::{Edge, EdgeKind, EndRef, Head, Node, Note, Proc, Scene, Shape, StateSym};
use crate::dot::Status;
use crate::layout::{Placement, Sizes};

const FONT: &str = "Helvetica, Arial, sans-serif";
/// Port radius.
const PR: f64 = 5.0;
const PORT_STEP: f64 = 26.0;
const BLUE: &str = "#2b5fa8";
const MARGIN_X: f64 = 20.0;
const MARGIN_TOP: f64 = 44.0;

/// Text width estimate: ASCII is narrow, the rest (kana, kanji, symbols) is square.
pub fn tw(s: &str, size: impl Into<f64>) -> f64 {
    s.chars()
        .map(|c| if (c as u32) < 128 { 0.6 } else { 1.0 })
        .sum::<f64>()
        * size.into()
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn note_text(n: &Note) -> String {
    match n {
        Note::Default(v) => format!("= {}", v),
        Note::Applied(v) => format!("← {} (p_apply)", v),
    }
}

fn proc_name(p: &Proc) -> String {
    match p.mark {
        Some(m) => format!("{} {}", m, p.name),
        None => p.name.clone(),
    }
}

/// (ellipse width, ellipse height, room for notes on the left).
fn proc_size(p: &Proc) -> (f64, f64, f64) {
    let ports = p
        .args
        .iter()
        .map(|a| tw(&a.name, 9) * 2.0)
        .fold(0.0, f64::max);
    let w = tw(&proc_name(p), 13)
        .max(tw(p.sub.as_deref().unwrap_or(""), 9))
        .max(ports)
        * 1.25
        + 60.0;
    let h = (PORT_STEP * p.args.len() as f64 + 22.0).max(50.0);
    let notes = p
        .args
        .iter()
        .filter_map(|a| a.note.as_ref())
        .map(|n| tw(&note_text(n), 9) + 22.0)
        .fold(0.0, f64::max);
    (w, h, notes)
}

fn states_width(states: &[StateSym]) -> f64 {
    states.iter().map(|s| tw(&s.name, 11) + 26.0).sum::<f64>() + 10.0 * (states.len() as f64 + 1.0)
}

/// Room on the right of an ellipse for the return port and its type.
fn out_size(p: &Proc) -> f64 {
    match &p.out {
        Some(l) => tw(l, 9) + 14.0,
        None => 0.0,
    }
}

/// The size of a node for the layout: (width, height), notes included.
pub fn size(n: &Node) -> (f64, f64) {
    match &n.shape {
        Shape::Process(p) => {
            let (w, h, notes) = proc_size(p);
            (w + notes + out_size(p), h)
        }
        Shape::Container {
            name,
            detail,
            procs,
        } => {
            let head = (tw(name, 13) * 1.1).max(tw(detail, 9)) + 30.0;
            let rows: Vec<_> = procs
                .iter()
                .map(|p| {
                    let (w, h, n) = proc_size(p);
                    (w + out_size(p), h, n)
                })
                .collect();
            let w = rows
                .iter()
                .map(|(w, _, n)| w + n + 30.0)
                .fold(head, f64::max);
            let h = if rows.is_empty() {
                48.0
            } else {
                46.0 + rows.iter().map(|(_, h, _)| h + 12.0).sum::<f64>() + 6.0
            };
            (w, h)
        }
        Shape::Ec {
            name,
            detail,
            states,
        } => {
            let w = (tw(name, 13) + 20.0)
                .max(tw(detail, 9))
                .max(states_width(states))
                + 30.0;
            (w, if states.is_empty() { 50.0 } else { 84.0 })
        }
        Shape::Object {
            name, sub, states, ..
        } => {
            let w = tw(name, 13)
                .max(tw(sub.as_deref().unwrap_or(""), 9))
                .max(states_width(states))
                + 36.0;
            let h = match (states.is_empty(), sub.is_some()) {
                (true, false) => 40.0,
                (true, true) => 50.0,
                (false, false) => 72.0,
                (false, true) => 84.0,
            };
            (w, h)
        }
    }
}

pub fn sizes(scene: &Scene) -> Sizes {
    scene
        .nodes
        .iter()
        .map(|n| (n.id.clone(), size(n)))
        .collect()
}

/// The label Graphviz reserves room for at the top of a frame.
pub fn frame_label(scene: &Scene, id: &str) -> String {
    let f = scene.frame(id).expect("frame");
    match &f.sub {
        Some(s) => format!("{}  {}", f.label, s),
        None => f.label.clone(),
    }
}

#[derive(Debug, Clone, Copy)]
enum Geo {
    Ellipse(f64, f64, f64, f64),
    Rect(f64, f64, f64, f64),
}

impl Geo {
    fn center(self) -> (f64, f64) {
        match self {
            Geo::Ellipse(cx, cy, ..) => (cx, cy),
            Geo::Rect(x0, y0, x1, y1) => ((x0 + x1) / 2.0, (y0 + y1) / 2.0),
        }
    }

    /// The point on the outline facing `to`, and the outward direction there.
    fn toward(self, to: (f64, f64)) -> ((f64, f64), (f64, f64)) {
        let (cx, cy) = self.center();
        let (dx, dy) = (to.0 - cx, to.1 - cy);
        let (hw, hh) = match self {
            Geo::Ellipse(_, _, rx, ry) => (rx, ry),
            Geo::Rect(x0, y0, x1, y1) => ((x1 - x0) / 2.0, (y1 - y0) / 2.0),
        };
        if dx.abs() / hw.max(1.0) >= dy.abs() / hh.max(1.0) {
            let s = dx.signum();
            ((cx + s * hw, cy), (s, 0.0))
        } else {
            let s = dy.signum();
            ((cx, cy + s * hh), (0.0, s))
        }
    }
}

#[derive(Default)]
struct Svg {
    frames: String,
    shapes: String,
    /// Backgrounds of edge labels, under the lines.
    label_bg: String,
    edges: String,
    text: String,
    geo: HashMap<String, Geo>,
    ports: HashMap<(String, usize), (f64, f64)>,
    outs: HashMap<String, (f64, f64)>,
    states: HashMap<(String, String), Geo>,
}

#[allow(clippy::too_many_arguments)]
fn text(
    out: &mut String,
    x: f64,
    y: f64,
    s: &str,
    size: f64,
    color: &str,
    bold: bool,
    anchor: &str,
) {
    let _ = writeln!(
        out,
        r#"<text x="{x:.1}" y="{y:.1}" font-size="{size}" fill="{color}"{w} text-anchor="{anchor}" dominant-baseline="middle" font-family="{FONT}">{s}</text>"#,
        w = if bold { r#" font-weight="bold""# } else { "" },
        s = esc(s)
    );
}

fn fill(status: Status) -> &'static str {
    status.fill().unwrap_or("#ffffff")
}

/// Draw a placed scene.
pub fn to_svg(scene: &Scene, place: &Placement) -> String {
    let mut s = Svg::default();
    let ox = MARGIN_X;
    let oy = MARGIN_TOP;
    // frames, outermost first (bigger first)
    let mut frames: Vec<_> = scene
        .frames
        .iter()
        .filter_map(|f| place.frames.get(&f.id).map(|b| (f, *b)))
        .collect();
    frames.sort_by(|a, b| {
        let area = |r: (f64, f64, f64, f64)| (r.2 - r.0) * (r.3 - r.1);
        area(b.1).total_cmp(&area(a.1))
    });
    for (f, (x0, y0, x1, y1)) in frames {
        let (x0, y0, x1, y1) = (x0 + ox, y0 + oy, x1 + ox, y1 + oy);
        let (stroke, rx, width) = if f.zoom {
            ("#111", 26.0, 1.3)
        } else {
            ("#999", 12.0, 1.2)
        };
        let _ = writeln!(
            s.frames,
            r##"<rect x="{x0:.1}" y="{y0:.1}" width="{:.1}" height="{:.1}" rx="{rx}" fill="{}" stroke="{stroke}" stroke-width="{width}"/>"##,
            x1 - x0,
            y1 - y0,
            f.status.fill().unwrap_or("none")
        );
        let lx = x0 + if f.zoom { 20.0 } else { 12.0 };
        text(
            &mut s.frames,
            lx,
            y0 + 14.0,
            &f.label,
            12.0,
            f.status.text(),
            true,
            "start",
        );
        if let Some(sub) = &f.sub {
            text(
                &mut s.frames,
                lx + tw(&f.label, 12.0) * 0.95 + 8.0,
                y0 + 14.0,
                sub,
                10.0,
                "#888",
                false,
                "start",
            );
        }
        s.geo.insert(f.id.clone(), Geo::Rect(x0, y0, x1, y1));
    }

    for n in &scene.nodes {
        let Some(&(x, y)) = place.nodes.get(&n.id) else {
            continue;
        };
        let (x, y) = (x + ox, y + oy);
        let (w, h) = size(n);
        match &n.shape {
            Shape::Process(p) => {
                let (pw, _, notes) = proc_size(p);
                let cx = x - w / 2.0 + notes + pw / 2.0;
                s.process(p, cx, y, n.status);
            }
            Shape::Container {
                name,
                detail,
                procs,
            } => {
                let (x0, y0) = (x - w / 2.0, y - h / 2.0);
                let _ = writeln!(
                    s.shapes,
                    r##"<rect x="{x0:.1}" y="{y0:.1}" width="{w:.1}" height="{h:.1}" fill="{}" stroke="#111" stroke-width="2.2"/>"##,
                    fill(n.status)
                );
                let _ = writeln!(
                    s.shapes,
                    r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="#e8eef7"/>"##,
                    x0 + 1.1,
                    y0 + 1.1,
                    w - 2.2,
                    (h - 2.2).min(40.0)
                );
                text(
                    &mut s.text,
                    x,
                    y0 + 14.0,
                    name,
                    13.0,
                    n.status.text(),
                    true,
                    "middle",
                );
                text(
                    &mut s.text,
                    x,
                    y0 + 30.0,
                    detail,
                    9.0,
                    "#555",
                    false,
                    "middle",
                );
                s.geo
                    .insert(n.id.clone(), Geo::Rect(x0, y0, x0 + w, y0 + h));
                let mut yy = y0 + 46.0;
                for p in procs {
                    let (pw, ph, notes) = proc_size(p);
                    let cx = x0 + (w - pw - notes - out_size(p)) / 2.0 + notes + pw / 2.0;
                    s.process(p, cx, yy + ph / 2.0 + 6.0, p.status);
                    yy += ph + 12.0;
                }
            }
            Shape::Ec {
                name,
                detail,
                states,
            } => {
                let (x0, y0) = (x - w / 2.0, y - h / 2.0);
                let _ = writeln!(
                    s.shapes,
                    r##"<rect x="{x0:.1}" y="{y0:.1}" width="{w:.1}" height="{h:.1}" rx="10" fill="{}" stroke="#111" stroke-width="1.3"/>"##,
                    fill(n.status)
                );
                if states.is_empty() {
                    text(
                        &mut s.text,
                        x,
                        y - 8.0,
                        &format!("⏱ {}", name),
                        13.0,
                        n.status.text(),
                        true,
                        "middle",
                    );
                    text(
                        &mut s.text,
                        x,
                        y + 11.0,
                        detail,
                        9.0,
                        "#555",
                        false,
                        "middle",
                    );
                } else {
                    text(
                        &mut s.text,
                        x,
                        y0 + 14.0,
                        name,
                        13.0,
                        n.status.text(),
                        true,
                        "middle",
                    );
                    text(
                        &mut s.text,
                        x,
                        y0 + 30.0,
                        detail,
                        9.0,
                        "#555",
                        false,
                        "middle",
                    );
                    s.state_row(&n.id, states, x, y0 + h - 34.0);
                }
                s.geo
                    .insert(n.id.clone(), Geo::Rect(x0, y0, x0 + w, y0 + h));
            }
            Shape::Object {
                name,
                sub,
                states,
                physical,
                environmental,
            } => {
                let (x0, y0) = (x - w / 2.0, y - h / 2.0);
                let _ = writeln!(
                    s.shapes,
                    r##"<rect x="{x0:.1}" y="{y0:.1}" width="{w:.1}" height="{h:.1}" fill="{}" stroke="#111" stroke-width="{}"{}/>"##,
                    fill(n.status),
                    if *physical { 2.4 } else { 1.2 },
                    if *environmental {
                        r#" stroke-dasharray="5,4""#
                    } else {
                        ""
                    }
                );
                let mut ty = if states.is_empty() { y } else { y0 + 16.0 };
                if sub.is_some() {
                    ty -= 7.0;
                }
                text(
                    &mut s.text,
                    x,
                    ty,
                    name,
                    13.0,
                    n.status.text(),
                    false,
                    "middle",
                );
                if let Some(sub) = sub {
                    text(&mut s.text, x, ty + 16.0, sub, 9.0, "#777", false, "middle");
                }
                if !states.is_empty() {
                    s.state_row(&n.id, states, x, y0 + h - 34.0);
                }
                s.geo
                    .insert(n.id.clone(), Geo::Rect(x0, y0, x0 + w, y0 + h));
            }
        }
    }

    for e in &scene.edges {
        s.edge(e);
    }

    let width = place.width + 2.0 * MARGIN_X;
    let height = place.height + MARGIN_TOP + 26.0;
    let mut o = String::new();
    let _ = writeln!(
        o,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.0}" height="{height:.0}" viewBox="0 0 {width:.0} {height:.0}">"#
    );
    o.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#fff\"/>\n");
    text(
        &mut o,
        width / 2.0,
        20.0,
        &scene.title,
        14.0,
        "#111",
        true,
        "middle",
    );
    o.push_str(&s.frames);
    o.push_str(&s.shapes);
    o.push_str(&s.label_bg);
    o.push_str(&s.edges);
    o.push_str(&s.text);
    o.push_str("</svg>\n");
    o
}

impl Svg {
    fn process(&mut self, p: &Proc, cx: f64, cy: f64, status: Status) {
        let (w, h, _) = proc_size(p);
        let (rx, ry) = (w / 2.0, h / 2.0);
        let mut extra = String::new();
        if p.dashed {
            extra.push_str(r#" stroke-dasharray="5,4""#);
        }
        let _ = writeln!(
            self.shapes,
            r##"<ellipse cx="{cx:.1}" cy="{cy:.1}" rx="{rx:.1}" ry="{ry:.1}" fill="{}" stroke="#111" stroke-width="{}"{extra}/>"##,
            fill(status),
            if p.physical { 2.4 } else { 1.3 }
        );
        let name = proc_name(p);
        let ty = if p.sub.is_some() { cy - 7.0 } else { cy };
        text(
            &mut self.text,
            cx,
            ty,
            &name,
            13.0,
            status.text(),
            false,
            "middle",
        );
        if let Some(sub) = &p.sub {
            text(
                &mut self.text,
                cx,
                cy + 10.0,
                sub,
                9.0,
                "#777",
                false,
                "middle",
            );
        }
        let n = p.args.len();
        for (i, a) in p.args.iter().enumerate() {
            let dy = (i as f64 - (n as f64 - 1.0) / 2.0) * PORT_STEP;
            let px = cx - rx * (1.0 - (dy / ry).powi(2)).max(0.0).sqrt();
            let py = cy + dy;
            self.ports.insert((p.id.clone(), i), (px, py));
            let _ = writeln!(
                self.text,
                r##"<circle cx="{px:.1}" cy="{py:.1}" r="{PR}" fill="{}" stroke="#111" stroke-width="1.2"/>"##,
                if a.connected { "#111" } else { "#fff" }
            );
            text(
                &mut self.text,
                px + 9.0,
                py,
                &a.name,
                9.0,
                "#444",
                false,
                "start",
            );
            if let Some(note) = &a.note {
                let (nx, ny) = (px - 14.0, py + if dy >= 0.0 { 16.0 } else { -16.0 });
                let _ = writeln!(
                    self.shapes,
                    r##"<path d="M{:.1},{:.1} L{nx:.1},{ny:.1}" stroke="#999" stroke-width="0.8" fill="none"/>"##,
                    px - 3.0,
                    py + if dy >= 0.0 { 3.0 } else { -3.0 }
                );
                let color = match note {
                    Note::Applied(_) => "#8a5a00",
                    Note::Default(_) => "#666",
                };
                text(
                    &mut self.text,
                    nx - 2.0,
                    ny + 2.0,
                    &note_text(note),
                    9.0,
                    color,
                    false,
                    "end",
                );
            }
        }
        if let Some(label) = &p.out {
            let (px, py) = (cx + rx, cy);
            self.outs.insert(p.id.clone(), (px + PR + 1.0, py));
            let _ = writeln!(
                self.text,
                r##"<path d="M{:.1},{:.1} L{:.1},{py:.1} L{:.1},{:.1} Z" fill="#111"/>"##,
                px - PR,
                py - PR,
                px + PR + 1.0,
                px - PR,
                py + PR
            );
            if !label.is_empty() {
                text(
                    &mut self.text,
                    px + 6.0,
                    py - 12.0,
                    label,
                    9.0,
                    "#444",
                    false,
                    "start",
                );
            }
        }
        self.geo.insert(p.id.clone(), Geo::Ellipse(cx, cy, rx, ry));
    }

    fn state_row(&mut self, owner: &str, states: &[StateSym], cx: f64, y0: f64) {
        let total = states_width(states);
        let mut x = cx - total / 2.0 + 10.0;
        for st in states {
            let sw = tw(&st.name, 11) + 26.0;
            let _ = writeln!(
                self.shapes,
                r##"<rect x="{x:.1}" y="{y0:.1}" width="{sw:.1}" height="24" rx="10" fill="#fff" stroke="#111" stroke-width="{}"/>"##,
                if st.initial { 2.2 } else { 1.0 }
            );
            if st.final_ {
                let _ = writeln!(
                    self.shapes,
                    r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="18" rx="8" fill="none" stroke="#111" stroke-width="0.8"/>"##,
                    x + 3.0,
                    y0 + 3.0,
                    sw - 6.0
                );
            }
            let label = if st.default {
                format!("{} ↙", st.name)
            } else {
                st.name.clone()
            };
            text(
                &mut self.text,
                x + sw / 2.0,
                y0 + 12.0,
                &label,
                11.0,
                "#111",
                false,
                "middle",
            );
            self.states.insert(
                (owner.to_string(), st.id.clone()),
                Geo::Rect(x, y0, x + sw, y0 + 24.0),
            );
            x += sw + 10.0;
        }
    }

    /// Centre of an end, to aim the other end at.
    fn center(&self, e: &EndRef) -> Option<(f64, f64)> {
        Some(match e {
            EndRef::Thing(t) => self.geo.get(t)?.center(),
            EndRef::Port(t, i) => *self.ports.get(&(t.clone(), *i))?,
            EndRef::Out(t) => match self.outs.get(t) {
                Some(&p) => p,
                None => self.geo.get(t)?.center(),
            },
            EndRef::State(t, s) => self
                .states
                .get(&(t.clone(), s.clone()))
                .or_else(|| self.geo.get(t))?
                .center(),
        })
    }

    /// The point of an end, and the direction the line leaves it in.
    fn point(&self, e: &EndRef, other: (f64, f64)) -> Option<((f64, f64), (f64, f64))> {
        Some(match e {
            EndRef::Thing(t) => {
                let g = *self.geo.get(t)?;
                let (p, n) = g.toward(other);
                match g {
                    // keep clear of the input ports on the left of a Process
                    Geo::Ellipse(cx, cy, rx, ry)
                        if n.0 < 0.0 && self.ports.contains_key(&(t.clone(), 0)) =>
                    {
                        // the upper left, above the ports
                        let _ = other;
                        ((cx - rx * 0.6, cy - ry * 0.8), (-0.6, -0.8))
                    }
                    _ => (p, n),
                }
            }
            EndRef::Port(t, i) => {
                let (x, y) = *self.ports.get(&(t.clone(), *i))?;
                ((x - PR, y), (-1.0, 0.0))
            }
            EndRef::Out(t) => match self.outs.get(t) {
                Some(&p) => (p, (1.0, 0.0)),
                None => self.geo.get(t)?.toward(other),
            },
            EndRef::State(t, s) => self
                .states
                .get(&(t.clone(), s.clone()))
                .or_else(|| self.geo.get(t))?
                .toward(other),
        })
    }

    fn edge(&mut self, e: &Edge) {
        let (Some(c0), Some(c1)) = (self.center(&e.from), self.center(&e.to)) else {
            return;
        };
        let (Some((p0, n0)), Some((p3, n3))) = (self.point(&e.from, c1), self.point(&e.to, c0))
        else {
            return;
        };
        let color = match e.kind {
            EdgeKind::Transform if e.status == Status::Released => BLUE,
            _ => e.status.line(),
        };
        let (width, dashed) = match e.kind {
            EdgeKind::Push => (2.6, false),
            EdgeKind::Transform => (3.2, false),
            EdgeKind::EcInvokes | EdgeKind::Call | EdgeKind::Pose | EdgeKind::Subscribe => {
                (1.2, true)
            }
            _ => (1.2, false),
        };
        let dash = if dashed {
            r#" stroke-dasharray="6,4""#
        } else {
            ""
        };

        // a line from a Thing to itself: a loop above it
        if e.from == e.to {
            let y = p0.1;
            let (cx, cy) = c0;
            let top = match self.geo.get(e.from.thing()) {
                Some(Geo::Ellipse(_, cy, _, ry)) => cy - ry,
                Some(Geo::Rect(_, y0, ..)) => *y0,
                None => y,
            };
            let (a, b) = ((cx + 18.0, top + 2.0), (cx - 18.0, top + 2.0));
            let _ = writeln!(
                self.edges,
                r#"<path d="M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{color}" stroke-width="{width}"{dash}/>"#,
                a.0,
                a.1,
                a.0 + 20.0,
                a.1 - 40.0,
                b.0 - 20.0,
                b.1 - 40.0,
                b.0,
                b.1
            );
            self.head(e.head, b, (b.0 - 20.0, b.1 - 40.0), color);
            if let Some(l) = &e.label {
                self.label(cx, top - 36.0, l, color);
            }
            let _ = cy;
            return;
        }

        let d = (((p3.0 - p0.0).powi(2) + (p3.1 - p0.1).powi(2)).sqrt() / 2.5).max(30.0);
        // end the line where the head starts
        let back = match e.head {
            Head::Arrow => 8.0,
            Head::WhiteDot | Head::BlackDot => 9.0,
            _ => 0.0,
        };
        let front = match e.tail {
            Head::Arrow => 8.0,
            Head::Triangle(_) => 13.0,
            Head::WhiteDot | Head::BlackDot => 9.0,
            _ => 0.0,
        };
        let q0 = (p0.0 + n0.0 * front, p0.1 + n0.1 * front);
        let q3 = (p3.0 + n3.0 * back, p3.1 + n3.1 * back);
        let c1p = (q0.0 + n0.0 * d, q0.1 + n0.1 * d);
        let c2p = (q3.0 + n3.0 * d, q3.1 + n3.1 * d);
        let _ = writeln!(
            self.edges,
            r#"<path d="M{:.1},{:.1} C{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="none" stroke="{color}" stroke-width="{width}"{dash}/>"#,
            q0.0, q0.1, c1p.0, c1p.1, c2p.0, c2p.1, q3.0, q3.1
        );
        self.head(e.head, p3, q3, color);
        self.head(e.tail, p0, q0, color);

        let bez = |t: f64| {
            let u = 1.0 - t;
            let f = |a: f64, b: f64, c: f64, d: f64| {
                u * u * u * a + 3.0 * u * u * t * b + 3.0 * u * t * t * c + t * t * t * d
            };
            (f(q0.0, c1p.0, c2p.0, q3.0), f(q0.1, c1p.1, c2p.1, q3.1))
        };
        let (mx, my) = bez(0.5);
        if e.kind == EdgeKind::Transform {
            let (ax, ay) = bez(0.47);
            let (bx, by) = bez(0.53);
            let a = (by - ay).atan2(bx - ax);
            let (l, w) = (16.0, 9.0);
            let tip = (mx + l / 2.0 * a.cos(), my + l / 2.0 * a.sin());
            let b1 = (
                mx - l / 2.0 * a.cos() - w * a.sin(),
                my - l / 2.0 * a.sin() + w * a.cos(),
            );
            let b2 = (
                mx - l / 2.0 * a.cos() + w * a.sin(),
                my - l / 2.0 * a.sin() - w * a.cos(),
            );
            let _ = writeln!(
                self.edges,
                r##"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} Z" fill="#fff" stroke="{color}" stroke-width="1.6"/>"##,
                tip.0, tip.1, b1.0, b1.1, b2.0, b2.1
            );
        }
        if let Some(l) = &e.label {
            let dy = if e.kind == EdgeKind::Transform {
                16.0
            } else {
                9.0
            };
            self.label(mx, my - dy, l, color);
        }
    }

    fn label(&mut self, x: f64, y: f64, s: &str, color: &str) {
        let w = tw(s, 9.0);
        let _ = writeln!(
            self.label_bg,
            r##"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="12" fill="#fff" opacity="0.85"/>"##,
            x - w / 2.0 - 3.0,
            y - 6.0,
            w + 6.0
        );
        text(&mut self.text, x, y, s, 9.0, color, false, "middle");
    }

    /// A mark whose point is at `at`, coming from `from`.
    fn head(&mut self, h: Head, at: (f64, f64), from: (f64, f64), color: &str) {
        let a = (at.1 - from.1).atan2(at.0 - from.0);
        let (c, s) = (a.cos(), a.sin());
        let pt = |back: f64, side: f64| (at.0 - back * c - side * s, at.1 - back * s + side * c);
        let poly = |pts: &[(f64, f64)], f: &str| {
            let d: Vec<String> = pts
                .iter()
                .map(|p| format!("{:.1},{:.1}", p.0, p.1))
                .collect();
            format!(
                r#"<path d="M{} Z" fill="{f}" stroke="{color}" stroke-width="1.2"/>"#,
                d.join(" L")
            )
        };
        let out = match h {
            Head::None => return,
            Head::Arrow => poly(&[at, pt(8.0, 4.0), pt(8.0, -4.0)], color),
            Head::OpenArrow => {
                let (a1, a2) = (pt(9.0, 5.0), pt(9.0, -5.0));
                format!(
                    r#"<path d="M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1}" fill="none" stroke="{color}" stroke-width="1.2"/>"#,
                    a1.0, a1.1, at.0, at.1, a2.0, a2.1
                )
            }
            Head::WhiteDot | Head::BlackDot => {
                let (x, y) = pt(4.5, 0.0);
                format!(
                    r#"<circle cx="{x:.1}" cy="{y:.1}" r="4.5" fill="{}" stroke="{color}" stroke-width="1.2"/>"#,
                    if h == Head::BlackDot { color } else { "#fff" }
                )
            }
            Head::Triangle(k) => {
                // the apex touches the whole
                let tri = [at, pt(13.0, 7.0), pt(13.0, -7.0)];
                let mut o = poly(
                    &tri,
                    if k == LinkKind::Aggregation {
                        color
                    } else {
                        "#fff"
                    },
                );
                match k {
                    LinkKind::Exhibition => {
                        o.push_str(&poly(&[pt(5.0, 0.0), pt(10.5, 3.0), pt(10.5, -3.0)], color))
                    }
                    LinkKind::Classification => {
                        let (x, y) = pt(9.0, 0.0);
                        let _ = write!(
                            o,
                            r#"<circle cx="{x:.1}" cy="{y:.1}" r="2.2" fill="{color}"/>"#
                        );
                    }
                    _ => {}
                }
                o
            }
        };
        self.edges.push_str(&out);
        self.edges.push('\n');
    }
}
