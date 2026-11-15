// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `opm_opl`: check and format juiz-OPL, and convert between OPL and the interchange XML.

use std::path::Path;
use std::process::ExitCode;

use opm_opl::{has_error, Diagnostic, Options};

const USAGE: &str = "\
usage:
  opm_opl check <file>...                 check OPL (.opl) or XML (.xml) files
  opm_opl fmt [--check | --write] [--ids | --no-ids] <file.opl>...
                                          print in the canonical form; --check fails when a
                                          file is not canonical, --write rewrites the file
  opm_opl to-xml <file.opl> [-o <out.xml>]
  opm_opl from-xml [--ids] <file.xml> [-o <out.opl>]

exit status: 0 ok, 1 errors found (or not canonical), 2 bad usage or I/O error";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let r = match cmd.as_str() {
        "check" => check(rest),
        "fmt" => fmt(rest),
        "to-xml" => to_xml(rest),
        "from-xml" => from_xml(rest),
        "-h" | "--help" | "help" => {
            println!("{}", USAGE);
            Ok(true)
        }
        _ => Err(format!("unknown command {:?}", cmd)),
    };
    match r {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(m) => {
            eprintln!("opm_opl: {}\n\n{}", m, USAGE);
            ExitCode::from(2)
        }
    }
}

struct Args {
    flags: Vec<String>,
    files: Vec<String>,
    out: Option<String>,
}

fn args(rest: &[String], known: &[&str]) -> Result<Args, String> {
    let mut a = Args {
        flags: Vec::new(),
        files: Vec::new(),
        out: None,
    };
    let mut it = rest.iter();
    while let Some(x) = it.next() {
        if x == "-o" {
            a.out = Some(it.next().ok_or("-o needs a path")?.clone());
        } else if x.starts_with('-') {
            if !known.contains(&x.as_str()) {
                return Err(format!("unknown option {:?}", x));
            }
            a.flags.push(x.clone());
        } else {
            a.files.push(x.clone());
        }
    }
    if a.files.is_empty() {
        return Err("no file given".to_string());
    }
    Ok(a)
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))
}

fn report(path: &str, diags: &[Diagnostic]) {
    for d in diags {
        eprintln!(
            "{}:{}",
            path,
            if d.pos.is_some() { "" } else { " " }.to_string() + &d.to_string()
        );
    }
}

fn is_xml(path: &str) -> bool {
    Path::new(path).extension().is_some_and(|e| e == "xml")
}

fn check(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let mut ok = true;
    for f in &a.files {
        let text = read(f)?;
        let diags: Vec<Diagnostic> = if is_xml(f) {
            match opm_model::read::from_str(&text) {
                Ok(m) => opm_model::check(&m)
                    .diagnostics
                    .into_iter()
                    .map(|d| Diagnostic {
                        code: opm_opl::Code::Model(d.rule),
                        severity: d.severity,
                        pos: None,
                        id: Some(d.id),
                        message: d.message,
                    })
                    .collect(),
                Err(e) => {
                    eprintln!("{}:{}", f, e);
                    ok = false;
                    continue;
                }
            }
        } else {
            opm_opl::check(&text).diagnostics
        };
        report(f, &diags);
        ok &= !has_error(&diags);
    }
    Ok(ok)
}

fn fmt(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &["--check", "--write", "--ids", "--no-ids"])?;
    let has = |f: &str| a.flags.iter().any(|x| x == f);
    let ids = if has("--ids") {
        Some(true)
    } else if has("--no-ids") {
        Some(false)
    } else {
        None
    };
    let mut ok = true;
    for f in &a.files {
        let text = read(f)?;
        match opm_opl::format(&text, ids) {
            Err(d) => {
                report(f, &d);
                ok = false;
            }
            Ok(out) => {
                if has("--check") {
                    if out != text {
                        eprintln!("{}: not in the canonical form", f);
                        ok = false;
                    }
                } else if has("--write") {
                    if out != text {
                        std::fs::write(f, out).map_err(|e| format!("{}: {}", f, e))?;
                    }
                } else {
                    print!("{}", out);
                }
            }
        }
    }
    Ok(ok)
}

fn emit(out: Option<&str>, text: &str) -> Result<(), String> {
    match out {
        Some(p) => std::fs::write(p, text).map_err(|e| format!("{}: {}", p, e)),
        None => {
            print!("{}", text);
            Ok(())
        }
    }
}

fn to_xml(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let [f] = a.files.as_slice() else {
        return Err("to-xml takes one file".to_string());
    };
    match opm_opl::to_xml(&read(f)?) {
        Ok((xml, warnings)) => {
            report(f, &warnings);
            emit(a.out.as_deref(), &xml)?;
            Ok(true)
        }
        Err(d) => {
            report(f, &d);
            Ok(false)
        }
    }
}

fn from_xml(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &["--ids"])?;
    let [f] = a.files.as_slice() else {
        return Err("from-xml takes one file".to_string());
    };
    let options = Options {
        ids: a.flags.iter().any(|x| x == "--ids"),
    };
    match opm_opl::from_xml(&read(f)?, options) {
        Ok(out) => {
            report(f, &out.diagnostics);
            emit(a.out.as_deref(), &out.text)?;
            Ok(true)
        }
        Err(d) => {
            report(f, &d);
            Ok(false)
        }
    }
}
