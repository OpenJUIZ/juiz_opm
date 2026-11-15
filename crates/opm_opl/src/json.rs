// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! JSON values of `[...]` (spec/opl.md §7.3, §9.3).
//!
//! A value is checked against RFC 8259 and written without white space. Strings and numbers are
//! kept as they were written and the keys of an object keep their order, so nothing but the
//! white space changes.

/// Read one JSON value at the start of `text` (after white space). Returns the value without
/// white space and the number of bytes read, or a message.
pub fn scan(text: &str) -> Result<(String, usize), String> {
    let mut s = Scanner {
        b: text.as_bytes(),
        i: 0,
        out: String::new(),
    };
    s.ws();
    s.value(0)?;
    Ok((s.out, s.i))
}

/// The value of a whole string, which must hold exactly one JSON value.
pub fn canonical(text: &str) -> Result<String, String> {
    let (v, n) = scan(text)?;
    if !text[n..].trim().is_empty() {
        return Err("text after the JSON value".to_string());
    }
    Ok(v)
}

struct Scanner<'a> {
    b: &'a [u8],
    i: usize,
    out: String,
}

const MAX_DEPTH: usize = 128;

impl Scanner<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn push_slice(&mut self, from: usize) {
        // The slice starts and ends on ASCII bytes, or inside a string where it is copied whole.
        self.out
            .push_str(std::str::from_utf8(&self.b[from..self.i]).expect("utf-8"));
    }

    fn value(&mut self, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("JSON nested too deep".to_string());
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string(),
            Some(b't') => self.literal("true"),
            Some(b'f') => self.literal("false"),
            Some(b'n') => self.literal("null"),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err("expected a JSON value".to_string()),
        }
    }

    fn literal(&mut self, word: &str) -> Result<(), String> {
        if self.b[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            self.out.push_str(word);
            Ok(())
        } else {
            Err("expected a JSON value".to_string())
        }
    }

    fn number(&mut self) -> Result<(), String> {
        let from = self.i;
        let bad = || "a malformed JSON number".to_string();
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        match self.peek() {
            Some(b'0') => self.i += 1,
            Some(c) if c.is_ascii_digit() => self.digits(),
            _ => return Err(bad()),
        }
        if self.peek() == Some(b'.') {
            self.i += 1;
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(bad());
            }
            self.digits();
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
                return Err(bad());
            }
            self.digits();
        }
        self.push_slice(from);
        Ok(())
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
    }

    fn string(&mut self) -> Result<(), String> {
        let from = self.i;
        self.i += 1;
        loop {
            match self.peek() {
                None => return Err("an unterminated JSON string".to_string()),
                Some(b'"') => {
                    self.i += 1;
                    break;
                }
                Some(b'\\') => {
                    self.i += 1;
                    match self.peek() {
                        Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.i += 1,
                        Some(b'u') => {
                            self.i += 1;
                            for _ in 0..4 {
                                if !self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                                    return Err("a malformed \\u escape".to_string());
                                }
                                self.i += 1;
                            }
                        }
                        _ => return Err("a malformed escape in a JSON string".to_string()),
                    }
                }
                Some(c) if c < 0x20 => {
                    return Err("a control character in a JSON string".to_string())
                }
                Some(_) => self.i += 1,
            }
        }
        self.push_slice(from);
        Ok(())
    }

    fn array(&mut self, depth: usize) -> Result<(), String> {
        self.i += 1;
        self.out.push('[');
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.out.push(']');
            return Ok(());
        }
        loop {
            self.ws();
            self.value(depth + 1)?;
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.out.push(',');
                }
                Some(b']') => {
                    self.i += 1;
                    self.out.push(']');
                    return Ok(());
                }
                _ => return Err("expected `,` or `]` in a JSON array".to_string()),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), String> {
        self.i += 1;
        self.out.push('{');
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.out.push('}');
            return Ok(());
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err("expected a key in a JSON object".to_string());
            }
            self.string()?;
            self.ws();
            if self.peek() != Some(b':') {
                return Err("expected `:` in a JSON object".to_string());
            }
            self.i += 1;
            self.out.push(':');
            self.ws();
            self.value(depth + 1)?;
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                    self.out.push(',');
                }
                Some(b'}') => {
                    self.i += 1;
                    self.out.push('}');
                    return Ok(());
                }
                _ => return Err("expected `,` or `}` in a JSON object".to_string()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_values() {
        assert_eq!(
            canonical(r#" { "a" : 1, "b": [1, 2 ] } "#).unwrap(),
            r#"{"a":1,"b":[1,2]}"#
        );
        assert_eq!(
            canonical(r#""COLOR_BGR2GRAY""#).unwrap(),
            r#""COLOR_BGR2GRAY""#
        );
        assert_eq!(canonical("1.50e3").unwrap(), "1.50e3");
        assert_eq!(canonical(r#""a b\"c""#).unwrap(), r#""a b\"c""#);
        assert!(canonical("01").is_err());
        assert!(canonical("[1,]").is_err());
        assert!(canonical("COLOR").is_err());
    }

    #[test]
    fn scan_stops_after_value() {
        let (v, n) = scan(r#"{"x": 3}, name: a]"#).unwrap();
        assert_eq!(v, r#"{"x":3}"#);
        assert_eq!(n, 8);
    }
}
