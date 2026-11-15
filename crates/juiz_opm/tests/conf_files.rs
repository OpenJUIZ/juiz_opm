// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Tests on the confs of the four sample repositories (spec/conf.md §7) and the juiz rules.
//!
//! - tests/conf/*/*.conf: the round trips of [`juiz_opm::round_trip`] hold, and the OPL
//!   written from the conf is the `.opl` next to it (`JUIZ_OPM_BLESS=1` rewrites them).
//! - tests/conf/*/*.opl: canonical OPL, and OPL → conf → OPL gives the same text.
//! - examples/opl: the OPL of a System maps to a conf, and back to the same OPL.
//! - tests/juiz/invalid: the file name starts with the one juiz rule that must be reported.
//!
//! If `OPM_ROUNDTRIP_OUT` is set, the XML written from each conf is saved under `<dir>/juiz`
//! so that CI validates it against the XSD (scripts/validate_xml.py --also).

use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            v.extend(files(&p, ext));
        } else if p.to_string_lossy().ends_with(ext) {
            v.push(p);
        }
    }
    v.sort();
    v
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap()
}

fn rel(p: &Path) -> String {
    p.strip_prefix(repo())
        .unwrap_or(p)
        .to_string_lossy()
        .into_owned()
}

fn confs() -> Vec<PathBuf> {
    let v = files(&repo().join("tests/conf"), ".conf");
    assert!(v.len() >= 56, "the confs of the four sample repositories");
    v
}

