// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! OPM (ISO 19450) metamodel and the juiz_opm XML interchange format.
//!
//! - [`model`]: the types of the metamodel (spec/metamodel.md), core and diagram layers.
//! - [`read`] / [`write`]: the interchange XML (spec/xml.md).
//! - [`check`]: the core well-formedness rules C1 to C12 (spec/metamodel.md §8).
//!
//! Annotations of other namespaces, such as juiz, are kept as [`Extensions`] without being
//! interpreted, and written back as they were read.
//!
//! ```
//! let text = r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
//!                          format_version="0.1" id="m">
//!   <opm:things>
//!     <opm:process id="p" name="Making"/>
//!     <opm:object id="o" name="Coffee"/>
//!   </opm:things>
//!   <opm:links><opm:result id="l" source="p" target="o"/></opm:links>
//! </opm:model>"#;
//! let model = opm_model::read::from_str(text).unwrap();
//! assert!(opm_model::check(&model).is_ok());
//! let again = opm_model::read::from_str(&opm_model::write::to_string(&model)).unwrap();
//! assert_eq!(model, again);
//! ```

pub mod check;
pub mod ext;
pub mod model;
pub mod read;
pub mod write;

pub use check::{check, Diagnostic, Report, Rule, Severity};
pub use ext::{ExtAttribute, ExtElement, ExtNode, Extensions, XmlAttribute};
pub use model::*;
pub use read::ReadError;
