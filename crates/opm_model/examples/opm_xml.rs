// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Read an interchange document, check it and write it back.
//!
//! ```sh
//! cargo run -p opm_model --example opm_xml -- check examples/xml/capture_filter.opm.xml
//! cargo run -p opm_model --example opm_xml -- fmt examples/xml/capture_filter.opm.xml
//! ```
//!
//! `check` prints the diagnostics and fails if there is an error. `fmt` prints the document as
//! opm_model writes it.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (cmd, path) = match args.as_slice() {
        [cmd, path] if cmd == "check" || cmd == "fmt" => (cmd.as_str(), path),
        _ => {
            eprintln!("usage: opm_xml (check|fmt) FILE");
            return ExitCode::from(2);
        }
    };
    let model = match opm_model::read::from_path(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}:{}", path, e);
            return ExitCode::FAILURE;
        }
    };
    match cmd {
        "fmt" => {
            print!("{}", opm_model::write::to_string(&model));
            ExitCode::SUCCESS
        }
        _ => {
            let report = opm_model::check(&model);
            for d in &report.diagnostics {
                println!("{}: {}", path, d);
            }
            if report.is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
