// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! `[...]`: the juiz annotations (spec/opl.md §7) and their XML form (spec/xml.md §7.2).

use opm_model::{ExtAttribute, ExtElement, ExtNode, Extensions, XmlAttribute, JUIZ_NS};

use crate::json;
use crate::lex::{Entry, Value};
use crate::names;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Word,
    Number,
    Bool,
    Json,
    List,
    /// `translation`: x y z.
    Vec3,
    /// `rotation`: x y z w.
    Vec4,
}

impl Form {
    /// Written as a child element in the XML (spec/xml.md §7.1).
    fn is_element(self) -> bool {
        matches!(self, Form::Json | Form::Vec3 | Form::Vec4)
    }
}

/// What a `[...]` belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Thing,
    State,
    Link,
}

/// Keys in the canonical order (spec/opl.md §9.3: `role` first, then the tables of metamodel
/// §9.3 / §9.4, then `status` and `refs`).
const THING_KEYS: &[(&str, Form)] = &[
    ("role", Form::Word),
    ("id", Form::Word),
    ("name", Form::Word),
    ("namespace", Form::Word),
    ("type", Form::Word),
    ("datatype", Form::Word),
    ("value_kind", Form::Word),
    ("value", Form::Json),
    ("rate", Form::Number),
    ("use_memo", Form::Bool),
    ("path", Form::Word),
    ("translation", Form::Vec3),
    ("rotation", Form::Vec4),
    ("writers", Form::List),
    ("params", Form::Json),
    ("default", Form::Json),
    ("status", Form::Word),
    ("refs", Form::List),
];

const LINK_KEYS: &[(&str, Form)] = &[
    ("arg", Form::Word),
    ("arg_index", Form::Number),
    ("connection", Form::Word),
    ("connection_name", Form::Word),
    ("applied", Form::Json),
    ("order", Form::Number),
    ("slot_name", Form::Word),
    ("reference", Form::Word),
    ("initial", Form::Bool),
    ("on_update", Form::Bool),
    ("end", Form::Word),
    ("status", Form::Word),
    ("refs", Form::List),
];

const STATE_KEYS: &[(&str, Form)] = &[("status", Form::Word), ("refs", Form::List)];

pub fn keys(target: Target) -> &'static [(&'static str, Form)] {
    match target {
        Target::Thing => THING_KEYS,
        Target::State => STATE_KEYS,
        Target::Link => LINK_KEYS,
    }
}

fn form_of(target: Target, key: &str) -> Option<Form> {
    keys(target)
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, f)| *f)
}

fn what(target: Target) -> &'static str {
    match target {
        Target::Thing => "a Thing",
        Target::State => "a State",
        Target::Link => "a link",
    }
}

/// Turn the entries of a `[...]` into extensions. Errors are (entry index, message) for O7.
pub fn to_extensions(target: Target, entries: &[Entry]) -> (Extensions, Vec<(usize, String)>) {
    let mut ext = Extensions::default();
    let mut errors = Vec::new();
    // Write in the canonical order whatever the order in the text.
    let mut order: Vec<usize> = (0..entries.len()).collect();
    order.sort_by_key(|&k| {
        keys(target)
            .iter()
            .position(|(n, _)| *n == entries[k].key)
            .unwrap_or(usize::MAX)
    });
    for k in order {
        let e = &entries[k];
        if entries[..k].iter().any(|f| f.key == e.key) {
            errors.push((k, format!("`{}` is written twice", e.key)));
            continue;
        }
        let Some(form) = form_of(target, &e.key) else {
            errors.push((
                k,
                format!("`{}` cannot be written on {}", e.key, what(target)),
            ));
            continue;
        };
        match value_to_xml(form, &e.key, &e.value) {
            Ok(Xml::Attr(v)) => ext.attributes.push(ExtAttribute {
                namespace: JUIZ_NS.to_string(),
                name: e.key.clone(),
                value: v,
            }),
            Ok(Xml::Elem(el)) => ext.elements.push(el),
            Err(m) => errors.push((k, m)),
        }
    }
    (ext, errors)
}

enum Xml {
    Attr(String),
    Elem(ExtElement),
}

fn atoms(v: &Value) -> Option<Vec<&str>> {
    match v {
        Value::Atoms(a) => Some(a.iter().map(|a| a.text.as_str()).collect()),
        Value::Json(_) => None,
    }
}

fn value_to_xml(form: Form, key: &str, v: &Value) -> Result<Xml, String> {
    let bad = |m: &str| format!("`{}` {}", key, m);
    if form == Form::Json {
        return match v {
            Value::Json(j) => Ok(Xml::Elem(ExtElement::with_text(JUIZ_NS, key, j.clone()))),
            Value::Atoms(_) => Err(bad("takes a JSON value")),
        };
    }
    let a = atoms(v).ok_or_else(|| bad("does not take JSON"))?;
    match form {
        Form::Word => match a.as_slice() {
            [w] => Ok(Xml::Attr(w.to_string())),
            _ => Err(bad("takes one word")),
        },
        Form::Number => match a.as_slice() {
            [w] => names::parse_number(w)
                .map(|x| Xml::Attr(names::number(x)))
                .ok_or_else(|| bad("takes a number")),
            _ => Err(bad("takes one number")),
        },
        Form::Bool => match a.as_slice() {
            [w @ ("true" | "false")] => Ok(Xml::Attr(w.to_string())),
            _ => Err(bad("takes `true` or `false`")),
        },
        Form::List => {
            if a.iter().any(|w| w.chars().any(char::is_whitespace)) {
                return Err(bad("takes atoms without white space"));
            }
            Ok(Xml::Attr(a.join(" ")))
        }
        Form::Vec3 | Form::Vec4 => {
            let names: &[&str] = if form == Form::Vec3 {
                &["x", "y", "z"]
            } else {
                &["x", "y", "z", "w"]
            };
            if a.len() != names.len() {
                return Err(bad(&format!("takes {} numbers", names.len())));
            }
            let mut el = ExtElement::new(JUIZ_NS, key);
            for (n, w) in names.iter().zip(&a) {
                let x = names::parse_number(w).ok_or_else(|| bad("takes numbers"))?;
                el.attributes.push(XmlAttribute {
                    namespace: None,
                    name: n.to_string(),
                    value: names::number(x),
                });
            }
            Ok(Xml::Elem(el))
        }
        Form::Json => unreachable!(),
    }
}

