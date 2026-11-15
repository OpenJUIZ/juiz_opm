// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! juiz_opm: the juiz conf to and from the OPM model of juiz_opm (spec/conf.md).
//!
//! - [`conf`]: the conf in the round-trip range, read as juiz reads it and written in the
//!   canonical form.
//! - [`to_model`] / [`to_conf`]: conf → model and model → conf.
//! - [`check`]: the juiz rules J1 to J13 (spec/metamodel.md §9.6).

pub mod check;
pub mod conf;
pub mod from_model;
pub mod to_model;

pub use check::check;
pub use conf::{Conf, Notes};
pub use from_model::to_conf;
pub use to_model::to_model;

/// conf text → model, with what reading did not carry.
pub fn model_of_conf(text: &str) -> Result<(opm_model::Model, Notes), Notes> {
    let (conf, mut notes) = Conf::read(text);
    let Some(conf) = conf else { return Err(notes) };
    match to_model(&conf) {
        Ok(m) => Ok((m, notes)),
        Err(e) => {
            notes.errors.extend(e);
            Err(notes)
        }
    }
}

/// conf text → juiz-OPL in the canonical form.
pub fn opl_of_conf(text: &str) -> Result<(String, Notes), Notes> {
    let (m, notes) = model_of_conf(text)?;
    Ok((opm_opl::write(&m, opm_opl::Options::default()).text, notes))
}

/// The canonical form of a conf (spec/conf.md §3).
pub fn canonical_conf(text: &str) -> Result<(String, Notes), Notes> {
    let (conf, notes) = Conf::read(text);
    match conf {
        Some(c) => Ok((c.write(), notes)),
        None => Err(notes),
    }
}

/// The round trips of spec/conf.md §7 for one conf: conf → model → conf, conf → OPL → conf and
/// conf → XML → conf all give the canonical conf, and the model breaks no core or juiz rule.
pub fn round_trip(text: &str) -> Result<(), String> {
    let errors = |n: Notes| n.errors.join("\n");
    let (canonical, _) = canonical_conf(text).map_err(errors)?;
    let (model, _) = model_of_conf(text).map_err(errors)?;

    let mut problems = Vec::new();
    let core = opm_model::check(&model);
    for d in core.errors() {
        problems.push(format!("core rule: {}", d));
    }
    for d in check(&model) {
        problems.push(format!("juiz rule: {}", d));
    }

    let opl = opm_opl::write(&model, opm_opl::Options::default()).text;
    let parsed = opm_opl::parse(&opl);
    if !parsed.is_ok() {
        problems.push("the OPL does not read back".to_string());
    }
    let xml = opm_model::write::to_string(&model);
    let from_xml = opm_model::read::from_str(&xml).map_err(|e| format!("XML: {}", e))?;

    for (what, m) in [
        ("model", &model),
        ("OPL", &parsed.model),
        ("XML", &from_xml),
    ] {
        match to_conf(m) {
            Ok(c) => {
                let back = c.write();
                if back != canonical {
                    problems.push(format!(
                        "conf -> {} -> conf differs from the canonical conf:\n{}",
                        what,
                        first_difference(&canonical, &back)
                    ));
                }
            }
            Err(es) => problems.push(format!("{} -> conf: {}", what, es.join("; "))),
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// The first differing line of two texts, for messages.
fn first_difference(a: &str, b: &str) -> String {
    let (la, lb): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    let n = la.iter().zip(&lb).take_while(|(x, y)| x == y).count();
    format!(
        "  line {}:\n  - {}\n  + {}",
        n + 1,
        la.get(n).unwrap_or(&"(end)"),
        lb.get(n).unwrap_or(&"(end)")
    )
}
