// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `juiz_opm`: convert juiz confs to and from juiz-OPL / XML, and check the juiz rules.

use std::path::Path;
use std::process::ExitCode;

use juiz_opm::Notes;
use opm_model::Model;

const USAGE: &str = "\
usage:
  juiz_opm from-conf [--xml] [--system <snapshot.json>] <file.conf> [-o <out>]
                                  conf to juiz-OPL (or the XML with --xml); with --system,
                                  add what the running System says (spec/system.md)
  juiz_opm fetch <http://host:port> [-o <snapshot.json>]
                                  take a snapshot of a running System
  juiz_opm from-system [--xml] [--conf] [--name <name>] <snapshot.json|http://host:port> [-o <out>]
                                  a running System to juiz-OPL (the XML with --xml, its conf
                                  with --conf)
  juiz_opm to-conf <file.opl|file.xml> [-o <out.conf>]
                                  juiz-OPL or XML to a conf in the canonical form
  juiz_opm fmt-conf <file.conf>... print a conf in the canonical form
  juiz_opm check [--system <snapshot.json>] <file>...
                                  check a conf, OPL or XML: the core rules and J1 to J13
                                  (J10 only with --system)
  juiz_opm round-trip <file.conf>...
                                  conf -> OPL -> conf must give the canonical conf

exit status: 0 ok, 1 errors found, 2 bad usage or I/O error";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((cmd, rest)) = args.split_first() else {
        eprintln!("{}", USAGE);
        return ExitCode::from(2);
    };
    let r = match cmd.as_str() {
        "from-conf" => from_conf(rest),
        "fetch" => fetch(rest),
        "from-system" => from_system(rest),
        "to-conf" => to_conf(rest),
        "fmt-conf" => fmt_conf(rest),
        "check" => check(rest),
        "round-trip" => round_trip(rest),
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
            eprintln!("juiz_opm: {}\n\n{}", m, USAGE);
            ExitCode::from(2)
        }
    }
}

struct Args {
    flags: Vec<String>,
    /// `--system` and `--name` with their values.
    values: Vec<(String, String)>,
    files: Vec<String>,
    out: Option<String>,
}

