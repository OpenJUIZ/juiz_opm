// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Tokens of juiz-OPL (spec/opl.md §2, §3). `{...}` and `[...]` are read whole into a
//! [`Block`], because their values (atoms and JSON) have their own lexical form (§7.3).

use crate::diag::{Code, Diagnostic, Pos};
use crate::json;

/// Keys of `[...]` whose value is JSON (spec/opl.md §7.3).
pub const JSON_KEYS: &[&str] = &["value", "applied", "params", "default"];

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    /// A bare word: letters, digits, `_` and `-`, not all digits.
    Word(String),
    /// A quoted name, tag or value, without the quotes and with the escapes resolved.
    Quoted(String),
    /// Digits, with an optional fraction (`60`, `60.0`).
    Num(String),
    /// A multiplicity in symbol form (`0..1`, `2..*`, `*`).
    Range(String),
    Comma,
    Period,
    Block(Block),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub pos: Pos,
}

/// `{...}` (core) or `[...]` (juiz).
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub juiz: bool,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: String,
    pub value: Value,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A JSON value without white space.
    Json(String),
    /// One or more atoms.
    Atoms(Vec<Atom>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Atom {
    pub text: String,
    pub quoted: bool,
}

pub fn tokenize(text: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut lx = Lexer {
        cs: text.chars().collect(),
        i: 0,
        line: 1,
        col: 1,
        line_start: true,
    };
    let mut out = Vec::new();
    loop {
        lx.skip();
        let pos = lx.pos();
        let Some(c) = lx.peek() else {
            out.push(Token { tok: Tok::Eof, pos });
            return Ok(out);
        };
        let tok = match c {
            '.' => {
                lx.bump();
                Tok::Period
            }
            ',' => {
                lx.bump();
                Tok::Comma
            }
            '*' => {
                lx.bump();
                Tok::Range("*".to_string())
            }
            '"' => Tok::Quoted(lx.quoted()?),
            '{' | '[' => Tok::Block(lx.block()?),
            c if c.is_ascii_digit() => lx.number_or_word(),
            c if c.is_ascii_alphabetic() || c == '_' => Tok::Word(lx.word()),
            c => {
                return Err(Diagnostic::error(
                    Code::Syntax,
                    Some(pos),
                    format!("unexpected character {:?}", c),
                ))
            }
        };
        out.push(Token { tok, pos });
    }
}

struct Lexer {
    cs: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
    /// Only white space since the start of the line.
    line_start: bool,
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

impl Lexer {
    fn pos(&self) -> Pos {
        Pos {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.cs.get(self.i).copied()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.cs.get(self.i + n).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
            self.line_start = true;
        } else {
            self.col += 1;
            if !c.is_whitespace() {
                self.line_start = false;
            }
        }
        Some(c)
    }

    /// White space and comment lines (spec/opl.md §2).
    fn skip(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else if c == '#' && self.line_start {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.bump();
                }
            } else {
                break;
            }
        }
    }

    /// White space only, inside a block.
    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }

    fn err(&self, pos: Pos, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(Code::Syntax, Some(pos), msg)
    }

    fn word(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek().filter(|&c| is_word_char(c)) {
            s.push(c);
            self.bump();
        }
        s
    }

    fn number_or_word(&mut self) -> Tok {
        let w = self.word();
        if !w.chars().all(|c| c.is_ascii_digit()) {
            return Tok::Word(w);
        }
        // `2..4`, `2..*`
        if self.peek() == Some('.') && self.peek_at(1) == Some('.') {
            self.bump();
            self.bump();
            let hi = if self.peek() == Some('*') {
                self.bump();
                "*".to_string()
            } else {
                self.word()
            };
            return Tok::Range(format!("{}..{}", w, hi));
        }
        // `60.5`; a `.` not followed by a digit ends the sentence.
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            let f = self.word();
            return Tok::Num(format!("{}.{}", w, f));
        }
        Tok::Num(w)
    }

    fn quoted(&mut self) -> Result<String, Diagnostic> {
        let start = self.pos();
        self.bump();
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') | Some('\r') => {
                    return Err(self.err(start, "unterminated quoted name"))
                }
                Some('"') => {
                    self.bump();
                    break;
                }
                Some('\\') => {
                    self.bump();
                    match self.peek() {
                        Some(c @ ('"' | '\\')) => {
                            s.push(c);
                            self.bump();
                        }
                        _ => {
                            return Err(self.err(self.pos(), "only \\\" and \\\\ are escapes"));
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.bump();
                }
            }
        }
        if s.is_empty() {
            return Err(self.err(start, "a quoted name must not be empty"));
        }
        Ok(s)
    }

    fn block(&mut self) -> Result<Block, Diagnostic> {
        let open = self.bump().expect("block start");
        let juiz = open == '[';
        let close = if juiz { ']' } else { '}' };
        let mut entries = Vec::new();
        loop {
            self.skip_ws();
            let pos = self.pos();
            let key = self.word();
            if key.is_empty() {
                return Err(self.err(pos, "expected a key"));
            }
            if !key.starts_with(|c: char| c.is_ascii_lowercase())
                || !key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                return Err(self.err(pos, format!("malformed key {:?}", key)));
            }
            self.skip_ws();
            if self.peek() != Some(':') {
                return Err(self.err(self.pos(), format!("expected `:` after {:?}", key)));
            }
            self.bump();
            self.skip_ws();
            let value = if juiz && JSON_KEYS.contains(&key.as_str()) {
                self.json()?
            } else {
                self.atoms(close)?
            };
            entries.push(Entry { key, value, pos });
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some(c) if c == close => {
                    self.bump();
                    return Ok(Block { juiz, entries });
                }
                _ => {
                    return Err(self.err(self.pos(), format!("expected `,` or `{}`", close)));
                }
            }
        }
    }

    fn json(&mut self) -> Result<Value, Diagnostic> {
        let pos = self.pos();
        let rest: String = self.cs[self.i..].iter().collect();
        let (v, n) = json::scan(&rest).map_err(|m| self.err(pos, m))?;
        let chars = rest[..n].chars().count();
        for _ in 0..chars {
            self.bump();
        }
        Ok(Value::Json(v))
    }

    fn atoms(&mut self, close: char) -> Result<Value, Diagnostic> {
        let mut atoms = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None => return Err(self.err(self.pos(), "unterminated block")),
                Some(c) if c == ',' || c == close => break,
                Some('"') => {
                    let text = self.quoted()?;
                    atoms.push(Atom { text, quoted: true });
                }
                Some(c) if matches!(c, '[' | ']' | '{' | '}') => {
                    return Err(self.err(self.pos(), format!("unexpected {:?} in a value", c)));
                }
                Some(_) => {
                    let mut text = String::new();
                    while let Some(c) = self.peek().filter(|&c| {
                        !c.is_whitespace() && !matches!(c, ',' | '[' | ']' | '{' | '}' | '"')
                    }) {
                        text.push(c);
                        self.bump();
                    }
                    atoms.push(Atom {
                        text,
                        quoted: false,
                    });
                }
            }
        }
        if atoms.is_empty() {
            return Err(self.err(self.pos(), "expected a value"));
        }
        Ok(Value::Atoms(atoms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        tokenize(s).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn words_numbers_ranges() {
        assert_eq!(
            toks("Camera 2 consists of 0..1 Arm.\n# comment\nX exceeds 60.5 min."),
            vec![
                Tok::Word("Camera".into()),
                Tok::Num("2".into()),
                Tok::Word("consists".into()),
                Tok::Word("of".into()),
                Tok::Range("0..1".into()),
                Tok::Word("Arm".into()),
                Tok::Period,
                Tok::Word("X".into()),
                Tok::Word("exceeds".into()),
                Tok::Num("60.5".into()),
                Tok::Word("min".into()),
                Tok::Period,
                Tok::Eof,
            ]
        );
    }

    #[test]
    fn blocks() {
        let t = toks(
            r#"A {id: a, description: "x \"y\""} [applied: { "k" : [1, 2] }, refs: juiz#1 juiz#2]."#,
        );
        let Tok::Block(core) = &t[1] else { panic!() };
        assert!(!core.juiz);
        assert_eq!(core.entries[1].key, "description");
        assert_eq!(
            core.entries[1].value,
            Value::Atoms(vec![Atom {
                text: "x \"y\"".into(),
                quoted: true
            }])
        );
        let Tok::Block(j) = &t[2] else { panic!() };
        assert_eq!(j.entries[0].value, Value::Json(r#"{"k":[1,2]}"#.into()));
        assert_eq!(
            j.entries[1].value,
            Value::Atoms(vec![
                Atom {
                    text: "juiz#1".into(),
                    quoted: false
                },
                Atom {
                    text: "juiz#2".into(),
                    quoted: false
                }
            ])
        );
    }

    #[test]
    fn hash_inside_a_line_is_not_a_comment() {
        assert_eq!(toks("A [refs: juiz#3].").len(), 4);
    }
}
