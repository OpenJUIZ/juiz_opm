// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The one command (decision 0018): `juiz_opm opl`, `draw` and `opx` run the other tools, and
//! the conf commands still work.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_juiz_opm"))
        .args(args)
        .current_dir(repo())
        .output()
        .unwrap()
}

fn out_dir() -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("command");
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn version() {
    let o = run(&["--version"]);
    assert!(o.status.success());
    assert_eq!(
        String::from_utf8_lossy(&o.stdout).trim(),
        format!("juiz_opm {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn opl_runs_opm_opl() {
    let o = run(&["opl", "check", "examples/opl/coffee_making.opl"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let o = run(&["opl", "fmt", "--check", "examples/opl/coffee_making.opl"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    // the usage names the command as it was called
    let o = run(&["opl", "--help"]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("juiz_opm opl check"));
    let o = run(&["opl", "nonsense"]);
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn draw_runs_opm_graph() {
    let o = run(&[
        "draw",
        "-T",
        "dot",
        "--diagram",
        "SD1",
        "examples/xml/coffee_making.opm.xml",
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stdout).starts_with("digraph \"SD1\""));
}

#[test]
fn opx_picks_the_format_from_the_output() {
    let dir = out_dir();
    let opz = dir.join("coffee.opz");
    let opx = dir.join("coffee.opx");
    let model = "examples/xml/coffee_making.opm.xml";
    assert!(run(&["opx", model, "-o", opz.to_str().unwrap()])
        .status
        .success());
    assert!(run(&["opx", model, "-o", opx.to_str().unwrap()])
        .status
        .success());
    let gz = std::fs::read(&opz).unwrap();
    assert_eq!(&gz[..2], &[0x1f, 0x8b], "gzip");
    let text = std::fs::read_to_string(&opx).unwrap();
    assert!(text.starts_with("<?xml") && text.contains("<OPX version=\"3\">"));
    // to stdout without -o
    let o = run(&["opx", model]);
    assert!(String::from_utf8_lossy(&o.stdout).contains("<OPX version=\"3\">"));
}

#[test]
fn conf_commands_still_work() {
    let o = run(&[
        "from-conf",
        "tests/conf/juiz_examples_rust/control_loop.conf",
    ]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(!o.stdout.is_empty());
}
