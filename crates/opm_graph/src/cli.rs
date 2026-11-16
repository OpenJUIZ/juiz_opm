// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The `opm_graph` command (also `juiz_opm draw`): draw an OPM model (juiz-OPL or the interchange XML) as the abbreviated
//! diagram (SVG or PNG), or as DOT in the OPM form.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  opm_graph [-T dot|svg|png] [--diagram <name>] [-o <out>] <file.opl | file.xml>
  opm_graph -T opx|opz [-o <out>] <file.opl | file.xml>

  Draws every OPD of the model (root first), or the whole model when it has no OPD.
  -T          output format (default svg). svg and png are the abbreviated diagram
              (spec/diagram.md); dot is the OPM form as Graphviz DOT. All run Graphviz
              `dot` (or OPM_GRAPH_DOT) but dot; png also runs rsvg-convert
              (or OPM_GRAPH_RSVG).
              opx writes the whole model, every OPD, as one OPCAT file to open in
              OPCAT or OPCloud (spec/opx.md); opz is the same gzipped. What OPX
              cannot hold is reported on stderr.
  --diagram   draw only the OPD of this name (SD, SD1, ...)
  -o          output file for one picture, or a directory for several
              (named <model file>.<OPD name>.<format>, or <model file>.<format>
              for a model without OPDs). Without -o, one picture goes to
              stdout; several need -o.

exit status: 0 ok, 1 the model could not be read or drawn, 2 bad usage or I/O error";

/// Run the command with its arguments (without the program name). `prog` is the name to
/// show in messages and the usage, such as `opm_graph` or `juiz_opm draw`.
pub fn run(prog: &str, args: Vec<String>) -> ExitCode {
    let usage = USAGE.replace("opm_graph ", &format!("{} ", prog));
    match run_args(prog, &usage, args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, m)) => {
            eprintln!("{}: {}", prog, m);
            if code == 2 {
                eprintln!("\n{}", usage);
            }
            ExitCode::from(code)
        }
    }
}

fn run_args(prog: &str, usage_text: &str, args: Vec<String>) -> Result<(), (u8, String)> {
    let usage = |m: String| (2u8, m);
    let mut format = "svg".to_string();
    let mut diagram = None;
    let mut out = None;
    let mut file = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{}", usage_text);
                return Ok(());
            }
            "-T" => format = it.next().ok_or_else(|| usage("-T needs a format".into()))?,
            "--diagram" => {
                diagram = Some(
                    it.next()
                        .ok_or_else(|| usage("--diagram needs a name".into()))?,
                )
            }
            "-o" => out = Some(it.next().ok_or_else(|| usage("-o needs a path".into()))?),
            s if s.starts_with("-T") && s.len() > 2 => format = s[2..].to_string(),
            s if s.starts_with('-') => return Err(usage(format!("unknown option {:?}", s))),
            _ if file.is_some() => return Err(usage("give one model file".into())),
            _ => file = Some(PathBuf::from(a)),
        }
    }
    if !["dot", "svg", "png", "opx", "opz"].contains(&format.as_str()) {
        return Err(usage(format!("unknown format {:?}", format)));
    }
    let file = file.ok_or_else(|| usage("no model file".into()))?;
    let model = crate::read_model(&file).map_err(|m| (1, m))?;
    if format == "opx" || format == "opz" {
        if diagram.is_some() {
            return Err(usage("--diagram does not go with -T opx / opz".into()));
        }
        let opx = crate::opx::to_opx(&model, &crate::opx::Options::now());
        for n in &opx.notes {
            eprintln!("{}: note: {}", prog, n);
        }
        let bytes = if format == "opz" {
            crate::opx::gzip(&opx.xml)
        } else {
            opx.xml.into_bytes()
        };
        return match out {
            Some(o) => std::fs::write(&o, bytes).map_err(|e| (2, format!("{}: {}", o, e))),
            None => {
                use std::io::Write;
                std::io::stdout()
                    .write_all(&bytes)
                    .map_err(|e| (2, e.to_string()))
            }
        };
    }
    // (name, is an OPD, text): DOT in the OPM form, or the abbreviated diagram in SVG
    let mut pictures: Vec<(String, bool, String)> = if format == "dot" {
        crate::draw(&model)
            .into_iter()
            .map(|p| (p.name, p.opd, p.dot))
            .collect()
    } else {
        crate::draw_svg(&model)
            .map_err(|m| (1, m))?
            .into_iter()
            .map(|p| (p.name, p.opd, p.svg))
            .collect()
    };
    if let Some(d) = &diagram {
        pictures.retain(|p| &p.0 == d);
        if pictures.is_empty() {
            return Err((1, format!("no OPD named {:?}", d)));
        }
    }

    let bytes = |text: &str| -> Result<Vec<u8>, (u8, String)> {
        if format == "png" {
            crate::svg::svg_to_png(text).map_err(|m| (1, m))
        } else {
            Ok(text.as_bytes().to_vec())
        }
    };
    let write = |p: &Path, b: &[u8]| {
        std::fs::write(p, b).map_err(|e| (2u8, format!("{}: {}", p.display(), e)))
    };

    match (out, pictures.len()) {
        (None, 1) => {
            use std::io::Write;
            std::io::stdout()
                .write_all(&bytes(&pictures[0].2)?)
                .map_err(|e| (2, e.to_string()))
        }
        (None, _) => Err(usage(format!(
            "the model has {} OPDs; give -o <directory> or --diagram",
            pictures.len()
        ))),
        (Some(o), 1) if !Path::new(&o).is_dir() => write(Path::new(&o), &bytes(&pictures[0].2)?),
        (Some(o), _) => {
            let dir = Path::new(&o);
            std::fs::create_dir_all(dir).map_err(|e| (2, format!("{}: {}", o, e)))?;
            let stem = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let stem = stem
                .strip_suffix(".opm.xml")
                .or_else(|| stem.strip_suffix(".xml"))
                .or_else(|| stem.strip_suffix(".opl"))
                .unwrap_or(&stem)
                .to_string();
            for (name, opd, text) in &pictures {
                let name = if *opd {
                    format!("{}.{}.{}", stem, safe(name), format)
                } else {
                    format!("{}.{}", stem, format)
                };
                write(&dir.join(name), &bytes(text)?)?;
            }
            Ok(())
        }
    }
}

/// A picture name usable in a file name.
fn safe(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
