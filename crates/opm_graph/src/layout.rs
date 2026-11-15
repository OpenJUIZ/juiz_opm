// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Placing a [`Scene`]: Graphviz `dot` places boxes of the symbols' sizes (`dot -Tjson`), and
//! the symbols are then drawn at those places by [`crate::draw`].

use std::collections::HashMap;
use std::fmt::Write;

use crate::abbrev::{EndRef, Scene};

/// Where things are, in points, with y growing downwards.
#[derive(Debug, Clone, Default)]
pub struct Placement {
    /// Node id → centre.
    pub nodes: HashMap<String, (f64, f64)>,
    /// Frame id → (x0, y0, x1, y1).
    pub frames: HashMap<String, (f64, f64, f64, f64)>,
    pub width: f64,
    pub height: f64,
}

/// The size of each node, as the drawing measures it: (width, height) in points.
pub type Sizes = HashMap<String, (f64, f64)>;

fn q(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The DOT given to Graphviz to place the scene: fixed-size empty boxes, one cluster per
/// frame, one edge per line (for the ranks), and the step order of InZooms.
pub fn layout_dot(scene: &Scene, sizes: &Sizes, frame_label: &dyn Fn(&str) -> String) -> String {
    let mut o = String::new();
    o.push_str("digraph g {\n  graph [rankdir=LR, nodesep=0.5, ranksep=0.9, fontname=\"Helvetica\"];\n  node [shape=box, fixedsize=true, label=\"\"];\n");
    // frames that a line starts or ends at get a point to aim at
    let aimed: Vec<&str> = scene
        .edges
        .iter()
        .flat_map(|e| [e.from.thing(), e.to.thing()])
        .filter(|t| scene.frame(t).is_some())
        .collect();
    let nested: Vec<&str> = scene
        .frames
        .iter()
        .flat_map(|f| f.members.iter().map(String::as_str))
        .collect();
    #[allow(clippy::too_many_arguments)]
    fn frame(
        o: &mut String,
        scene: &Scene,
        sizes: &Sizes,
        id: &str,
        depth: usize,
        frame_label: &dyn Fn(&str) -> String,
        aimed: &[&str],
    ) {
        let pad = "  ".repeat(depth);
        let f = scene.frame(id).expect("frame");
        let i = scene.frames.iter().position(|g| g.id == id).unwrap();
        let _ = writeln!(
            o,
            "{}subgraph cluster_{} {{ label={}; fontsize=12; labeljust=l; margin=18;",
            pad,
            i,
            q(&frame_label(id))
        );
        if aimed.contains(&id) {
            let _ = writeln!(o, "{}  {} [shape=point, width=0.01];", pad, q(&anchor(id)));
        }
        for m in &f.members {
            if scene.frame(m).is_some() {
                frame(o, scene, sizes, m, depth + 1, frame_label, aimed);
            } else {
                node(o, sizes, m, depth + 1);
            }
        }
        let _ = writeln!(o, "{}}}", pad);
    }
    fn node(o: &mut String, sizes: &Sizes, id: &str, depth: usize) {
        if let Some((w, h)) = sizes.get(id) {
            let _ = writeln!(
                o,
                "{}{} [width={:.3}, height={:.3}];",
                "  ".repeat(depth),
                q(id),
                w / 72.0,
                (h + 24.0) / 72.0
            );
        }
    }
    for f in &scene.frames {
        if !nested.contains(&f.id.as_str()) {
            frame(&mut o, scene, sizes, &f.id, 1, frame_label, &aimed);
        }
    }
    for n in &scene.nodes {
        if !nested.contains(&n.id.as_str()) {
            node(&mut o, sizes, &n.id, 1);
        }
    }
    let lid = |e: &EndRef| -> Option<String> {
        let t = e.thing();
        if scene.frame(t).is_some() {
            return Some(anchor(t));
        }
        scene.holder(t).map(|n| n.id.clone())
    };
    for e in &scene.edges {
        let (Some(a), Some(b)) = (lid(&e.from), lid(&e.to)) else {
            continue;
        };
        if a != b {
            // the label takes room between the ranks, as it will in the drawing
            match &e.label {
                Some(l) => {
                    let _ = writeln!(o, "  {} -> {} [label={}, fontsize=9];", q(&a), q(&b), q(l));
                }
                None => {
                    let _ = writeln!(o, "  {} -> {};", q(&a), q(&b));
                }
            }
        }
    }
    for (a, b) in &scene.order {
        if sizes.contains_key(a) && sizes.contains_key(b) {
            let _ = writeln!(o, "  {} -> {} [style=invis, weight=5];", q(a), q(b));
        }
    }
    o.push_str("}\n");
    o
}

fn anchor(frame: &str) -> String {
    format!("frame:{}", frame)
}

/// Run `dot -Tjson` on the layout DOT and read the places back.
pub fn place(dot: &str, scene: &Scene) -> Result<Placement, String> {
    let out = crate::svg::render(dot, "json")?;
    let text = String::from_utf8_lossy(&out);
    let v = Json::parse(&text)?;
    let bb = |s: &str| -> Option<(f64, f64, f64, f64)> {
        let n: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (n.len() == 4).then(|| (n[0], n[1], n[2], n[3]))
    };
    let (_, _, w, h) = v
        .get("bb")
        .and_then(Json::as_str)
        .and_then(bb)
        .ok_or("dot -Tjson: no bb")?;
    let mut p = Placement {
        width: w,
        height: h,
        ..Placement::default()
    };
    for o in v.get("objects").and_then(Json::as_array).unwrap_or(&[]) {
        let Some(name) = o.get("name").and_then(Json::as_str) else {
            continue;
        };
        if let Some(c) = name.strip_prefix("cluster_") {
            if let (Some(b), Ok(i)) = (
                o.get("bb").and_then(Json::as_str).and_then(bb),
                c.parse::<usize>(),
            ) {
                // y grows downwards in the drawing
                if let Some(f) = scene.frames.get(i) {
                    p.frames.insert(f.id.clone(), (b.0, h - b.3, b.2, h - b.1));
                }
            }
        } else if let Some(pos) = o.get("pos").and_then(Json::as_str) {
            let n: Vec<f64> = pos.split(',').filter_map(|x| x.parse().ok()).collect();
            if n.len() == 2 {
                p.nodes.insert(name.to_string(), (n[0], h - n[1]));
            }
        }
    }
    Ok(p)
}

/// Just enough JSON to read `dot -Tjson`.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(s: &str) -> Result<Json, String> {
        let b = s.as_bytes();
        let mut i = 0;
        let v = value(b, &mut i)?;
        ws(b, &mut i);
        if i != b.len() {
            return Err(format!("json: trailing text at {}", i));
        }
        Ok(v)
    }

    pub fn get(&self, k: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Arr(a) => Some(a),
            _ => None,
        }
    }
}

