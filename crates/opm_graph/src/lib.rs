// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Draw OPM models (spec/diagram.md).
//!
//! - [`view`]: what one picture shows (§2): the Things and links of one OPD, or of the whole
//!   model when it has no OPD.
//! - [`abbrev`]: a view gathered into the symbols of the abbreviated diagram (§3).
//! - [`layout`]: Graphviz places the symbols; [`draw`] writes the SVG.
//! - [`dot`]: a view as Graphviz DOT, in the OPM form (§3.11) with the status colors (§3.10).
//!   Views without juiz elements are drawn from it.
//! - [`svg`]: the outside commands (Graphviz `dot`, `rsvg-convert`).
//! - [`opx`]: the whole model as an OPCAT file (OPX), to open in OPCAT or OPCloud.
//!
//! [`draw_svg`] draws the abbreviated diagram; [`draw`] gives the DOT in the OPM form.
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

pub mod abbrev;
pub mod dot;
pub mod draw;
pub mod layout;
pub mod opx;
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

/// Draw one view as the abbreviated diagram (spec/diagram.md §3) in SVG. Graphviz `dot`
/// places the symbols.
///
/// A view without any juiz element has nothing to gather, so it is the OPM form (§3.11); it is
/// drawn from [`dot::to_dot`] by Graphviz, which draws the OPM shapes better (decision 0014).
pub fn abbreviated_svg(view: &view::View) -> Result<String, String> {
    let juiz = view
        .things
        .iter()
        .any(|t| t.extensions.attribute(opm_model::JUIZ_NS, "role").is_some());
    if !juiz {
        return svg::to_svg(&dot::to_dot(view));
    }
    let scene = abbrev::build(view);
    let sizes = draw::sizes(&scene);
    let dot = layout::layout_dot(&scene, &sizes, &|id| draw::frame_label(&scene, id));
    let place = layout::place(&dot, &scene)?;
    Ok(draw::to_svg(&scene, &place))
}

/// One abbreviated diagram in SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawing {
    /// As [`Picture::name`].
    pub name: String,
    pub opd: bool,
    pub svg: String,
}

/// Every OPD of the model as the abbreviated diagram in SVG, root first (see [`draw`]).
pub fn draw_svg(model: &Model) -> Result<Vec<Drawing>, String> {
    view::views(model)
        .iter()
        .map(|v| {
            Ok(Drawing {
                name: v.name(),
                opd: v.diagram.is_some(),
                svg: abbreviated_svg(v)?,
            })
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
