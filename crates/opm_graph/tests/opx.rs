// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Writing OPX (spec/opx.md).
//!
//! Every example, every conf of the four sample repositories and every System snapshot is
//! written as OPX, and the file is checked for what a reader of OPX needs (spec/opx.md §6):
//! every id it refers to exists, link ends are of the right kind, every line ends on a Thing
//! or State drawn on the same OPD with the ends of its logical link, and so on.
//! With `OPM_OPX_OUT=<dir>` the files are written there (`.opz`).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use opm_graph::opx::{to_opx, Options};
use opm_model::Model;
use roxmltree::{Document, Node};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The `.opl` / `.xml` model files under a directory (recursively).
fn models(dir: &str) -> Vec<PathBuf> {
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(p).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "opl" || e == "xml") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&repo().join(dir), &mut v);
    v.sort();
    v
}

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(name))
}

fn req<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Node<'a, 'i> {
    child(n, name).unwrap_or_else(|| panic!("<{}> has no <{}>", n.tag_name().name(), name))
}

fn num(n: Node, a: &str) -> u64 {
    n.attribute(a)
        .unwrap_or_else(|| panic!("<{}> has no {}", n.tag_name().name(), a))
        .parse()
        .unwrap_or_else(|_| panic!("<{}> {} is not a number", n.tag_name().name(), a))
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Object,
    State,
    Process,
    Link,
    Relation,
}