fn args(rest: &[String], known: &[&str]) -> Result<Args, String> {
    let mut a = Args {
        flags: Vec::new(),
        values: Vec::new(),
        files: Vec::new(),
        out: None,
    };
    let mut it = rest.iter();
    while let Some(x) = it.next() {
        if x == "-o" {
            a.out = Some(it.next().ok_or("-o needs a path")?.clone());
        } else if matches!(x.as_str(), "--system" | "--name") && known.contains(&x.as_str()) {
            let v = it.next().ok_or_else(|| format!("{} needs a value", x))?;
            a.values.push((x.clone(), v.clone()));
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

impl Args {
    fn has(&self, flag: &str) -> bool {
        self.flags.iter().any(|f| f == flag)
    }

    fn value(&self, key: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// A snapshot from a file, or taken now from a URL.
fn snapshot(src: &str) -> Result<serde_json::Value, String> {
    if src.starts_with("http://") || src.starts_with("https://") {
        return juiz_opm::system::fetch(src);
    }
    serde_json::from_str(&read(src)?).map_err(|e| format!("{}: {}", src, e))
}

fn system_info(a: &Args) -> Result<Option<juiz_opm::SystemInfo>, String> {
    match a.value("--system") {
        Some(src) => Ok(Some(juiz_opm::SystemInfo::from_snapshot(&snapshot(src)?)?)),
        None => Ok(None),
    }
}

fn write_model(m: &Model, xml: bool) -> String {
    if xml {
        opm_model::write::to_string(m)
    } else {
        opm_opl::write(m, opm_opl::Options::default()).text
    }
}

fn fetch(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let snap = juiz_opm::system::fetch(one(&a)?)?;
    let mut text = serde_json::to_string_pretty(&snap).expect("JSON");
    text.push('\n');
    emit(&a.out, &text)?;
    Ok(true)
}

fn from_system(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &["--xml", "--conf", "--name"])?;
    let src = one(&a)?;
    let name = a.value("--name").unwrap_or("system");
    let (conf, model, notes) = juiz_opm::model_of_snapshot(&snapshot(src)?, name)?;
    report(src, &notes);
    let text = if a.has("--conf") {
        conf.write()
    } else {
        write_model(&model, a.has("--xml"))
    };
    emit(&a.out, &text)?;
    Ok(true)
}

fn one(a: &Args) -> Result<&str, String> {
    match a.files.as_slice() {
        [f] => Ok(f),
        _ => Err("give one file".to_string()),
    }
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path, e))
}

fn emit(out: &Option<String>, text: &str) -> Result<(), String> {
    match out {
        Some(p) => std::fs::write(p, text).map_err(|e| format!("{}: {}", p, e)),
        None => {
            print!("{}", text);
            Ok(())
        }
    }
}

fn report(path: &str, notes: &Notes) {
    for w in &notes.warnings {
        eprintln!("{}: warning: {}", path, w);
    }
    for e in &notes.errors {
        eprintln!("{}: error: {}", path, e);
    }
}

fn ext(path: &str) -> &str {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
}

/// Read an OPL or XML file as a model, printing the errors of reading.
fn model_of(path: &str) -> Result<Option<Model>, String> {
    let text = read(path)?;
    if ext(path) == "xml" {
        return match opm_model::read::from_str(&text) {
            Ok(m) => Ok(Some(m)),
            Err(e) => {
                eprintln!("{}: error: XML: {}", path, e);
                Ok(None)
            }
        };
    }
    let p = opm_opl::parse(&text);
    if !p.is_ok() {
        for d in &p.diagnostics {
            eprintln!("{}: {}", path, d);
        }
        return Ok(None);
    }
    Ok(Some(p.model))
}

fn from_conf(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &["--xml", "--system"])?;
    let path = one(&a)?;
    let text = read(path)?;
    let r = match system_info(&a)? {
        Some(info) => juiz_opm::model_of_conf_with(&text, &info),
        None => juiz_opm::model_of_conf(&text),
    };
    match r {
        Ok((m, notes)) => {
            report(path, &notes);
            emit(&a.out, &write_model(&m, a.has("--xml")))?;
            Ok(true)
        }
        Err(notes) => {
            report(path, &notes);
            Ok(false)
        }
    }
}

fn to_conf(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let path = one(&a)?;
    let Some(m) = model_of(path)? else {
        return Ok(false);
    };
    match juiz_opm::to_conf(&m) {
        Ok(c) => {
            emit(&a.out, &c.write())?;
            Ok(true)
        }
        Err(es) => {
            for e in es {
                eprintln!("{}: error: {}", path, e);
            }
            Ok(false)
        }
    }
}

fn fmt_conf(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let mut ok = true;
    for path in &a.files {
        match juiz_opm::canonical_conf(&read(path)?) {
            Ok((text, notes)) => {
                report(path, &notes);
                print!("{}", text);
            }
            Err(notes) => {
                report(path, &notes);
                ok = false;
            }
        }
    }
    Ok(ok)
}

fn check(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &["--system"])?;
    let info = system_info(&a)?;
    let mut ok = true;
    for path in &a.files {
        let model = if matches!(ext(path), "opl" | "xml") {
            model_of(path)?
        } else {
            let text = read(path)?;
            let r = match &info {
                Some(i) => juiz_opm::model_of_conf_with(&text, i),
                None => juiz_opm::model_of_conf(&text),
            };
            match r {
                Ok((m, notes)) => {
                    report(path, &notes);
                    Some(m)
                }
                Err(notes) => {
                    report(path, &notes);
                    None
                }
            }
        };
        let Some(m) = model else {
            ok = false;
            continue;
        };
        let core = opm_model::check(&m);
        for d in &core.diagnostics {
            eprintln!("{}: {}", path, d);
        }
        ok &= core.is_ok();
        let js = match &info {
            Some(i) => juiz_opm::check_with_system(&m, &i.composite_types),
            None => juiz_opm::check(&m),
        };
        for d in &js {
            eprintln!("{}: {}", path, d);
        }
        ok &= js.is_empty();
    }
    Ok(ok)
}

fn round_trip(rest: &[String]) -> Result<bool, String> {
    let a = args(rest, &[])?;
    let mut ok = true;
    for path in &a.files {
        match juiz_opm::round_trip(&read(path)?) {
            Ok(()) => println!("{}: ok", path),
            Err(m) => {
                println!("{}: FAILED\n{}", path, m);
                ok = false;
            }
        }
    }
    Ok(ok)
}
