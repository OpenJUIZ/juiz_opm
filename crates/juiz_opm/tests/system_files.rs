// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Tests on the snapshots of running Systems (spec/system.md §6).
//!
//! tests/system/<repo>/<name>.json is a snapshot of tests/conf/<repo>/<name>.conf running.
//!
//! - The model of a snapshot breaks no core rule and no juiz rule, J10 included, and its OPL is
//!   the `.opl` next to the snapshot (`JUIZ_OPM_BLESS=1` rewrites them).
//! - The conf of a snapshot is the conf it ran from, without what the profiles do not hold.
//! - conf + snapshot → model → conf gives the canonical conf, as the conf alone does.

use std::path::{Path, PathBuf};

use juiz_opm::system::{conf_of_snapshot, profile_part};
use juiz_opm::{Conf, SystemInfo};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e))
}

/// (snapshot, the conf it ran from)
fn snapshots() -> Vec<(PathBuf, PathBuf)> {
    let mut v = Vec::new();
    for repo_dir in std::fs::read_dir(repo().join("tests/system")).unwrap() {
        let dir = repo_dir.unwrap().path();
        if !dir.is_dir() {
            continue;
        }
        for e in std::fs::read_dir(&dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().is_some_and(|x| x == "json") {
                let conf = repo()
                    .join("tests/conf")
                    .join(dir.file_name().unwrap())
                    .join(p.file_stem().unwrap())
                    .with_extension("conf");
                v.push((p, conf));
            }
        }
    }
    v.sort();
    assert!(v.len() >= 16, "the snapshots of the sample repositories");
    v
}

fn snapshot(p: &Path) -> serde_json::Value {
    serde_json::from_str(&read(p)).unwrap()
}

fn conf(p: &Path) -> Conf {
    let (c, n) = Conf::read(&read(p));
    c.unwrap_or_else(|| panic!("{}: {:?}", p.display(), n.errors))
}

#[test]
fn snapshots_give_a_valid_model_and_the_checked_in_opl() {
    let bless = std::env::var("JUIZ_OPM_BLESS").is_ok();
    for (snap, conf_path) in snapshots() {
        let s = snapshot(&snap);
        let name = conf(&conf_path).name;
        let (_, model, _) = juiz_opm::model_of_snapshot(&s, &name)
            .unwrap_or_else(|e| panic!("{}: {}", snap.display(), e));
        let core = opm_model::check(&model);
        assert!(core.is_ok(), "{}: {:?}", snap.display(), core.diagnostics);
        let info = SystemInfo::from_snapshot(&s).unwrap();
        let found = juiz_opm::check_with_system(&model, &info.composite_types);
        assert!(found.is_empty(), "{}: {:?}", snap.display(), found);
        let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
        let golden = snap.with_extension("opl");
        if bless {
            std::fs::write(&golden, &opl).unwrap();
        } else {
            assert_eq!(read(&golden), opl, "{} differs", golden.display());
        }
        // The XML and the OPL give the same conf back.
        let back = juiz_opm::to_conf(&opm_opl::parse(&opl).model).unwrap();
        let xml = opm_model::write::to_string(&model);
        let back_xml = juiz_opm::to_conf(&opm_model::read::from_str(&xml).unwrap()).unwrap();
        assert_eq!(back.write(), back_xml.write(), "{}", snap.display());
        if let Ok(dir) = std::env::var("OPM_ROUNDTRIP_OUT") {
            let dir = repo().join(dir).join("system");
            std::fs::create_dir_all(&dir).unwrap();
            let sub = snap
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy();
            let file = format!(
                "{}__{}.opm.xml",
                sub,
                snap.file_stem().unwrap().to_string_lossy()
            );
            std::fs::write(dir.join(file), xml).unwrap();
        }
    }
}

#[test]
fn snapshots_give_the_conf_they_ran_from() {
    let mut failed = Vec::new();
    for (snap, conf_path) in snapshots() {
        let c = conf(&conf_path);
        let (from_snap, _) = conf_of_snapshot(&snapshot(&snap), &c.name).unwrap();
        let (want, got) = (profile_part(&c).write(), profile_part(&from_snap).write());
        if want != got {
            failed.push(format!(
                "{}:\n--- conf\n{}--- snapshot\n{}",
                snap.display(),
                want,
                got
            ));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

#[test]
fn a_conf_with_its_system_round_trips() {
    for (snap, conf_path) in snapshots() {
        let text = read(&conf_path);
        let info = SystemInfo::from_snapshot(&snapshot(&snap)).unwrap();
        let (canonical, _) = juiz_opm::canonical_conf(&text).unwrap();
        let (model, _) = juiz_opm::model_of_conf_with(&text, &info)
            .unwrap_or_else(|n| panic!("{}: {:?}", conf_path.display(), n.errors));
        let found = juiz_opm::check_with_system(&model, &info.composite_types);
        assert!(found.is_empty(), "{}: {:?}", conf_path.display(), found);
        let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
        let back = juiz_opm::to_conf(&opm_opl::parse(&opl).model)
            .unwrap()
            .write();
        assert_eq!(
            canonical,
            back,
            "{}: conf + System -> OPL -> conf",
            conf_path.display()
        );
    }
}

#[test]
fn j10_needs_the_composite() {
    let s = snapshot(&repo().join("tests/system/juiz_examples_rust/composite.json"));
    let info = SystemInfo::from_snapshot(&s).unwrap();
    let (mut c, _) = conf_of_snapshot(&s, "s").unwrap();
    c.composites.clear();
    let model = juiz_opm::to_model(&c).unwrap();
    let found = juiz_opm::check_with_system(&model, &info.composite_types);
    assert!(
        !found.is_empty() && found.iter().all(|d| d.rule == 10),
        "{:?}",
        found
    );
    assert!(juiz_opm::check(&model).is_empty());
}