/// Check one OPX document; returns the number of OPDs.
fn check(xml: &str, what: &str) -> usize {
    let doc = Document::parse(xml).unwrap_or_else(|e| panic!("{}: not XML: {}", what, e));
    let root = doc.root_element();
    assert_eq!(root.tag_name().name(), "OPX");
    assert_eq!(root.attribute("version"), Some("3"));
    let sys = req(root, "OPMSystem");
    for a in ["author", "name", "creationDate"] {
        assert!(
            sys.attribute(a).is_some(),
            "{}: OPMSystem has no {}",
            what,
            a
        );
    }
    let props = req(sys, "SystemProperties");
    req(props, "SystemConfiguration");
    req(props, "GeneralInfo");

    // the logical part
    let logical = req(sys, "LogicalStructure");
    let mut kinds: HashMap<u64, Kind> = HashMap::new();
    let mut owner: HashMap<u64, u64> = HashMap::new();
    let mut ends: HashMap<u64, (u64, u64, u64)> = HashMap::new();
    let mut add = |id: u64, k: Kind| {
        assert!(kinds.insert(id, k).is_none(), "{}: id {} twice", what, id);
    };
    let entity = |n: Node| -> u64 {
        let e = req(n, "EntityAttr");
        assert!(e.attribute("environmental").is_some());
        req(e, "name");
        num(e, "id")
    };
    for o in req(logical, "ObjectSection")
        .children()
        .filter(|c| c.is_element())
    {
        let id = entity(o);
        add(id, Kind::Object);
        let t = req(o, "ThingAttr");
        num(t, "numberOfInstances");
        for s in o.children().filter(|c| c.has_tag_name("LogicalState")) {
            let sid = entity(s);
            add(sid, Kind::State);
            owner.insert(sid, id);
        }
    }
    for p in req(logical, "ProcessSection")
        .children()
        .filter(|c| c.is_element())
    {
        add(entity(p), Kind::Process);
        req(p, "ThingAttr");
    }
    let mut link_rows = Vec::new();
    for r in req(logical, "RelationSection")
        .children()
        .filter(|c| c.is_element())
    {
        let id = entity(r);
        add(id, Kind::Relation);
        let row = (
            num(r, "relationType"),
            num(r, "sourceId"),
            num(r, "destinationId"),
        );
        ends.insert(id, row);
        link_rows.push(row);
    }
    for l in req(logical, "LinkSection")
        .children()
        .filter(|c| c.is_element())
    {
        let id = entity(l);
        add(id, Kind::Link);
        let row = (
            num(l, "linkType"),
            num(l, "sourceId"),
            num(l, "destinationId"),
        );
        ends.insert(id, row);
        link_rows.push(row);
    }
    let max = num(sys, "maxEntityEntry");
    assert!(kinds.keys().all(|&k| k <= max), "{}: maxEntityEntry", what);
    // the ends of each type (docs/research/opcat-opx.md, metamodel §6.2)
    for (t, s, d) in &link_rows {
        let (s, d) = (kinds[s], kinds[d]);
        let thing_or_state = |k: Kind| matches!(k, Kind::Object | Kind::State);
        let ok = match t {
            301 | 303 | 304 | 305 | 308 | 310 => thing_or_state(s) && d == Kind::Process,
            302 => s == Kind::Object && d == Kind::Process,
            306 => s == Kind::Process && thing_or_state(d),
            307 | 309 => s == Kind::Process && d == Kind::Process,
            201 | 203 => s == d,
            204 => s == d && s != Kind::State,
            202 | 205 | 206 => s != Kind::State && d != Kind::State,
            _ => false,
        };
        assert!(ok, "{}: type {} from {:?} to {:?}", what, t, s, d);
    }

    // the OPDs
    let mut opd_ids = HashSet::new();
    fn opd(
        n: Node,
        root: bool,
        what: &str,
        kinds: &HashMap<u64, Kind>,
        owner: &HashMap<u64, u64>,
        ends: &HashMap<u64, (u64, u64, u64)>,
        opd_ids: &mut HashSet<u64>,
    ) {
        let id = num(n, "id");
        assert!(opd_ids.insert(id), "{}: OPD {} twice", what, id);
        assert_eq!(root, id == 1, "{}: the root OPD is 1", what);
        let name = n.attribute("name").unwrap();
        let what = format!("{} {}", what, name);
        // in-OPD id -> entity id, and the box (absolute)
        let mut inst: HashMap<u64, u64> = HashMap::new();
        let mut add = |in_opd: u64, entity: u64| {
            assert!(
                kinds.contains_key(&entity),
                "{}: no entity {}",
                what,
                entity
            );
            assert!(
                inst.insert(in_opd, entity).is_none(),
                "{}: in-OPD id {} twice",
                what,
                in_opd
            );
        };
        fn thing(
            vt: Node,
            what: &str,
            bounds: Option<(i64, i64)>,
            kinds: &HashMap<u64, Kind>,
            owner: &HashMap<u64, u64>,
            add: &mut dyn FnMut(u64, u64),
        ) {
            let data = req(vt, "ThingData");
            let v = child(data, "VisualProcess")
                .or_else(|| child(data, "VisualObject"))
                .unwrap();
            let ia = req(v, "InstanceAttr");
            let e = num(ia, "entityId");
            let want = if v.has_tag_name("VisualProcess") {
                Kind::Process
            } else {
                Kind::Object
            };
            assert_eq!(kinds.get(&e), Some(&want), "{}: entity {}", what, e);
            add(num(ia, "entityInOpdId"), e);
            let ce = req(v, "ConnectionEdgeAttr");
            let (x, y, w, h) = (
                num(ce, "x") as i64,
                num(ce, "y") as i64,
                num(ce, "width") as i64,
                num(ce, "height") as i64,
            );
            assert!(w > 0 && h > 0, "{}: entity {} has no size", what, e);
            if let Some((pw, ph)) = bounds {
                assert!(
                    x + w <= pw && y + h <= ph,
                    "{}: entity {} sticks out",
                    what,
                    e
                );
            }
            req(v, "ThingInstanceAttr");
            for s in v.children().filter(|c| c.has_tag_name("VisualState")) {
                let ia = req(s, "InstanceAttr");
                let se = num(ia, "entityId");
                assert_eq!(owner.get(&se), Some(&e), "{}: state {} of {}", what, se, e);
                add(num(ia, "entityInOpdId"), se);
                req(s, "ConnectionEdgeAttr");
            }
            for c in req(vt, "ChildrenContainer")
                .children()
                .filter(|c| c.has_tag_name("VisualThing"))
            {
                thing(c, what, Some((w, h)), kinds, owner, add);
            }
        }
        if !root {
            let m = req(n, "MainEntity");
            thing(req(m, "VisualThing"), &what, None, kinds, owner, &mut add);
        }
        for vt in req(n, "ThingSection").children().filter(|c| c.is_element()) {
            thing(vt, &what, None, kinds, owner, &mut add);
        }
        let line = |entity: u64, la: Node| {
            let (_, s, d) = ends[&entity];
            assert_eq!(num(la, "sourceId"), s, "{}: source of {}", what, entity);
            assert_eq!(
                num(la, "destinationId"),
                d,
                "{}: destination of {}",
                what,
                entity
            );
            assert_eq!(
                inst.get(&num(la, "sourceInOpdId")),
                Some(&s),
                "{}: {}",
                what,
                entity
            );
            assert_eq!(
                inst.get(&num(la, "destinationInOpdId")),
                Some(&d),
                "{}: {}",
                what,
                entity
            );
            num(la, "sourceConnectionSide");
            num(la, "destinationConnectionSide");
        };
        let mut lines: HashMap<u64, u64> = HashMap::new();
        let mut visual = |ia: Node, k: Kind| -> u64 {
            let e = num(ia, "entityId");
            assert_eq!(kinds.get(&e), Some(&k), "{}: entity {}", what, e);
            assert!(lines.insert(num(ia, "entityInOpdId"), e).is_none());
            e
        };
        for cp in req(n, "FundamentalRelationSection")
            .children()
            .filter(|c| c.is_element())
        {
            let s = num(cp, "sourceId");
            assert_eq!(inst.get(&num(cp, "sourceInOpdId")), Some(&s), "{}", what);
            for vf in cp.children().filter(|c| c.is_element()) {
                let e = visual(req(vf, "InstanceAttr"), Kind::Relation);
                let (_, ls, ld) = ends[&e];
                assert_eq!((ls, ld), (s, num(vf, "destinationId")), "{}: {}", what, e);
                assert_eq!(
                    inst.get(&num(vf, "destinationInOpdId")),
                    Some(&ld),
                    "{}",
                    what
                );
            }
        }
        for g in req(n, "GeneralRelationSection")
            .children()
            .filter(|c| c.is_element())
        {
            let e = visual(req(g, "InstanceAttr"), Kind::Relation);
            line(e, req(g, "LineAttr"));
        }
        let section = req(n, "VisualLinkSection");
        for l in section.children().filter(|c| c.has_tag_name("VisualLink")) {
            let e = visual(req(l, "InstanceAttr"), Kind::Link);
            line(e, req(l, "LineAttr"));
        }
        for g in section.children().filter(|c| c.has_tag_name("OrXorGroup")) {
            for m in g.children().filter(|c| c.is_element()) {
                assert_eq!(
                    lines.get(&num(m, "memberInOpdId")),
                    Some(&num(m, "memberId")),
                    "{}: fan member",
                    what
                );
            }
        }
        let max = num(n, "maxEntityEntry");
        assert!(
            inst.keys().chain(lines.keys()).all(|&k| k <= max),
            "{}: maxEntityEntry",
            what
        );
        for c in req(n, "InZoomed").children().filter(|c| c.is_element()) {
            opd(c, false, &what, kinds, owner, ends, opd_ids);
        }
        for (section, props) in [
            ("Unfolded", "UnfoldingProperties"),
            ("View", "ViewProperties"),
        ] {
            for p in req(n, section).children().filter(|c| c.has_tag_name(props)) {
                assert!(
                    kinds.contains_key(&num(p, "entityId")),
                    "{}: {}",
                    what,
                    props
                );
                opd(req(p, "OPD"), false, &what, kinds, owner, ends, opd_ids);
            }
        }
    }
    let vp = req(sys, "VisualPart");
    let root_opd = req(vp, "OPD");
    opd(root_opd, true, what, &kinds, &owner, &ends, &mut opd_ids);
    assert!(opd_ids.iter().all(|&o| o <= num(sys, "maxOpdEntry")));
    opd_ids.len()
}

