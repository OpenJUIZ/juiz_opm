// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Names, quoting, numbers and generated ids (spec/opl.md §3, §9.3, §10.4).

/// The reserved words of spec/opl.md §3.3.
pub const RESERVED: &[&str] = &[
    "a",
    "affects",
    "an",
    "and",
    "are",
    "as",
    "at",
    "be",
    "can",
    "case",
    "changes",
    "consists",
    "consumed",
    "consumes",
    "default",
    "duration",
    "either",
    "environmental",
    "exactly",
    "exceeds",
    "exhibits",
    "exists",
    "falls",
    "final",
    "finally",
    "from",
    "handle",
    "handles",
    "if",
    "in",
    "informatical",
    "initial",
    "initially",
    "initiates",
    "instance",
    "into",
    "invokes",
    "is",
    "itself",
    "least",
    "many",
    "nothing",
    "object",
    "of",
    "one",
    "optional",
    "or",
    "occurs",
    "otherwise",
    "parallel",
    "path",
    "physical",
    "process",
    "requires",
    "sequence",
    "short",
    "skipped",
    "systemic",
    "that",
    "to",
    "well",
    "which",
    "yields",
    "zooms",
];

/// First words of a Thing name that must be quoted (spec/opl.md §3.1).
const SENTENCE_STARTS: &[&str] = &["Model", "State", "Following", "Either", "Exactly", "At"];

/// Verbs that make a bare tag read as another sentence (spec/opl.md §5.4).
const TAG_VERBS: &[&str] = &[
    "consists",
    "exhibits",
    "consumes",
    "yields",
    "affects",
    "changes",
    "requires",
    "handles",
    "invokes",
    "initiates",
    "occurs",
    "can",
    "zooms",
];

pub fn is_reserved(word: &str) -> bool {
    RESERVED.contains(&word)
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Whether the name can be written bare as a Thing name.
pub fn is_bare_thing(name: &str) -> bool {
    let mut words = name.split(' ');
    let Some(first) = words.next() else {
        return false;
    };
    if SENTENCE_STARTS.contains(&first) {
        return false;
    }
    let ok_word = |w: &str, first: bool| {
        let Some(c) = w.chars().next() else {
            return false;
        };
        (c.is_ascii_uppercase() || (!first && c.is_ascii_digit())) && w.chars().all(is_word_char)
    };
    ok_word(first, true) && words.all(|w| ok_word(w, false))
}

/// Whether the name can be written bare as a State name or a path label.
pub fn is_bare_state(name: &str) -> bool {
    let mut cs = name.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_lowercase())
        && cs.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !is_reserved(name)
}

/// Whether the tag can be written bare (spec/opl.md §5.4).
pub fn is_bare_tag(tag: &str) -> bool {
    let words: Vec<&str> = tag.split(' ').collect();
    let well_formed = words.iter().all(|w| {
        matches!(w.chars().next(), Some(c) if c.is_ascii_lowercase())
            && w.chars().all(|c| c.is_ascii_lowercase() || c == '-')
    });
    if !well_formed {
        return false;
    }
    if TAG_VERBS.contains(&words[0]) {
        return false;
    }
    if words[0] == "is" {
        if let Some(w) = words.get(1) {
            if matches!(
                *w,
                "a" | "an"
                    | "physical"
                    | "informatical"
                    | "environmental"
                    | "systemic"
                    | "object"
                    | "process"
                    | "initially"
                    | "finally"
            ) {
                return false;
            }
        }
    }
    true
}

/// `"..."` with `\"` and `\\` escaped.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

pub fn thing(name: &str) -> String {
    if is_bare_thing(name) {
        name.to_string()
    } else {
        quote(name)
    }
}

pub fn state(name: &str) -> String {
    if is_bare_state(name) {
        name.to_string()
    } else {
        quote(name)
    }
}

pub fn tag(t: &str) -> String {
    if is_bare_tag(t) {
        t.to_string()
    } else {
        quote(t)
    }
}

/// An atom of a `[...]` / `{...}` value: quoted when it holds a character that ends an atom.
pub fn atom(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .all(|c| !c.is_whitespace() && !matches!(c, ',' | '[' | ']' | '{' | '}' | '"'))
    {
        s.to_string()
    } else {
        quote(s)
    }
}

/// Whether a quoted string can hold this text (no line break, spec/opl.md §3.1).
pub fn quotable(s: &str) -> bool {
    !s.contains(['\n', '\r'])
}

/// `a` or `an` before this word (spec/opl.md §5.2).
pub fn article(next: &str) -> &'static str {
    match next.trim_start_matches('"').chars().next() {
        Some(c) if "aeiouAEIOU".contains(c) => "an",
        _ => "a",
    }
}

/// The shortest decimal that reads back as the same number, without a decimal point for an
/// integer (spec/opl.md §9.3).
pub fn number(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    format!("{}", x)
}

/// Parse a number of the OPL grammar or of an XML attribute.
pub fn parse_number(s: &str) -> Option<f64> {
    let ok = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'));
    if !ok {
        return None;
    }
    s.parse::<f64>().ok().filter(|x| x.is_finite())
}

/// The id made from a name (spec/opl.md §10.4): lower-case ASCII letters and digits, every
/// other run of characters one `_`, no `_` at either end; `t` if empty, `t_` before a digit.
pub fn id_from_name(name: &str, empty: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('_');
            }
            gap = false;
            out.push(c.to_ascii_lowercase());
        } else {
            gap = true;
        }
    }
    if out.is_empty() {
        return empty.to_string();
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert_str(0, &format!("{}_", empty));
    }
    out
}

/// The form of an id (spec/xml.md §4).
pub fn is_id(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_things() {
        assert!(is_bare_thing("Main Loop EC Cycle"));
        assert!(is_bare_thing("Camera 2"));
        assert!(is_bare_thing("Base-Arm Transform"));
        assert!(!is_bare_thing("talker0"));
        assert!(!is_bare_thing("Model Train"));
        assert!(!is_bare_thing("Camera  2"));
        assert!(!is_bare_thing("Camera two"));
        assert!(!is_bare_thing(""));
    }

    #[test]
    fn bare_states_and_tags() {
        assert!(is_bare_state("started"));
        assert!(!is_bare_state("is"));
        assert!(!is_bare_state("almost boiling"));
        assert!(is_bare_tag("is located in"));
        assert!(!is_bare_tag("is a part of"));
        assert!(!is_bare_tag("contains Water"));
        assert!(!is_bare_tag("requires power"));
        assert!(is_bare_tag("source"));
    }

    #[test]
    fn ids() {
        assert_eq!(
            id_from_name("Main Loop EC Cycle", "t"),
            "main_loop_ec_cycle"
        );
        assert_eq!(id_from_name("robot1/arm", "t"), "robot1_arm");
        assert_eq!(id_from_name("  ", "t"), "t");
        assert_eq!(id_from_name("2 Arms", "t"), "t_2_arms");
        assert_eq!(id_from_name("almost boiling", "s"), "almost_boiling");
    }

    #[test]
    fn numbers() {
        assert_eq!(number(10.0), "10");
        assert_eq!(number(0.5), "0.5");
        assert_eq!(number(-0.0), "0");
        assert_eq!(number(60.0), "60");
        assert_eq!(parse_number("60.0"), Some(60.0));
        assert_eq!(parse_number("x"), None);
    }
}
