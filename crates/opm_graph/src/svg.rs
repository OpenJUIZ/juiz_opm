// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! DOT to other formats through the Graphviz `dot` command, which must be on the `PATH`
//! (or named by the `OPM_GRAPH_DOT` environment variable).

use std::io::Write;
use std::process::{Command, Stdio};

/// The `dot` command to run.
pub fn dot_command() -> String {
    std::env::var("OPM_GRAPH_DOT").unwrap_or_else(|_| "dot".to_string())
}

/// Run `dot -T<format>` on the text. `format` is a Graphviz output format (`svg`, `png`, ...).
pub fn render(dot: &str, format: &str) -> Result<Vec<u8>, String> {
    let cmd = dot_command();
    let mut child = Command::new(&cmd)
        .arg(format!("-T{}", format))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run {:?} (is Graphviz installed?): {}", cmd, e))?;
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(dot.as_bytes())
        .map_err(|e| format!("{}: {}", cmd, e))?;
    let out = child
        .wait_with_output()
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

/// [`render`] to SVG.
pub fn to_svg(dot: &str) -> Result<String, String> {
    render(dot, "svg").map(|b| String::from_utf8_lossy(&b).into_owned())
}
