// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Drawing the examples.
//!
//! - Every example (XML and OPL) draws, and the DOT is accepted by Graphviz when `dot` is on
//!   the PATH (CI installs it). With `OPM_GRAPH_OUT=<dir>` the SVGs are written there.
//! - What each OPD of the examples shows follows spec/diagram.md §2.

use std::path::{Path, PathBuf};

use opm_graph::view::{self, View};
use opm_model::{LinkKind, Model};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &str, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(repo().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(ext))
        .collect();
    v.sort();
    assert!(!v.is_empty(), "no {} files in {}", ext, dir);
    v
}

fn example(name: &str) -> Model {
    opm_graph::read_model(&repo().join("examples/xml").join(name)).unwrap()
}

fn has_dot() -> bool {
    std::process::Command::new(opm_graph::svg::dot_command())
        .arg("-V")
        .output()
        .is_ok()
}

#[test]
fn every_example_draws() {
    let out = std::env::var_os("OPM_GRAPH_OUT").map(PathBuf::from);
    let dot = has_dot();
    if !dot {
        eprintln!("Graphviz `dot` not found: the DOT is not rendered");
    }
    let mut paths = files("examples/xml", ".opm.xml");
    paths.extend(files("examples/opl", ".opl"));
    for path in paths {
        let model = opm_graph::read_model(&path).unwrap();
        let pictures = opm_graph::draw(&model);
        assert!(!pictures.is_empty());
        for p in &pictures {
            assert!(p.dot.starts_with("digraph "), "{}", path.display());
            if dot {
                let svg = opm_graph::svg::to_svg(&p.dot)
                    .unwrap_or_else(|e| panic!("{} {}: {}", path.display(), p.name, e));
                assert!(svg.contains("<svg"));
                if let Some(dir) = &out {
                    std::fs::create_dir_all(dir).unwrap();
                    let stem = path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .replace('.', "_");
                    std::fs::write(dir.join(format!("{}.{}.svg", stem, p.name)), svg).unwrap();
                }
            }
        }
    }
}

fn names<'a>(v: &View<'a>) -> Vec<&'a str> {
    v.things.iter().map(|t| t.id.as_str()).collect()
}

fn link<'a, 'v>(v: &'v View<'a>, id: &str) -> Option<&'v view::ShownLink<'a>> {
    v.links.iter().find(|l| l.link.id == id)
}

#[test]
fn without_opd_everything_is_drawn() {
    let m = example("talker_listener.opm.xml");
    let vs = view::views(&m);
    assert_eq!(vs.len(), 1);
    assert_eq!(vs[0].name(), "talker_listener");
    assert_eq!(vs[0].things.len(), m.things.len());
    assert_eq!(vs[0].links.len(), m.links.len());
    // the condition from a State starts at its port
    let dot = opm_graph::draw(&m).remove(0).dot;
    assert!(
        dot.contains("t0:s1 -> t1 [arrowhead=odot, label=\"c\"]"),
        "{}",
        dot
    );
}

#[test]
fn suppressed_links_and_appearances() {
    let m = example("tutorial.opm.xml");
    let vs = view::views(&m);
    assert_eq!(
        vs.iter().map(|v| v.name()).collect::<Vec<_>>(),
        ["SD", "SD1"]
    );
    // SD: the call from Client to inc0 is suppressed, the other calls are drawn
    assert!(link(&vs[0], "l_client_inc0").is_none());
    assert!(link(&vs[0], "l_client_get0").is_some());
    // SD1 draws counter0 and its container processes only, and the links among them
    assert_eq!(names(&vs[1]), ["counter0", "increment0", "get0"]);
    assert!(vs[1]
        .links
        .iter()
        .all(
            |l| ["counter0", "increment0", "get0"].contains(&l.source.thing.as_str())
                && ["counter0", "increment0", "get0"].contains(&l.target.thing.as_str())
        ));
    assert_eq!(vs[1].links.len(), 2);
}

#[test]
fn in_zoom_and_derived_links() {
    let m = example("coffee_making.opm.xml");
    let vs = view::views(&m);
    let (sd, sd1, sd2) = (&vs[0], &vs[1], &vs[2]);

    // SD: the links of the inner processes show on Coffee Making
    assert!(sd.zooms.is_empty());
    let barista = link(sd, "l_barista").unwrap();
    assert!(barista.derived);
    assert_eq!(barista.target.thing, "making");
    // the state change moves up as an effect without states
    let heat = link(sd, "l_heat").unwrap();
    assert_eq!(heat.source.thing, "making");
    assert_eq!(
        (heat.from_state.as_ref(), heat.to_state.as_ref()),
        (None, None)
    );
    // Ground Coffee is not on SD: neither are its links
    assert!(link(sd, "l_ground").is_none());

    // SD1: Coffee Making is drawn with its inside, top down
    assert_eq!(sd1.zooms.len(), 1);
    assert_eq!(sd1.zooms[0].inner, ["heating", "grinding", "brewing"]);
    assert!(sd1.links.iter().all(|l| !l.derived));
    let heat = link(sd1, "l_heat").unwrap();
    assert_eq!(heat.from_state.as_deref(), Some("water_cold"));
    let dot = opm_graph::draw(&m).remove(1).dot;
    assert!(dot.contains("rankdir=TB"));
    assert!(dot.contains("subgraph cluster_z"));
    assert!(dot.contains("XOR"));

    // SD2: cold is suppressed, the tagged link is drawn
    assert!(sd2.hidden_states.contains("water_cold"));
    let tagged = sd2
        .links
        .iter()
        .find(|l| l.link.kind == LinkKind::Tagged)
        .unwrap();
    assert_eq!(tagged.link.tag.as_deref(), Some("is located in"));
    let dot = opm_graph::draw(&m).remove(2).dot;
    assert!(!dot.contains(">cold<"));
    assert!(dot.contains(">hot<"));
}

#[test]
fn status_colors() {
    let text = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
        xmlns:juiz="https://openjuiz.github.io/ns/juiz/v0" format_version="0.1" id="m">
      <opm:things>
        <opm:process id="p" name="P" juiz:status="in_progress" juiz:refs="juiz#12"/>
        <opm:object id="o" name="O" juiz:status="planned"/>
      </opm:things>
      <opm:links>
        <opm:result id="l" source="p" target="o" juiz:status="defect" juiz:refs="juiz#3"/>
      </opm:links>
    </opm:model>"#;
    let m = opm_model::read::from_str(text).unwrap();
    let dot = opm_graph::draw(&m).remove(0).dot;
    assert!(dot.contains("fillcolor=\"#fff3c4\""), "{}", dot);
    assert!(dot.contains("juiz#12"));
    assert!(dot.contains("fillcolor=\"#eeeeee\""));
    assert!(dot.contains("color=\"#c83232\""));
    assert!(dot.contains("xlabel=\"juiz#3\""));
}

#[test]
fn opl_and_xml_draw_alike() {
    // The OPL examples have no OPDs, so they match the XML examples drawn without theirs.
    for path in files("examples/opl", ".opl") {
        let from_opl = opm_graph::read_model(&path).unwrap();
        let xml = repo().join("examples/xml").join(
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .replace(".opl", ".opm.xml"),
        );
        let mut from_xml = opm_graph::read_model(&xml).unwrap();
        from_xml.diagram = None;
        let a = view::views(&from_opl);
        let b = view::views(&from_xml);
        assert_eq!(a[0].things.len(), b[0].things.len(), "{}", path.display());
        assert_eq!(a[0].links.len(), b[0].links.len(), "{}", path.display());
    }
}
