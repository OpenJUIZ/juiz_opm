// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Tests on the OPL files of the repository (spec/opl.md §10.3).
//!
//! - examples/opl and tests/opl: no error, already canonical, and OPL → XML → OPL gives the
//!   same text. The examples also match the OPL written from examples/xml.
//! - XML → OPL with ids → XML gives the same model, but for what OPL leaves out.
//! - tests/opl/invalid: the file name starts with the rule that must be reported.
//!
//! If `OPM_ROUNDTRIP_OUT` is set, the XML written from OPL is saved under `<dir>/opl` so that
//! CI validates it against the XSD (scripts/validate_xml.py --also).

use std::path::{Path, PathBuf};

use opm_model::{read, Model};
use opm_opl::{Code, Options};

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
    assert!(!v.is_empty(), "no {} in {}", ext, dir);
    v
}

fn name(p: &Path) -> String {
    p.file_name().unwrap().to_string_lossy().into_owned()
}

fn save(file: &str, text: &str) {
    if let Ok(dir) = std::env::var("OPM_ROUNDTRIP_OUT") {
        let dir = repo().join(dir).join("opl");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(file), text).unwrap();
    }
}

fn canonical_files() -> Vec<PathBuf> {
    let mut v = files("examples/opl", ".opl");
    v.extend(files("tests/opl", ".opl"));
    v
}

#[test]
fn canonical_and_round_trip() {
    for path in canonical_files() {
        let n = name(&path);
        let text = std::fs::read_to_string(&path).unwrap();
        let checked = opm_opl::check(&text);
        assert!(checked.is_ok(), "{}: {:?}", n, checked.diagnostics);
        let fmt = opm_opl::format(&text, None).unwrap();
        assert_eq!(fmt, text, "{} is not canonical", n);

        let (xml, _) = opm_opl::to_xml(&text).unwrap();
        save(&n.replace(".opl", ".opm.xml"), &xml);
        let back = opm_opl::from_xml(&xml, Options::default()).unwrap();
        assert_eq!(back.text, text, "{}: OPL → XML → OPL", n);
    }
}

#[test]
fn examples_match_their_xml() {
    for path in files("examples/xml", ".opm.xml") {
        let n = name(&path);
        let xml = std::fs::read_to_string(&path).unwrap();
        let out = opm_opl::from_xml(&xml, Options::default()).unwrap();
        let opl = repo()
            .join("examples/opl")
            .join(n.replace(".opm.xml", ".opl"));
        let want = std::fs::read_to_string(&opl).unwrap();
        assert_eq!(out.text, want, "{} → OPL differs from {}", n, opl.display());
        // Only the OPDs are left out of these examples.
        for d in &out.diagnostics {
            assert_eq!(d.code, Code::Dropped, "{}: {}", n, d);
            assert!(d.message.contains("OPD"), "{}: {}", n, d);
        }
    }
}

/// The model without what OPL does not hold, in an order that does not depend on the text.
fn normalize(mut m: Model) -> Model {
    m.diagram = None;
    m.prefixes.clear();
    let ext = |e: &mut opm_model::Extensions| {
        e.attributes
            .sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
        e.elements
            .sort_by(|a, b| (&a.namespace, &a.name).cmp(&(&b.namespace, &b.name)));
    };
    ext(&mut m.extensions);
    m.things.sort_by(|a, b| a.id.cmp(&b.id));
    for t in &mut m.things {
        ext(&mut t.extensions);
        if let opm_model::ThingKind::Object { states } = &mut t.kind {
            for s in states {
                ext(&mut s.extensions);
            }
        }
    }
    m.links.sort_by(|a, b| a.id.cmp(&b.id));
    for l in &mut m.links {
        ext(&mut l.extensions);
    }
    m.fans.sort_by(|a, b| a.id.cmp(&b.id));
    for f in &mut m.fans {
        f.links.sort();
    }
    m.in_zooms.sort_by(|a, b| a.id.cmp(&b.id));
    for z in &mut m.in_zooms {
        for s in &mut z.steps {
            s.sort();
        }
        z.parts.sort();
    }
    m
}

#[test]
fn xml_with_ids_round_trip() {
    for path in files("examples/xml", ".opm.xml") {
        let n = name(&path);
        let original = read::from_path(&path).unwrap();
        let xml = std::fs::read_to_string(&path).unwrap();
        let opl = opm_opl::from_xml(&xml, Options { ids: true }).unwrap().text;
        let parsed = opm_opl::parse(&opl);
        assert!(parsed.is_ok(), "{}: {:?}", n, parsed.diagnostics);
        assert!(parsed.explicit_ids);
        assert_eq!(
            normalize(parsed.model),
            normalize(original),
            "{}: XML → OPL (ids) → model",
            n
        );
        // `fmt` keeps the ids of a text that has them.
        assert_eq!(opm_opl::format(&opl, None).unwrap(), opl, "{}", n);
    }
}

#[test]
fn invalid_files_report_their_rule() {
    for path in files("tests/opl/invalid", ".opl") {
        let n = name(&path);
        let want = n.split('_').next().unwrap().to_ascii_uppercase();
        let text = std::fs::read_to_string(&path).unwrap();
        let p = opm_opl::check(&text);
        assert!(!p.is_ok(), "{} has no error", n);
        assert!(
            p.diagnostics
                .iter()
                .any(|d| d.is_error() && d.code.to_string().to_ascii_uppercase() == want),
            "{}: wanted {}, got {:?}",
            n,
            want,
            p.diagnostics
        );
    }
}