/// The `[...]` of an element, without the brackets, or `None` when there is nothing to write.
/// `dropped` gets what cannot be written in OPL (spec/opl.md §10.2); `errors` gets juiz values of
/// a malformed form.
pub fn from_extensions(
    target: Target,
    ext: &Extensions,
    dropped: &mut Vec<String>,
    errors: &mut Vec<String>,
) -> Option<String> {
    let mut parts = Vec::new();
    for (key, form) in keys(target) {
        if form.is_element() {
            let Some(el) = ext.element(JUIZ_NS, key) else {
                continue;
            };
            match element_value(*form, el) {
                Ok(v) => parts.push(format!("{}: {}", key, v)),
                Err(m) => errors.push(format!("juiz:{} {}", key, m)),
            }
        } else if let Some(v) = ext.attribute(JUIZ_NS, key) {
            match attribute_value(*form, v) {
                Ok(v) => parts.push(format!("{}: {}", key, v)),
                Err(m) => errors.push(format!("juiz:{} {}", key, m)),
            }
        }
    }
    for a in &ext.attributes {
        let known =
            a.namespace == JUIZ_NS && form_of(target, &a.name).is_some_and(|f| !f.is_element());
        if !known {
            dropped.push(format!("attribute {{{}}}{}", a.namespace, a.name));
        }
    }
    for (k, e) in ext.elements.iter().enumerate() {
        let known = e.namespace == JUIZ_NS
            && form_of(target, &e.name).is_some_and(Form::is_element)
            && !ext.elements[..k]
                .iter()
                .any(|f| f.namespace == e.namespace && f.name == e.name);
        if !known {
            dropped.push(format!("element {{{}}}{}", e.namespace, e.name));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn attribute_value(form: Form, v: &str) -> Result<String, String> {
    match form {
        Form::Word => {
            if v.is_empty() {
                Err("is empty".to_string())
            } else {
                Ok(names::atom(v))
            }
        }
        Form::Number => names::parse_number(v.trim())
            .map(names::number)
            .ok_or_else(|| format!("{:?} is not a number", v)),
        Form::Bool => match v.trim() {
            "true" | "1" => Ok("true".to_string()),
            "false" | "0" => Ok("false".to_string()),
            _ => Err(format!("{:?} is not a boolean", v)),
        },
        Form::List => {
            let items: Vec<String> = v.split_whitespace().map(names::atom).collect();
            if items.is_empty() {
                Err("is empty".to_string())
            } else {
                Ok(items.join(" "))
            }
        }
        _ => unreachable!(),
    }
}

fn element_value(form: Form, el: &ExtElement) -> Result<String, String> {
    match form {
        Form::Json => {
            if el.children.iter().any(|c| matches!(c, ExtNode::Element(_))) {
                return Err("holds elements, not JSON text".to_string());
            }
            json::canonical(&el.text()).map_err(|m| format!("is not JSON: {}", m))
        }
        Form::Vec3 | Form::Vec4 => {
            let names: &[(&str, f64)] = if form == Form::Vec3 {
                &[("x", 0.0), ("y", 0.0), ("z", 0.0)]
            } else {
                &[("x", 0.0), ("y", 0.0), ("z", 0.0), ("w", 1.0)]
            };
            let mut out = Vec::new();
            for (n, d) in names {
                let x = match el.attribute(None, n) {
                    Some(v) => names::parse_number(v.trim())
                        .ok_or_else(|| format!("{}={:?} is not a number", n, v))?,
                    None => *d,
                };
                out.push(names::number(x));
            }
            Ok(out.join(" "))
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Pos;
    use crate::lex::{Atom, Entry};

    fn entry(k: &str, v: Value) -> Entry {
        Entry {
            key: k.into(),
            value: v,
            pos: Pos::default(),
        }
    }

    fn atoms(v: &[&str]) -> Value {
        Value::Atoms(
            v.iter()
                .map(|t| Atom {
                    text: t.to_string(),
                    quoted: false,
                })
                .collect(),
        )
    }

    #[test]
    fn round_trip_of_annotations() {
        let es = vec![
            entry("type", atoms(&["arm"])),
            entry("role", atoms(&["transform"])),
            entry("translation", atoms(&["0.1", "0", "0.50"])),
            entry("refs", atoms(&["juiz#1", "juiz#2"])),
        ];
        let (ext, errs) = to_extensions(Target::Thing, &es);
        assert!(errs.is_empty());
        assert_eq!(ext.attributes[0].name, "role");
        let mut d = Vec::new();
        let mut e = Vec::new();
        let s = from_extensions(Target::Thing, &ext, &mut d, &mut e).unwrap();
        assert_eq!(
            s,
            "role: transform, type: arm, translation: 0.1 0 0.5, refs: juiz#1 juiz#2"
        );
        assert!(d.is_empty() && e.is_empty());
    }

    #[test]
    fn wrong_keys() {
        let es = vec![entry("arg", atoms(&["x"])), entry("rate", atoms(&["fast"]))];
        let (_, errs) = to_extensions(Target::Thing, &es);
        assert_eq!(errs.len(), 2);
    }
}
