// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The symbols of the abbreviated diagram (spec/diagram.md §3) for the examples.

use std::path::{Path, PathBuf};

use opm_graph::abbrev::{self, EdgeKind, EndRef, Note, Scene, Shape};
use opm_graph::view;
use opm_model::Model;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn example(name: &str) -> Model {
    opm_graph::read_model(&repo().join("examples/xml").join(name)).unwrap()
}

fn scenes(m: &Model) -> Vec<Scene> {
    view::views(m).iter().map(abbrev::build).collect()
}

fn has_dot() -> bool {
    std::process::Command::new(opm_graph::svg::dot_command())
        .arg("-V")
        .output()
        .is_ok()
}

fn edges(s: &Scene, kind: EdgeKind) -> Vec<&abbrev::Edge> {
    s.edges.iter().filter(|e| e.kind == kind).collect()
}

#[test]
fn talker_listener() {
    let m = example("talker_listener.opm.xml");
    let s = &scenes(&m)[0];
    // the System is a frame around the EC and both processes
    let f = s.frame("talker_listener").unwrap();
    assert_eq!(f.members, ["ec0", "listener0", "talker0"]);
    // the cycle and the values are drawn by the symbols, not on their own
    let ids: Vec<&str> = s.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["ec0", "listener0", "talker0"]);
    // the EC invokes the talker from its started state
    let ec = edges(s, EdgeKind::EcInvokes);
    assert_eq!(ec.len(), 1);
    assert_eq!(
        ec[0].from,
        EndRef::State("ec0".into(), "ec0.started".into())
    );
    assert_eq!(ec[0].to, EndRef::Thing("talker0".into()));
    assert_eq!(ec[0].label.as_deref(), Some("periodically invokes · 1 Hz"));
    // the push connection runs from the return port to the argument port
    let push = edges(s, EdgeKind::Push);
    assert_eq!(push.len(), 1);
    assert_eq!(push[0].from, EndRef::Out("talker0".into()));
    assert_eq!(push[0].to, EndRef::Port("listener0".into(), 0));
    assert_eq!(push[0].label.as_deref(), Some("push (con0)"));
    // nothing is left in the OPM form (the invocation of the push is in the connection)
    assert!(s.edges.iter().all(|e| !matches!(e.kind, EdgeKind::Opm(_))));
    let Shape::Process(p) = &s.node("listener0").unwrap().shape else {
        panic!()
    };
    assert!(p.args[0].connected);
}

