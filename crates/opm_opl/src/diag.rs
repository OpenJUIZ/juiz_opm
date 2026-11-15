// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Diagnostics of juiz-OPL (spec/opl.md §8, §14.1 of this crate's spec section).

use std::fmt;

pub use opm_model::Severity;

/// A position in an OPL document: line and column, both from 1. Columns count characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Pos {
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// What a diagnostic is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Code {
    /// The text does not follow the grammar (spec/opl.md §3, §4).
    Syntax,
    /// A well-formedness rule of OPL, O1 to O12 (spec/opl.md §8).
    O(u8),
    /// A core rule of the model, checked by opm_model after reading (metamodel §8).
    Model(opm_model::Rule),
    /// Something of the model that OPL cannot write (spec/opl.md §10.2).
    Unwritable,
    /// Something of the model that is left out when writing OPL (spec/opl.md §10.2).
    Dropped,
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Code::Syntax => f.write_str("syntax"),
            Code::O(n) => write!(f, "O{}", n),
            Code::Model(r) => write!(f, "{}", r),
            Code::Unwritable => f.write_str("unwritable"),
            Code::Dropped => f.write_str("dropped"),
        }
    }
}

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: Code,
    pub severity: Severity,
    /// Where in the OPL text, when the finding comes from one.
    pub pos: Option<Pos>,
    /// The id of the model element, when known.
    pub id: Option<String>,
    pub message: String,
}

impl Diagnostic {
    pub fn error(code: Code, pos: Option<Pos>, message: impl Into<String>) -> Self {
        Diagnostic {
            code,
            severity: Severity::Error,
            pos,
            id: None,
            message: message.into(),
        }
    }

    pub fn warning(code: Code, pos: Option<Pos>, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            ..Self::error(code, pos, message)
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// `3:14: error[O1] capture: "Capture" has no declaration`. The position and the id are left
/// out when unknown.
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(p) = self.pos {
            write!(f, "{}: ", p)?;
        }
        let s = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{}[{}]", s, self.code)?;
        if let Some(id) = &self.id {
            write!(f, " {}", id)?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Whether any of these is an error.
pub fn has_error(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(Diagnostic::is_error)
}
