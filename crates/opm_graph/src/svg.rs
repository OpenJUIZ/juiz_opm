// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The outside commands: Graphviz `dot` (or the command in `OPM_GRAPH_DOT`) places and draws,
//! and `rsvg-convert` (or `OPM_GRAPH_RSVG`) turns an SVG into PNG.

use std::io::Write;
use std::process::{Command, Stdio};

/// The `dot` command to run.
pub fn dot_command() -> String {
    std::env::var("OPM_GRAPH_DOT").unwrap_or_else(|_| "dot".to_string())
}

fn run(cmd: &str, args: &[String], input: &[u8], what: &str) -> Result<Vec<u8>, String> {
    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {:?} (is {} installed?): {}", cmd, what, e))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = input.to_vec();
    // write from another thread so that a large output cannot block the input
    let writer = std::thread::spawn(move || stdin.write_all(&input));
    let out = child
        .wait_with_output()
        .map_err(|e| format!("{}: {}", cmd, e))?;
    writer
        .join()
        .map_err(|_| format!("{}: writing failed", cmd))?
        .map_err(|e| format!("{}: {}", cmd, e))?;
    if !out.status.success() {
        return Err(format!(
            "{} failed: {}",
            cmd,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(out.stdout)
}

/// An SVG as PNG through `rsvg-convert` (librsvg), at twice the size.
pub fn svg_to_png(svg: &str) -> Result<Vec<u8>, String> {
    let cmd = std::env::var("OPM_GRAPH_RSVG").unwrap_or_else(|_| "rsvg-convert".to_string());
    run(
        &cmd,
        &["--zoom=2".to_string(), "--format=png".to_string()],
        svg.as_bytes(),
        "librsvg",
    )
}

/// Run `dot -T<format>` on the text. `format` is a Graphviz output format (`svg`, `png`, ...).
pub fn render(dot: &str, format: &str) -> Result<Vec<u8>, String> {
    run(
        &dot_command(),
        &[format!("-T{}", format)],
        dot.as_bytes(),
        "Graphviz",
    )
}

/// [`render`] to SVG.
pub fn to_svg(dot: &str) -> Result<String, String> {
    render(dot, "svg").map(|b| String::from_utf8_lossy(&b).into_owned())
}