fn read(p: &Path) -> Model {
    opm_graph::read_model(p).unwrap_or_else(|e| panic!("{}", e))
}

fn write_out(p: &Path, xml: &str) {
    if let Ok(dir) = std::env::var("OPM_OPX_OUT") {
        let dir = Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        let name = p
            .file_name()
            .unwrap()
            .to_string_lossy()
            .replace(".opm.xml", "")
            .replace(".opl", "")
            .replace(".xml", "");
        std::fs::write(dir.join(format!("{}.opz", name)), opm_graph::opx::gzip(xml)).unwrap();
    }
}

#[test]
fn examples_write_sound_opx() {
    for p in models("examples/xml") {
        let model = read(&p);
        let opx = to_opx(&model, &Options::default());
        let opds = check(&opx.xml, &p.display().to_string());
        let want = match model.diagram {
            Some(_) => model.diagrams().len(),
            None => 1 + model.in_zooms.len(),
        };
        assert_eq!(opds, want, "{}", p.display());
        write_out(&p, &opx.xml);
    }
}

#[test]
fn juiz_models_write_sound_opx() {
    // confs and System snapshots written as juiz-OPL; they have no OPD
    let mut n = 0;
    for dir in ["tests/conf", "tests/system"] {
        for p in models(dir)
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "opl"))
        {
            let model = read(&p);
            let opx = to_opx(&model, &Options::default());
            let opds = check(&opx.xml, &p.display().to_string());
            assert_eq!(opds, 1 + model.in_zooms.len(), "{}", p.display());
            write_out(&p, &opx.xml);
            n += 1;
        }
    }
    assert!(n > 50);
}

