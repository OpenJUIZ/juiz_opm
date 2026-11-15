// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `opm_graph`: draw an OPM model (juiz-OPL or the interchange XML) as the abbreviated
//! diagram (SVG or PNG), or as DOT in the OPM form.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  opm_graph [-T dot|svg|png] [--diagram <name>] [-o <out>] <file.opl | file.xml>

  Draws every OPD of the model (root first), or the whole model when it has no OPD.
  -T          output format (default svg). svg and png are the abbreviated diagram
              (spec/diagram.md); dot is the OPM form as Graphviz DOT. All run Graphviz
              `dot` (or OPM_GRAPH_DOT) but dot; png also runs rsvg-convert
              (or OPM_GRAPH_RSVG).
  --diagram   draw only the OPD of this name (SD, SD1, ...)
  -o          output file for one picture, or a directory for several
              (named <model file>.<OPD name>.<format>, or <model file>.<format>
              for a model without OPDs). Without -o, one picture goes to
              stdout; several need -o.

exit status: 0 ok, 1 the model could not be read or drawn, 2 bad usage or I/O error";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, m)) => {
            eprintln!("opm_graph: {}", m);
            if code == 2 {
                eprintln!("\n{}", USAGE);
            }
            ExitCode::from(code)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), (u8, String)> {
    let usage = |m: String| (2u8, m);
    let mut format = "svg".to_string();
    let mut diagram = None;
    let mut out = None;
    let mut file = None;
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{}", USAGE);
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
    if !["dot", "svg", "png"].contains(&format.as_str()) {
        return Err(usage(format!("unknown format {:?}", format)));
    }
    let file = file.ok_or_else(|| usage("no model file".into()))?;
    let model = opm_graph::read_model(&file).map_err(|m| (1, m))?;
    // (name, is an OPD, text): DOT in the OPM form, or the abbreviated diagram in SVG
    let mut pictures: Vec<(String, bool, String)> = if format == "dot" {
        opm_graph::draw(&model)
            .into_iter()
            .map(|p| (p.name, p.opd, p.dot))
            .collect()
    } else {
        opm_graph::draw_svg(&model)
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
            opm_graph::svg::svg_to_png(text).map_err(|m| (1, m))
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
