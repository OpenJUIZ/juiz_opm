// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Tests on the XML files of the repository (spec/xml.md §14).
//!
//! - examples/xml: read, check (no error), and round trip.
//! - tests/xml/invalid: rejected by the XSD; the reader or the checker must reject them too.
//! - tests/xml/check: valid against the XSD; the checker reports the rule in the file name.
//!
//! If `OPM_ROUNDTRIP_OUT` is set, the written examples are saved there so that CI can validate
//! them against the XSD (scripts/validate_xml.py --also). A relative path is taken from the
//! repository root.

use std::path::{Path, PathBuf};

use opm_model::{check, read, write, Rule, Severity, JUIZ_NS};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn xml_files(dir: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(repo().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "xml"))
        .collect();
    v.sort();
    assert!(!v.is_empty(), "no xml in {}", dir);
    v
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

#[test]
fn examples_round_trip() {
    let out = std::env::var_os("OPM_ROUNDTRIP_OUT").map(|d| repo().join(d));
    if let Some(dir) = &out {
        std::fs::create_dir_all(dir).unwrap();
    }
    for path in xml_files("examples/xml") {
        let m1 = read::from_path(&path).unwrap_or_else(|e| panic!("{}: {}", name(&path), e));
        let report = check(&m1);
        assert!(report.is_ok(), "{}:\n{}", name(&path), report);

        let s1 = write::to_string(&m1);
        let m2 = read::from_str(&s1)
            .unwrap_or_else(|e| panic!("{} (written): {}\n{}", name(&path), e, s1));
        assert_eq!(m1, m2, "{}: the model changed in a round trip", name(&path));
        let s2 = write::to_string(&m2);
        assert_eq!(s1, s2, "{}: writing is not stable", name(&path));

        if let Some(dir) = &out {
            std::fs::write(dir.join(name(&path)), &s1).unwrap();
        }
    }
}

#[test]
fn examples_warnings() {
    // Only the EC cycle of capture_filter changes nothing (C8, metamodel §8).
    let mut warnings = Vec::new();
    for path in xml_files("examples/xml") {
        let m = read::from_path(&path).unwrap();
        for d in check(&m).warnings() {
            warnings.push(format!("{} {} {}", name(&path), d.rule, d.id));
        }
    }
    assert_eq!(warnings, vec!["capture_filter.opm.xml C8 ec_cycle"]);
}

#[test]
fn juiz_annotations_are_kept() {
    let path = repo().join("examples/xml/capture_filter.opm.xml");
    let m = read::from_str(&write::to_string(&read::from_path(path).unwrap())).unwrap();
    let camera = m.thing("camera").unwrap();
    assert_eq!(
        camera.extensions.attribute(JUIZ_NS, "role"),
        Some("container")
    );
    assert_eq!(
        camera.extensions.attribute(JUIZ_NS, "type"),
        Some("video_capture")
    );
    let code = m.link("l_code_filter").unwrap();
    assert_eq!(code.extensions.attribute(JUIZ_NS, "arg_index"), Some("1"));
    let applied = code.extensions.element(JUIZ_NS, "applied").unwrap();
    assert_eq!(applied.text(), "\"COLOR_BGR2GRAY\"");
    assert_eq!(m.prefixes, vec![("juiz".to_string(), JUIZ_NS.to_string())]);
}

#[test]
fn coffee_making_content() {
    let m = read::from_path(repo().join("examples/xml/coffee_making.opm.xml")).unwrap();
    assert_eq!(m.things.len(), 12);
    assert_eq!(m.links.len(), 10);
    let z = m.in_zoom_of("making").unwrap();
    assert_eq!(
        z.steps,
        vec![
            vec!["heating".to_string(), "grinding".to_string()],
            vec!["brewing".to_string()]
        ]
    );
    let heat = m.link("l_heat").unwrap();
    assert_eq!(heat.from_state.as_deref(), Some("water_cold"));
    let diagrams: Vec<&str> = m.diagrams().iter().map(|d| d.id.as_str()).collect();
    assert_eq!(diagrams, vec!["sd", "sd1", "sd2"]);
    let (owner, cold) = m.state("water_cold").unwrap();
    assert_eq!(owner.id, "water");
    assert!(cold.initial && cold.default);
}

/// The rule each file of tests/xml/invalid breaks, or `None` when the reader rejects it.
/// Files about juiz annotations are for the juiz checker (Step 7) and pass the core.
fn invalid_expectation(file: &str) -> Option<Option<Rule>> {
    Some(match file {
        "child_diagram_without_refinement.xml" => None,
        "modifier_on_result.xml" => None,
        "duplicate_id.xml" | "missing_ref.xml" => Some(Rule::C1),
        "duplicate_state_name.xml" | "duplicate_thing_name.xml" => Some(Rule::C2),
        "instrument_to_object.xml" => Some(Rule::C3),
        "effect_from_object.xml" => Some(Rule::C5),
        "step_member_object.xml" | "two_in_zooms.xml" => Some(Rule::C9),
        "fan_one_link.xml" => Some(Rule::C11),
        "id_non_ascii.xml" => Some(Rule::F1),
        f if f.starts_with("juiz_") => return None,
        f => panic!("no expectation for tests/xml/invalid/{}", f),
    })
}

#[test]
fn invalid_files_are_rejected() {
    for path in xml_files("tests/xml/invalid") {
        let file = name(&path);
        let read = read::from_path(&path);
        match invalid_expectation(&file) {
            None => {
                let m = read.unwrap_or_else(|e| panic!("{}: {}", file, e));
                assert!(
                    check(&m).is_ok(),
                    "{}: the core accepts juiz mistakes",
                    file
                );
            }
            Some(None) => assert!(read.is_err(), "{}: the reader must reject it", file),
            Some(Some(rule)) => {
                let m = read.unwrap_or_else(|e| panic!("{}: {}", file, e));
                let r = check(&m);
                assert!(
                    r.errors().any(|d| d.rule == rule),
                    "{}: expected an error of {}, got\n{}",
                    file,
                    rule,
                    r
                );
            }
        }
    }
}

#[test]
fn check_files_report_their_rule() {
    for path in xml_files("tests/xml/check") {
        let file = name(&path);
        // c05_state_of_other_object.xml -> C5
        let n: u32 = file[1..3]
            .parse()
            .unwrap_or_else(|_| panic!("{}: name it cNN_...", file));
        let m = read::from_path(&path).unwrap_or_else(|e| panic!("{}: {}", file, e));
        let r = check(&m);
        let found: Vec<_> = r
            .diagnostics
            .iter()
            .map(|d| (format!("{}", d.rule), d.severity))
            .collect();
        let want = format!("C{}", n);
        let severity = if n == 8 {
            Severity::Warning
        } else {
            Severity::Error
        };
        assert!(
            found.contains(&(want.clone(), severity)),
            "{}: expected {} as {:?}, got\n{}",
            file,
            want,
            severity,
            r
        );
        // Nothing else is wrong in the file.
        assert!(
            r.diagnostics.iter().all(|d| format!("{}", d.rule) == want),
            "{}: other diagnostics\n{}",
            file,
            r
        );
    }
}
