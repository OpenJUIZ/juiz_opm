// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Draw OPM models (spec/diagram.md).
//!
//! - [`view`]: what one picture shows (§2): the Things and links of one OPD, or of the whole
//!   model when it has no OPD.
//! - [`dot`]: a view as Graphviz DOT, in the OPM form (§3.11) with the status colors (§3.10).
//! - [`svg`]: DOT to SVG through the Graphviz `dot` command.
//!
//! The juiz symbols of the abbreviated diagram (§3.1 to §3.9) come in Step 6b. Until then every
//! model is drawn in the OPM form, as a model without juiz annotations is.
//!
//! ```
//! let text = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
//!                          format_version="0.1" id="m" name="Coffee">
//!   <opm:things>
//!     <opm:process id="p" name="Making"/>
//!     <opm:object id="o" name="Coffee"/>
//!   </opm:things>
//!   <opm:links><opm:result id="l" source="p" target="o"/></opm:links>
//! </opm:model>"#;
//! let model = opm_model::read::from_str(text).unwrap();
//! let pictures = opm_graph::draw(&model);
//! assert_eq!(pictures.len(), 1);
//! assert!(pictures[0].dot.contains("t0 -> t1 [arrowhead=normal]"));
//! ```

pub mod dot;
pub mod svg;
pub mod view;

use opm_model::Model;

/// One picture: one OPD, or the whole model.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    /// The OPD name (`SD`, `SD1`, ...), or the model name when there is no OPD.
    pub name: String,
    /// Whether the picture is an OPD (`false`: the whole model, which has no OPD).
    pub opd: bool,
    pub dot: String,
}

/// Draw every OPD of the model as DOT, root first. A model without OPDs gives one picture of
/// everything.
pub fn draw(model: &Model) -> Vec<Picture> {
    view::views(model)
        .iter()
        .map(|v| Picture {
            name: v.name(),
            opd: v.diagram.is_some(),
            dot: dot::to_dot(v),
        })
        .collect()
}

/// Read a model from juiz-OPL (`.opl`) or the interchange XML (anything else). Returns the
/// errors as text.
pub fn read_model(path: &std::path::Path) -> Result<Model, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    if path.extension().is_some_and(|e| e == "opl") {
        let p = opm_opl::parse(&text);
        if opm_opl::has_error(&p.diagnostics) {
            let errs: Vec<String> = p
                .diagnostics
                .iter()
                .map(|d| format!("{}:{}", path.display(), d))
                .collect();
            return Err(errs.join("\n"));
        }
        Ok(p.model)
    } else {
        opm_model::read::from_str(&text).map_err(|e| format!("{}: {}", path.display(), e))
    }
}