#[test]
fn sample_confs_round_trip() {
    let mut failed = Vec::new();
    for path in confs() {
        if let Err(m) = juiz_opm::round_trip(&read(&path)) {
            failed.push(format!("{}:\n{}", rel(&path), m));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n\n"));
}

#[test]
fn sample_confs_give_the_checked_in_opl() {
    let bless = std::env::var("JUIZ_OPM_BLESS").is_ok();
    let out = std::env::var("OPM_ROUNDTRIP_OUT")
        .ok()
        .map(|d| repo().join(d).join("juiz"));
    for path in confs() {
        let (model, _) = juiz_opm::model_of_conf(&read(&path))
            .unwrap_or_else(|n| panic!("{}: {:?}", rel(&path), n.errors));
        let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
        let golden = path.with_extension("opl");
        if bless {
            std::fs::write(&golden, &opl).unwrap();
        } else {
            assert_eq!(
                read(&golden),
                opl,
                "{} differs from the OPL of the conf",
                rel(&golden)
            );
        }
        if let Some(dir) = &out {
            let sub = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy();
            let file = path.with_extension("opm.xml");
            let file = format!("{}__{}", sub, file.file_name().unwrap().to_string_lossy());
            std::fs::create_dir_all(dir).unwrap();
            std::fs::write(dir.join(file), opm_model::write::to_string(&model)).unwrap();
        }
    }
}

/// OPL → conf → OPL is the identity on the canonical OPL of a System.
fn opl_round_trip(path: &Path) {
    let text = read(path);
    let p = opm_opl::check(&text);
    assert!(p.is_ok(), "{}: {:?}", rel(path), p.diagnostics);
    assert_eq!(
        opm_opl::format(&text, None).unwrap(),
        text,
        "{} is not canonical",
        rel(path)
    );
    let conf = juiz_opm::to_conf(&p.model).unwrap_or_else(|e| panic!("{}: {:?}", rel(path), e));
    let model = juiz_opm::to_model(&conf).unwrap();
    let back = opm_opl::write(&model, opm_opl::Options::default()).text;
    assert_eq!(text, back, "{}: OPL -> conf -> OPL", rel(path));
}

#[test]
fn opl_of_samples_round_trips() {
    for path in files(&repo().join("tests/conf"), ".opl") {
        opl_round_trip(&path);
    }
}

#[test]
fn examples_of_a_system_map_to_a_conf() {
    // talker_listener.opl was written by hand from the Python sample (tutorial ch.4).
    opl_round_trip(&repo().join("examples/opl/talker_listener.opl"));
    let from_conf = read(&repo().join("tests/conf/juiz_examples_python/talker_listener.opl"));
    assert_eq!(
        read(&repo().join("examples/opl/talker_listener.opl")),
        from_conf
    );
    // The others name things by hand, so only conf → OPL → conf is the identity there.
    for f in ["capture_filter.opl", "tutorial.opl"] {
        let p = opm_opl::parse(&read(&repo().join("examples/opl").join(f)));
        let conf = juiz_opm::to_conf(&p.model).unwrap_or_else(|e| panic!("{}: {:?}", f, e));
        let text = conf.write();
        let (again, _) = juiz_opm::canonical_conf(&text).unwrap();
        assert_eq!(text, again, "{}: the conf is canonical", f);
        juiz_opm::round_trip(&text).unwrap_or_else(|m| panic!("{}: {}", f, m));
    }
}

#[test]
fn invalid_models_break_their_rule() {
    let files = files(&repo().join("tests/juiz/invalid"), ".opl");
    assert!(files.len() >= 12);
    for path in files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let rule: u8 = name[1..name.find('_').unwrap()].parse().unwrap();
        let p = opm_opl::parse(&read(&path));
        assert!(p.is_ok(), "{}: {:?}", name, p.diagnostics);
        let found = juiz_opm::check(&p.model);
        assert!(
            !found.is_empty() && found.iter().all(|d| d.rule == rule),
            "{}: expected only J{}, got {:?}",
            name,
            rule,
            found
        );
    }
}

#[test]
fn bad_juiz_annotations_in_xml() {
    // The XSD rejects these; the juiz checker must too (spec/xml.md §8).
    for f in [
        "juiz_bad_connection.xml",
        "juiz_bad_datatype.xml",
        "juiz_unknown_role.xml",
    ] {
        let text = read(&repo().join("tests/xml/invalid").join(f));
        let m = opm_model::read::from_str(&text).unwrap();
        let found = juiz_opm::check(&m);
        assert!(found.iter().any(|d| d.rule == 0), "{}: {:?}", f, found);
    }
}

#[test]
fn reading_a_conf() {
    // The old connection form (an ID and `type`), an ID in `bind`, and a default namespace.
    let text = r#"
name: s
namespace: robot1
plugins: {}
processes:
  - type_name: talker
    name: talker0
    publishes: [chatter, /global]
  - type_name: listener
    name: listener0
    namespace: /other
    subscribes: { arg1: /robot1/chatter }
containers:
  - type_name: cam
    name: cam0
    processes:
      - type_name: cam_read
        name: read0
connections:
  - arg_name: img
    type: pull
    source: { id: "core://core/ContainerProcess/robot1/read0::cam_read" }
    destination: { type_name: listener, name: listener0, namespace: /other }
ecs:
  - type_name: one_shot_ec
    name: ec0
    bind: ["core://core/container_process/robot1/read0::cam_read:cam0"]
"#;
    let (conf, notes) = juiz_opm::Conf::read(text);
    let conf = conf.unwrap();
    assert!(notes.warnings.iter().any(|w| w.contains("`plugins`")));
    assert!(notes.warnings.iter().any(|w| w.contains("`type`")));
    assert_eq!(conf.processes[1].publishes, ["global", "robot1/chatter"]);
    assert_eq!(conf.processes[0].namespace, "other");
    assert_eq!(
        conf.processes[0].subscribes,
        [("arg1".to_string(), "robot1/chatter".to_string())]
    );
    let c = &conf.connections[0];
    assert_eq!(c.source.full_name(), "robot1/read0");
    assert_eq!(c.source.container_name.as_deref(), Some("cam0"));
    assert_eq!(conf.ecs[0].bind[0].full_name(), "robot1/read0");
    juiz_opm::round_trip(text).unwrap();
    let written = conf.write();
    assert!(written.contains("- /global"), "{}", written);
    assert!(written.contains("connection_type: Pull"), "{}", written);
}

#[test]
fn unresolved_references_are_errors() {
    let text = "name: s\nprocesses:\n  - {type_name: a, name: a0}\necs:\n  - {type_name: main_loop_ec, name: ec0, bind: [{type_name: a, name: nowhere}]}\n";
    let e = juiz_opm::model_of_conf(text).unwrap_err();
    assert!(e.errors[0].contains("nowhere"), "{:?}", e.errors);
}