fn ws(b: &[u8], i: &mut usize) {
    while *i < b.len() && b[*i].is_ascii_whitespace() {
        *i += 1;
    }
}

fn value(b: &[u8], i: &mut usize) -> Result<Json, String> {
    ws(b, i);
    let err = |i: usize| format!("json: unexpected input at {}", i);
    match b.get(*i) {
        Some(b'{') => {
            *i += 1;
            let mut m = Vec::new();
            loop {
                ws(b, i);
                if b.get(*i) == Some(&b'}') {
                    *i += 1;
                    return Ok(Json::Obj(m));
                }
                let k = match value(b, i)? {
                    Json::Str(k) => k,
                    _ => return Err(err(*i)),
                };
                ws(b, i);
                if b.get(*i) != Some(&b':') {
                    return Err(err(*i));
                }
                *i += 1;
                m.push((k, value(b, i)?));
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b'}') => {}
                    _ => return Err(err(*i)),
                }
            }
        }
        Some(b'[') => {
            *i += 1;
            let mut a = Vec::new();
            loop {
                ws(b, i);
                if b.get(*i) == Some(&b']') {
                    *i += 1;
                    return Ok(Json::Arr(a));
                }
                a.push(value(b, i)?);
                ws(b, i);
                match b.get(*i) {
                    Some(b',') => *i += 1,
                    Some(b']') => {}
                    _ => return Err(err(*i)),
                }
            }
        }
        Some(b'"') => {
            *i += 1;
            let mut s = Vec::new();
            while let Some(&c) = b.get(*i) {
                *i += 1;
                match c {
                    b'"' => return Ok(Json::Str(String::from_utf8_lossy(&s).into_owned())),
                    b'\\' => {
                        let e = *b.get(*i).ok_or_else(|| err(*i))?;
                        *i += 1;
                        match e {
                            b'n' => s.push(b'\n'),
                            b't' => s.push(b'\t'),
                            b'r' => s.push(b'\r'),
                            b'b' => s.push(8),
                            b'f' => s.push(12),
                            b'u' => {
                                let h =
                                    std::str::from_utf8(b.get(*i..*i + 4).ok_or_else(|| err(*i))?)
                                        .map_err(|_| err(*i))?;
                                let c = u32::from_str_radix(h, 16).map_err(|_| err(*i))?;
                                *i += 4;
                                let ch = char::from_u32(c).unwrap_or('\u{fffd}');
                                let mut buf = [0; 4];
                                s.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                            }
                            c => s.push(c),
                        }
                    }
                    c => s.push(c),
                }
            }
            Err(err(*i))
        }
        Some(b't') if b[*i..].starts_with(b"true") => {
            *i += 4;
            Ok(Json::Bool(true))
        }
        Some(b'f') if b[*i..].starts_with(b"false") => {
            *i += 5;
            Ok(Json::Bool(false))
        }
        Some(b'n') if b[*i..].starts_with(b"null") => {
            *i += 4;
            Ok(Json::Null)
        }
        Some(c) if c.is_ascii_digit() || *c == b'-' => {
            let st = *i;
            while *i < b.len() && matches!(b[*i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                *i += 1;
            }
            std::str::from_utf8(&b[st..*i])
                .ok()
                .and_then(|s| s.parse().ok())
                .map(Json::Num)
                .ok_or_else(|| err(st))
        }
        _ => Err(err(*i)),
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn json() {
        let v = Json::parse(r#"{"a": [1, -2.5e1, "x\"yé"], "b": {"c": true, "d": null}}"#).unwrap();
        assert_eq!(
            v.get("a").unwrap().as_array().unwrap()[2].as_str(),
            Some("x\"yé")
        );
        assert_eq!(v.get("b").unwrap().get("c"), Some(&Json::Bool(true)));
        assert!(Json::parse("{").is_err());
    }
}
