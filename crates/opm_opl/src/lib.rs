// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! juiz-OPL (spec/opl.md): read, check, write in the canonical form, and convert to and from
//! the interchange XML of [`opm_model`].
//!
//! ```
//! let text = "Making is a process.\nMaking yields Coffee.\nCoffee is a physical object.\n";
//! let read = opm_opl::parse(text);
//! assert!(read.diagnostics.is_empty());
//! assert_eq!(read.model.links[0].id, "result-making-coffee");
//! let out = opm_opl::write(&read.model, opm_opl::Options::default());
//! assert_eq!(
//!     out.text,
//!     "Coffee is a physical object.\n\nMaking is a process.\nMaking yields Coffee.\n"
//! );
//! ```

pub mod build;
pub mod diag;
pub mod json;
pub mod juiz;
pub mod lex;
pub mod names;
pub mod parse;
pub mod write;

use std::collections::HashMap;

pub use diag::{has_error, Code, Diagnostic, Pos, Severity};
use opm_model::Model;
pub use write::{Options, Output};

/// A model read from OPL, with what was found while reading.
#[derive(Debug, Clone)]
pub struct Parsed {
    pub model: Model,
    /// Element id → the sentence that wrote it.
    pub positions: HashMap<String, Pos>,
    /// Some element had `{id: ...}` (the "with ids" form, spec/opl.md §10.4).
    pub explicit_ids: bool,
    pub diagnostics: Vec<Diagnostic>,
}

impl Parsed {
    pub fn is_ok(&self) -> bool {
        !has_error(&self.diagnostics)
    }
}

/// Read OPL: the grammar (§4) and the rules O1 to O12 (§8). The model is complete only when
/// there is no error.
pub fn parse(text: &str) -> Parsed {
    let tokens = match lex::tokenize(text) {
        Ok(t) => t,
        Err(d) => {
            return Parsed {
                model: Model::new("opm-model"),
                positions: HashMap::new(),
                explicit_ids: false,
                diagnostics: vec![d],
            }
        }
    };
    let (doc, mut diagnostics) = parse::parse(tokens);
    let (built, more) = build::build(&doc);
    diagnostics.extend(more);
    diagnostics.sort_by_key(|d| d.pos);
    Parsed {
        model: built.model,
        positions: built.positions,
        explicit_ids: built.explicit_ids,
        diagnostics,
    }
}

/// [`parse`], then the core rules of the model (opm_model's C1 to C12) when reading found no
/// error. The juiz rules J1 to J13 are checked by the juiz_opm crate.
pub fn check(text: &str) -> Parsed {
    let mut p = parse(text);
    if p.is_ok() {
        let report = opm_model::check(&p.model);
        for d in report.diagnostics {
            p.diagnostics.push(Diagnostic {
                code: Code::Model(d.rule),
                severity: d.severity,
                pos: p.positions.get(&d.id).copied(),
                id: Some(d.id),
                message: d.message,
            });
        }
    }
    p
}

/// Write a model as OPL in the canonical form (§9).
pub fn write(model: &Model, options: Options) -> Output {
    write::to_string(model, options)
}

/// Rewrite OPL in the canonical form. With `ids: None` the ids are written when the text had
/// some `{id: ...}`.
pub fn format(text: &str, ids: Option<bool>) -> Result<String, Vec<Diagnostic>> {
    let p = parse(text);
    if !p.is_ok() {
        return Err(p.diagnostics);
    }
    let out = write(
        &p.model,
        Options {
            ids: ids.unwrap_or(p.explicit_ids),
        },
    );
    if has_error(&out.diagnostics) {
        return Err(out.diagnostics);
    }
    Ok(out.text)
}

/// OPL to the interchange XML. Only the errors of reading stop it; warnings are returned.
pub fn to_xml(text: &str) -> Result<(String, Vec<Diagnostic>), Vec<Diagnostic>> {
    let p = check(text);
    if !p.is_ok() {
        return Err(p.diagnostics);
    }
    Ok((opm_model::write::to_string(&p.model), p.diagnostics))
}

/// The interchange XML to OPL. What OPL cannot hold is reported as warnings (dropped) or
/// errors (spec/opl.md §10.2).
pub fn from_xml(xml: &str, options: Options) -> Result<Output, Vec<Diagnostic>> {
    let model = opm_model::read::from_str(xml)
        .map_err(|e| vec![Diagnostic::error(Code::Syntax, None, format!("XML: {}", e))])?;
    let out = write(&model, options);
    if has_error(&out.diagnostics) {
        return Err(out.diagnostics);
    }
    Ok(out)
}