#[test]
fn tutorial() {
    let m = example("tutorial.opm.xml");
    let ss = scenes(&m);
    let sd = &ss[0];
    let Shape::Container {
        name,
        detail,
        procs,
    } = &sd.node("counter0").unwrap().shape
    else {
        panic!("counter0 is a container")
    };
    assert_eq!(name, "counter0");
    assert_eq!(detail, r#"example_container_python · {"initial_value":3}"#);
    let marks: Vec<_> = procs.iter().map(|p| (p.name.as_str(), p.mark)).collect();
    assert_eq!(marks, [("increment0", Some("✎")), ("get0", Some("👁"))]);
    assert_eq!(procs[0].args[0].note, Some(Note::Default("1".into())));
    let Shape::Process(inc0) = &sd.node("inc0").unwrap().shape else {
        panic!()
    };
    assert!(!inc0.args[0].connected);
    assert_eq!(inc0.args[0].note, Some(Note::Default("0".into())));
    assert!(inc0.out.is_some());
    // the call to inc0 is suppressed on SD
    let calls: Vec<_> = edges(sd, EdgeKind::Call)
        .iter()
        .map(|e| e.to.thing().to_string())
        .collect();
    assert_eq!(calls, ["increment0", "get0"]);
    let Shape::Process(client) = &sd.node("client").unwrap().shape else {
        panic!()
    };
    assert!(client.dashed);

    // SD1 draws the container with its processes only, without ports
    let sd1 = &ss[1];
    assert_eq!(sd1.nodes.len(), 1);
    assert!(sd1.frames.is_empty());
    let Shape::Container { procs, .. } = &sd1.nodes[0].shape else {
        panic!()
    };
    assert_eq!(procs.len(), 2);
    assert!(procs.iter().all(|p| p.args.is_empty() && p.out.is_none()));
}

#[test]
fn capture_filter_applied() {
    let m = example("capture_filter.opm.xml");
    let s = &scenes(&m)[0];
    let Shape::Process(f) = &s
        .nodes
        .iter()
        .find(|n| matches!(&n.shape, Shape::Process(p) if p.args.len() == 2))
        .unwrap()
        .shape
    else {
        panic!()
    };
    assert!(f.args[0].connected);
    assert!(matches!(&f.args[1].note, Some(Note::Applied(v)) if v.contains("COLOR_BGR2GRAY")));
    assert_eq!(f.out.as_deref(), Some("Image"));
}

#[test]
fn composite_in_zoom() {
    let m = example("composite.opm.xml");
    let ss = scenes(&m);
    // SD: the composite with the ports of its inner nodes
    let Shape::Process(c) = &ss[0].node("blur_gray").unwrap().shape else {
        panic!()
    };
    assert_eq!(c.args.len(), 1);
    assert!(c.out.is_some());
    // SD1: a frame with the nodes, the const as a note, the intermediate as a line
    let f = ss[1].frame("blur_gray").unwrap();
    assert!(f.zoom);
    assert_eq!(f.members, ["n_cvt", "n_blur"]);
    assert_eq!(ss[1].order, [("n_cvt".to_string(), "n_blur".to_string())]);
    let pull = edges(&ss[1], EdgeKind::Pull);
    assert_eq!(pull.len(), 1);
    assert_eq!(pull[0].from, EndRef::Out("n_cvt".into()));
    let Shape::Process(cvt) = &ss[1].node("n_cvt").unwrap().shape else {
        panic!()
    };
    assert_eq!(
        cvt.args[1].note,
        Some(Note::Default("\"COLOR_BGR2GRAY\"".into()))
    );
}

const TRANSFORMS: &str = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
    xmlns:juiz="https://openjuiz.github.io/ns/juiz/v0" format_version="0.1" id="m" name="tf">
  <opm:things>
    <opm:object id="map" name="map" juiz:role="container" juiz:type="periodic_anchor" juiz:rate="30"/>
    <opm:object id="base0" name="base0" juiz:role="container" juiz:type="mobile_base"/>
    <opm:object id="tf" name="map to base0" juiz:role="transform">
      <juiz:translation x="0.1" z="0.3"/>
    </opm:object>
    <opm:object id="pose" name="base0 pose" juiz:role="pose"/>
    <opm:process id="view" name="map_view0" juiz:role="process" juiz:type="map_view"/>
    <opm:object id="topic" name="odom" juiz:role="topic"/>
    <opm:process id="pub" name="odom_pub0" juiz:role="process" juiz:type="odom_pub"/>
  </opm:things>
  <opm:links>
    <opm:tagged id="t1" source="tf" target="map" tag="from" juiz:end="source"/>
    <opm:tagged id="t2" source="tf" target="base0" tag="to" juiz:end="destination"/>
    <opm:exhibition id="e1" source="base0" target="pose"/>
    <opm:instrument id="i1" source="pose" target="view" modifier="event"
        juiz:arg="pose" juiz:arg_index="0" juiz:reference="map" juiz:initial="true"/>
    <opm:result id="r1" source="pub" target="topic"/>
    <opm:instrument id="i2" source="topic" target="view" modifier="event"
        juiz:arg="odom" juiz:arg_index="1"/>
  </opm:links>
</opm:model>"#;

#[test]
fn transforms_poses_and_topics() {
    let m = opm_model::read::from_str(TRANSFORMS).unwrap();
    let s = &scenes(&m)[0];
    let ids: Vec<&str> = s.nodes.iter().map(|n| n.id.as_str()).collect();
    assert_eq!(ids, ["map", "base0", "view", "topic", "pub"]);
    let Shape::Container { name, detail, .. } = &s.node("map").unwrap().shape else {
        panic!()
    };
    assert_eq!(
        (name.as_str(), detail.as_str()),
        ("⚓ map", "periodic_anchor · 30 Hz")
    );
    let tf = edges(s, EdgeKind::Transform);
    assert_eq!(tf.len(), 1);
    assert_eq!((tf[0].from.thing(), tf[0].to.thing()), ("map", "base0"));
    assert_eq!(tf[0].label.as_deref(), Some("(0.1, 0, 0.3)"));
    let pose = edges(s, EdgeKind::Pose);
    assert_eq!(pose[0].from, EndRef::Thing("base0".into()));
    assert_eq!(pose[0].to, EndRef::Port("view".into(), 0));
    assert_eq!(pose[0].label.as_deref(), Some("pose · ref: map · initial"));
    assert_eq!(
        edges(s, EdgeKind::Publish)[0].from,
        EndRef::Out("pub".into())
    );
    assert_eq!(
        edges(s, EdgeKind::Subscribe)[0].to,
        EndRef::Port("view".into(), 1)
    );
    assert!(s.edges.iter().all(|e| !matches!(e.kind, EdgeKind::Opm(_))));
    if has_dot() {
        let v = view::views(&m);
        let svg = opm_graph::abbreviated_svg(&v[0]).unwrap();
        assert!(svg.contains("#2b5fa8"));
    }
}

#[test]
fn every_example_draws_abbreviated() {
    if !has_dot() {
        eprintln!("Graphviz `dot` not found: nothing is drawn");
        return;
    }
    let out = std::env::var_os("OPM_GRAPH_OUT").map(PathBuf::from);
    for dir in ["examples/xml", "examples/opl"] {
        for e in std::fs::read_dir(repo().join(dir)).unwrap() {
            let path = e.unwrap().path();
            let m = opm_graph::read_model(&path).unwrap();
            for d in opm_graph::draw_svg(&m).unwrap() {
                assert!(d.svg.contains("<svg"), "{}", path.display());
                if let Some(o) = &out {
                    std::fs::create_dir_all(o).unwrap();
                    let stem = path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace('.', "_");
                    std::fs::write(o.join(format!("{}.{}.abbrev.svg", stem, d.name)), &d.svg)
                        .unwrap();
                }
            }
        }
    }
}