/// Generated ids (spec/opl.md §10.4).
#[test]
fn generated_ids() {
    let p = opm_opl::parse(
        "Model \"m\".\n\
         Main Loop EC is an object.\nMain Loop EC can be stopped or started.\n\
         Main Loop EC Cycle is a process.\n\
         Main Loop EC Cycle occurs if Main Loop EC is started, otherwise Main Loop EC Cycle is skipped.\n\
         Main Loop EC Cycle invokes itself.\n\
         Brewing is a process.\nBrewing consumes either A or B.\nBrewing consumes A.\n\
         A is an object.\nB is an object {id: a_2}.\n\
         \"a\" is an object.\n\
         Brewing zooms into nothing.\n",
    );
    assert!(p.is_ok(), "{:?}", p.diagnostics);
    let m = &p.model;
    assert_eq!(m.id, "opm-model");
    let ids: Vec<&str> = m.things.iter().map(|t| t.id.as_str()).collect();
    // Name order: "A" < "B" < "Brewing" < ... < "a"; "a" collides with "A".
    assert!(
        ids.contains(&"a") && ids.contains(&"a_3") && ids.contains(&"a_2"),
        "{:?}",
        ids
    );
    let link = |s: &str| m.links.iter().any(|l| l.id == s);
    assert!(link("instrument-main_loop_ec.started-main_loop_ec_cycle"));
    assert!(link("invocation-main_loop_ec_cycle-main_loop_ec_cycle"));
    // "Brewing consumes A." comes before "Brewing consumes either ..." in the canonical order.
    assert!(link("consumption-a-brewing"));
    assert!(link("consumption-a-brewing-2"));
    assert!(link("consumption-a_2-brewing"));
    assert_eq!(m.fans[0].id, "fan-brewing");
    assert_eq!(m.in_zooms[0].id, "in_zoom-brewing");
    assert_eq!(m.things[0].states().len(), 0);
}

/// Forms accepted on input that the canonical form writes another way (spec/opl.md §9.1).
#[test]
fn input_forms_are_normalized() {
    let cases = [
        (
            "P is a process. A is an object. B is an object. P requires A and B.",
            "A is an object.\n\nB is an object.\n\nP is a process.\nP requires A.\nP requires B.\n",
        ),
        (
            "Water is a informatical systemic object. Water can be cold, hot. State cold of Water is initial and default.",
            "Water is an object.\nWater can be cold or hot.\nWater is initially cold.\nState cold of Water is default.\n",
        ),
        (
            "P is a process. A is an object. B is an object. At least one of A and B handle P.",
            "A is an object.\n\nB is an object.\n\nP is a process.\nAt least one of A or B handles P.\n",
        ),
        (
            "P is a process. A is an object. P occurs if A exists, in which case A is consumed, otherwise P is skipped.",
            "A is an object.\n\nP is a process.\nP occurs if A exists, in which case P consumes A, otherwise P is skipped.\n",
        ),
        (
            "R is an object. G is an object. R consists of 0..1 G. R consists of 2..* \"g2\". \"g2\" is an object.",
            "G is an object.\n\nR is an object.\nR consists of 2 to many \"g2\".\nR consists of an optional G.\n\n\"g2\" is an object.\n",
        ),
        (
            "# a comment\nA is an object [status: planned, role: value].",
            "A is an object [role: value, status: planned].\n",
        ),
    ];
    for (input, want) in cases {
        assert_eq!(opm_opl::format(input, None).unwrap(), want, "{}", input);
    }
}

/// What the XML has and OPL cannot write (spec/opl.md §10.2).
#[test]
fn unwritable_models() {
    let base = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0" format_version="0.1" id="m">
  <opm:things><opm:object id="a" name="A"/><opm:object id="b" name="B"/><opm:process id="p" name="P"/></opm:things>
  <opm:links>LINKS</opm:links>FANS
</opm:model>"#;
    let doc = |links: &str, fans: &str| base.replace("LINKS", links).replace("FANS", fans);
    let errs = |xml: String| match opm_opl::from_xml(&xml, Options::default()) {
        Ok(o) => panic!("written: {}", o.text),
        Err(d) => d,
    };
    let d = errs(doc(
        r#"<opm:consumption id="l" source="a" target="p" modifier="event" source_multiplicity="2"/>"#,
        "",
    ));
    assert_eq!(d[0].code, Code::Unwritable);
    let d = errs(doc(
        r#"<opm:instrument id="l1" source="a" target="p" path="p1"/><opm:instrument id="l2" source="b" target="p"/>"#,
        r#"<opm:fans><opm:fan id="f" operator="xor"><opm:link_ref ref="l1"/><opm:link_ref ref="l2"/></opm:fan></opm:fans>"#,
    ));
    assert_eq!(d[0].code, Code::Unwritable);
    let d = errs(doc(
        r#"<opm:overtime_exception id="l" source="p" target="p"/>"#,
        "",
    ));
    assert_eq!(d[0].code, Code::Unwritable);
}