#[test]
fn every_link_is_drawn_somewhere() {
    // A model without OPDs: every link shows on the SD or on an in-zoomed OPD.
    for p in models("tests/conf").into_iter().take(20) {
        let model = read(&p);
        let opx = to_opx(&model, &Options::default());
        let doc = Document::parse(&opx.xml).unwrap();
        let logical: HashSet<&str> = doc
            .descendants()
            .filter(|n| n.has_tag_name("LogicalLink") || n.has_tag_name("LogicalRelation"))
            .map(|n| req(n, "EntityAttr").attribute("id").unwrap())
            .collect();
        let drawn: HashSet<&str> = doc
            .descendants()
            .filter(|n| n.has_tag_name("InstanceAttr"))
            .filter_map(|n| n.attribute("entityId"))
            .collect();
        let missing: Vec<_> = logical.difference(&drawn).collect();
        assert!(
            missing.is_empty(),
            "{}: not drawn {:?}",
            p.display(),
            missing
        );
    }
}

#[test]
fn mapping_of_kinds() {
    let text = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0" format_version="0.1" id="m" name="M">
  <opm:things>
    <opm:process id="p" name="Making"/>
    <opm:process id="q" name="Serving"/>
    <opm:object id="w" name="Water">
      <opm:state id="cold" name="cold"/><opm:state id="hot" name="hot"/>
    </opm:object>
    <opm:object id="b" name="Beans"/>
    <opm:object id="c" name="Coffee"/>
    <opm:object id="m1" name="Machine"/>
  </opm:things>
  <opm:links>
    <opm:effect id="e1" source="p" target="w" from_state="cold" to_state="hot"/>
    <opm:consumption id="c1" source="b" target="p" modifier="event"/>
    <opm:result id="r1" source="p" target="c"/>
    <opm:instrument id="i1" source="m1" target="p" modifier="condition"/>
    <opm:effect id="e2" source="q" target="c"/>
    <opm:invocation id="v1" source="p" target="q"/>
    <opm:undertime_exception id="x1" source="p" target="q"/>
    <opm:aggregation id="a1" source="m1" target="b" ordered="true"/>
    <opm:tagged id="t1" source="c" target="m1" tag="comes from" backward_tag="makes"/>
  </opm:links>
</opm:model>"#;
    let model = opm_model::read::from_str(text).unwrap();
    let opx = to_opx(&model, &Options::default());
    check(&opx.xml, "mapping");
    let doc = Document::parse(&opx.xml).unwrap();
    let types: Vec<(String, String)> = doc
        .descendants()
        .filter(|n| n.has_tag_name("LogicalLink") || n.has_tag_name("LogicalRelation"))
        .map(|n| {
            (
                n.attribute("linkType")
                    .or(n.attribute("relationType"))
                    .unwrap()
                    .to_string(),
                req(req(n, "EntityAttr"), "name")
                    .text()
                    .unwrap()
                    .to_string(),
            )
        })
        .collect();
    let t: Vec<&str> = types.iter().map(|t| t.0.as_str()).collect();
    // relations first, then links, each in model order; e1 is two links
    assert_eq!(
        t,
        ["204", "206", "301", "306", "310", "306", "304", "302", "307", "309"]
    );
    assert_eq!(
        opx.notes,
        [
            "overtime and undertime exception links are both OPCAT's exception link",
            "the order of an aggregation is not written",
        ]
    );
    assert!(opx
        .xml
        .contains("forwardRelationMeaning=\"comes from\" backwardRelationMeaning=\"makes\""));
}
