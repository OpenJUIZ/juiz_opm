// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! The grammar of juiz-OPL (spec/opl.md §4) into a syntax tree. Names are not resolved here;
//! [`crate::build`] does that once every declaration is known (§8).

use opm_model::{FanOperator, LinkKind, Modifier};

use crate::diag::{Code, Diagnostic, Pos};
use crate::lex::{Block, Tok, Token};
use crate::names;

/// A Thing, State or label name as written, without quotes.
#[derive(Debug, Clone, PartialEq)]
pub struct Name {
    pub text: String,
    pub pos: Pos,
}

/// `[multiplicity] [state] thing`.
#[derive(Debug, Clone, PartialEq)]
pub struct End {
    /// In symbol form (`2..*`).
    pub mult: Option<String>,
    pub state: Option<Name>,
    pub thing: Name,
}

impl End {
    fn plain(thing: Name) -> Self {
        End {
            mult: None,
            state: None,
            thing,
        }
    }

    fn same(&self, other: &End) -> bool {
        self.thing.text == other.thing.text
            && self.state.as_ref().map(|s| &s.text) == other.state.as_ref().map(|s| &s.text)
    }

    fn show(&self) -> String {
        match &self.state {
            Some(s) => format!("{} {}", s.text, self.thing.text),
            None => self.thing.text.clone(),
        }
    }
}

/// One link as a sentence says it, with the ends still names.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkSpec {
    pub kind: LinkKind,
    pub modifier: Option<Modifier>,
    pub source: End,
    pub target: End,
    pub from_state: Option<Name>,
    pub to_state: Option<Name>,
    pub tag: Option<String>,
    pub backward_tag: Option<String>,
    /// The number and unit of an exception sentence (O12).
    pub exception: Option<(f64, String)>,
    pub pos: Pos,
}

impl LinkSpec {
    fn new(kind: LinkKind, source: End, target: End, pos: Pos) -> Self {
        LinkSpec {
            kind,
            modifier: None,
            source,
            target,
            from_state: None,
            to_state: None,
            tag: None,
            backward_tag: None,
            exception: None,
            pos,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoomBody {
    pub nothing: bool,
    /// Steps in order; a step of more than one name is parallel.
    pub steps: Vec<Vec<Name>>,
    /// `in that sequence` was written.
    pub sequenced: bool,
    /// Some step was written with `parallel`.
    pub parallel: bool,
    /// After `as well as`.
    pub parts: Vec<Name>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Sentence {
    Decl {
        thing: Name,
        process: bool,
        qualifiers: Vec<Name>,
    },
    StateList {
        thing: Name,
        states: Vec<Name>,
    },
    StateEnds {
        thing: Name,
        initial: Vec<Name>,
        final_: Vec<Name>,
    },
    StateSentence {
        state: Name,
        thing: Name,
        flags: Vec<Name>,
    },
    Zoom {
        thing: Name,
        body: ZoomBody,
    },
    /// A sentence of one link.
    Link(Box<LinkSpec>),
    /// A list sentence (`P requires A and B.`), input only (§9.1).
    Links(Vec<LinkSpec>),
    Fan {
        operator: FanOperator,
        links: Vec<LinkSpec>,
    },
}

impl Sentence {
    fn is_procedural_link(&self) -> bool {
        match self {
            Sentence::Link(l) => l.kind.is_procedural(),
            Sentence::Links(ls) => ls.iter().all(|l| l.kind.is_procedural()),
            Sentence::Fan { .. } => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub pos: Pos,
    pub path: Option<Name>,
    pub sentence: Sentence,
    pub core: Option<Block>,
    pub juiz: Option<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub name: String,
    pub core: Option<Block>,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    pub header: Option<Header>,
    pub statements: Vec<Statement>,
}

/// Parse tokens. Syntax errors are reported and the parser goes on after the next `.`, so one
/// run reports every broken sentence.
pub fn parse(tokens: Vec<Token>) -> (Document, Vec<Diagnostic>) {
    let mut p = Parser {
        t: tokens,
        i: 0,
        diags: Vec::new(),
    };
    let mut doc = Document::default();
    if p.is_word("Model") {
        match p.header() {
            Ok(h) => doc.header = Some(h),
            Err(d) => p.recover(d),
        }
    }
    while !matches!(p.peek(), Tok::Eof) {
        match p.statement() {
            Ok(s) => doc.statements.push(s),
            Err(d) => p.recover(d),
        }
    }
    (doc, p.diags)
}

type R<T> = Result<T, Diagnostic>;

struct Parser {
    t: Vec<Token>,
    i: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    // ---- token helpers

    fn peek(&self) -> &Tok {
        self.peek_at(0)
    }

    fn peek_at(&self, n: usize) -> &Tok {
        let k = (self.i + n).min(self.t.len() - 1);
        &self.t[k].tok
    }

    fn pos(&self) -> Pos {
        self.t[self.i.min(self.t.len() - 1)].pos
    }

    fn bump(&mut self) -> Token {
        let t = self.t[self.i.min(self.t.len() - 1)].clone();
        if self.i < self.t.len() - 1 {
            self.i += 1;
        }
        t
    }

    fn word_at(&self, n: usize) -> Option<&str> {
        match self.peek_at(n) {
            Tok::Word(w) => Some(w),
            _ => None,
        }
    }

    fn is_word(&self, w: &str) -> bool {
        self.word_at(0) == Some(w)
    }

    fn is_words(&self, ws: &[&str]) -> bool {
        ws.iter()
            .enumerate()
            .all(|(k, w)| self.word_at(k) == Some(*w))
    }

    fn eat(&mut self, w: &str) -> bool {
        if self.is_word(w) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, w: &str) -> R<()> {
        if self.eat(w) {
            Ok(())
        } else {
            Err(self.unexpected(&format!("`{}`", w)))
        }
    }

    fn expect_words(&mut self, ws: &[&str]) -> R<()> {
        for w in ws {
            self.expect(w)?;
        }
        Ok(())
    }

    fn eat_comma(&mut self) -> bool {
        if matches!(self.peek(), Tok::Comma) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_comma(&mut self) -> R<()> {
        if self.eat_comma() {
            Ok(())
        } else {
            Err(self.unexpected("`,`"))
        }
    }

    fn describe(t: &Tok) -> String {
        match t {
            Tok::Word(w) => format!("`{}`", w),
            Tok::Quoted(q) => names::quote(q),
            Tok::Num(n) | Tok::Range(n) => format!("`{}`", n),
            Tok::Comma => "`,`".into(),
            Tok::Period => "`.`".into(),
            Tok::Block(b) if b.juiz => "`[...]`".into(),
            Tok::Block(_) => "`{...}`".into(),
            Tok::Eof => "the end of the text".into(),
        }
    }

    fn unexpected(&self, wanted: &str) -> Diagnostic {
        Diagnostic::error(
            Code::Syntax,
            Some(self.pos()),
            format!("expected {}, found {}", wanted, Self::describe(self.peek())),
        )
    }

    fn recover(&mut self, d: Diagnostic) {
        self.diags.push(d);
        loop {
            match self.peek() {
                Tok::Eof => return,
                Tok::Period => {
                    self.bump();
                    return;
                }
                _ => {
                    self.bump();
                }
            }
        }
    }

    fn o(&mut self, rule: u8, pos: Pos, msg: impl Into<String>) {
        self.diags
            .push(Diagnostic::error(Code::O(rule), Some(pos), msg));
    }

    // ---- names

    fn is_thing_start_at(&self, n: usize) -> bool {
        match self.peek_at(n) {
            Tok::Quoted(_) => true,
            Tok::Word(w) => w.starts_with(|c: char| c.is_ascii_uppercase()),
            _ => false,
        }
    }

    fn thing(&mut self) -> R<Name> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Quoted(q) => {
                self.bump();
                Ok(Name { text: q, pos })
            }
            Tok::Word(w) if w.starts_with(|c: char| c.is_ascii_uppercase()) => {
                self.bump();
                let mut text = w;
                loop {
                    match self.peek() {
                        Tok::Word(w)
                            if w.starts_with(|c: char| {
                                c.is_ascii_uppercase() || c.is_ascii_digit()
                            }) =>
                        {
                            text.push(' ');
                            text.push_str(w);
                        }
                        Tok::Num(n) if !n.contains('.') => {
                            text.push(' ');
                            text.push_str(n);
                        }
                        _ => break,
                    }
                    self.bump();
                }
                Ok(Name { text, pos })
            }
            _ => Err(self.unexpected("a Thing name")),
        }
    }

    fn is_bare_state_word(w: &str) -> bool {
        names::is_bare_state(w)
    }

    fn state(&mut self) -> R<Name> {
        let pos = self.pos();
        match self.peek().clone() {
            Tok::Quoted(q) => {
                self.bump();
                Ok(Name { text: q, pos })
            }
            Tok::Word(w) if Self::is_bare_state_word(&w) => {
                self.bump();
                Ok(Name { text: w, pos })
            }
            _ => Err(self.unexpected("a State name")),
        }
    }

    /// A State before a Thing: a bare State word, or a quoted name followed by a Thing name.
    fn is_state_then_thing(&self) -> bool {
        match self.peek() {
            Tok::Word(w) => Self::is_bare_state_word(w),
            Tok::Quoted(_) => self.is_thing_start_at(1),
            _ => false,
        }
    }

    fn mult(&mut self) -> R<Option<String>> {
        if !self.mult_starts(0) {
            return Ok(None);
        }
        let pos = self.pos();
        let m = match self.bump().tok {
            Tok::Range(r) => r,
            Tok::Num(n) => {
                if self.eat("to") {
                    match self.peek().clone() {
                        Tok::Num(m) if !m.contains('.') => {
                            self.bump();
                            format!("{}..{}", n, m)
                        }
                        Tok::Word(w) if w == "many" => {
                            self.bump();
                            format!("{}..*", n)
                        }
                        _ => return Err(self.unexpected("a number or `many`")),
                    }
                } else {
                    n
                }
            }
            Tok::Word(w) if w == "optional" => "*".to_string(),
            Tok::Word(w) if w == "an" => {
                self.bump();
                "0..1".to_string()
            }
            Tok::Word(w) if w == "at" => {
                self.bump();
                self.bump();
                "1..*".to_string()
            }
            _ => {
                return Err(Diagnostic::error(
                    Code::Syntax,
                    Some(pos),
                    "malformed multiplicity",
                ))
            }
        };
        Ok(Some(m))
    }

    /// `[multiplicity] [state] thing`.
    fn end(&mut self, allow_state: bool) -> R<End> {
        let mult = self.mult()?;
        let state = if allow_state && self.is_state_then_thing() {
            Some(self.state()?)
        } else {
            None
        };
        let thing = self.thing()?;
        Ok(End { mult, state, thing })
    }

    /// `x, y, and z` with the given conjunctions; one item is a list too. `starts` tells
    /// whether an item starts that many tokens ahead.
    fn list<T>(
        &mut self,
        conj: &[&str],
        starts: fn(&Self, usize) -> bool,
        mut item: impl FnMut(&mut Self) -> R<T>,
    ) -> R<Vec<T>> {
        let is_conj = |p: &Self, n: usize| p.word_at(n).is_some_and(|w| conj.contains(&w));
        let mut out = vec![item(self)?];
        loop {
            let comma = matches!(self.peek(), Tok::Comma);
            if comma && is_conj(self, 1) && starts(self, 2) {
                self.bump();
                self.bump();
                out.push(item(self)?);
                return Ok(out);
            } else if comma && starts(self, 1) {
                self.bump();
                out.push(item(self)?);
            } else if is_conj(self, 0) && starts(self, 1) {
                self.bump();
                out.push(item(self)?);
                return Ok(out);
            } else {
                return Ok(out);
            }
        }
    }

    fn mult_starts(&self, n: usize) -> bool {
        match self.peek_at(n) {
            Tok::Num(x) => !x.contains('.'),
            Tok::Range(_) => true,
            Tok::Word(w) => match w.as_str() {
                "optional" => true,
                "an" => self.word_at(n + 1) == Some("optional"),
                "at" => {
                    self.word_at(n + 1) == Some("least")
                        && self.word_at(n + 2) == Some("one")
                        && self.word_at(n + 3) != Some("of")
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn end_starts(&self, n: usize) -> bool {
        self.mult_starts(n)
            || self.is_thing_start_at(n)
            || matches!(self.peek_at(n), Tok::Word(w) if Self::is_bare_state_word(w) && self.is_thing_start_at(n + 1))
    }

    fn state_starts(&self, n: usize) -> bool {
        match self.peek_at(n) {
            Tok::Quoted(_) => true,
            Tok::Word(w) => Self::is_bare_state_word(w),
            _ => false,
        }
    }

    fn thing_starts(&self, n: usize) -> bool {
        self.is_thing_start_at(n)
    }

    fn flag_starts(&self, n: usize) -> bool {
        matches!(self.word_at(n), Some("initial" | "final" | "default"))
    }

    // ---- document

    fn header(&mut self) -> R<Header> {
        let pos = self.pos();
        self.expect("Model")?;
        let name = match self.bump().tok {
            Tok::Quoted(q) => q,
            _ => {
                return Err(Diagnostic::error(
                    Code::Syntax,
                    Some(pos),
                    "the model name must be quoted (`Model \"...\".`)",
                ))
            }
        };
        let core = match self.peek() {
            Tok::Block(b) if !b.juiz => {
                let b = b.clone();
                self.bump();
                Some(b)
            }
            _ => None,
        };
        if !matches!(self.peek(), Tok::Period) {
            return Err(self.unexpected("`.`"));
        }
        self.bump();
        Ok(Header { name, core, pos })
    }

    fn statement(&mut self) -> R<Statement> {
        let pos = self.pos();
        if self.is_word("Model") {
            return Err(Diagnostic::error(
                Code::Syntax,
                Some(pos),
                "the `Model` heading must be the first sentence",
            ));
        }
        let path = if self.is_word("Following") {
            self.bump();
            self.expect("path")?;
            let l = self.state()?;
            self.expect_comma()?;
            Some(l)
        } else {
            None
        };
        let sentence = self.sentence()?;
        if let Some(l) = &path {
            if !sentence.is_procedural_link() {
                self.o(
                    11,
                    l.pos,
                    "`Following path` belongs only to a procedural link sentence",
                );
            }
        }
        let mut core = None;
        let mut juiz = None;
        while let Tok::Block(b) = self.peek().clone() {
            let bpos = self.pos();
            let taken = if b.juiz {
                juiz.is_some()
            } else {
                core.is_some() || juiz.is_some()
            };
            if taken {
                return Err(Diagnostic::error(
                    Code::Syntax,
                    Some(bpos),
                    "write at most one `{...}` and then at most one `[...]`",
                ));
            }
            if b.juiz {
                juiz = Some(b);
            } else {
                core = Some(b);
            }
            self.bump();
        }
        if !matches!(self.peek(), Tok::Period) {
            return Err(self.unexpected("`.`"));
        }
        self.bump();
        Ok(Statement {
            pos,
            path,
            sentence,
            core,
            juiz,
        })
    }

    fn sentence(&mut self) -> R<Sentence> {
        if self.is_word("State") {
            return self.state_sentence();
        }
        if self.is_word("Either") || self.is_word("Exactly") || self.is_words(&["At", "least"]) {
            let pos = self.pos();
            let (operator, ends) = self.fan_ends(true)?;
            if !(self.eat("handles") || self.eat("handle")) {
                return Err(self.unexpected("`handles`"));
            }
            let p = self.end(false)?;
            let links = ends
                .into_iter()
                .map(|e| LinkSpec::new(LinkKind::Agent, e, p.clone(), pos))
                .collect();
            return Ok(Sentence::Fan { operator, links });
        }
        let pos = self.pos();
        let subjects = self.list(&["and"], Self::end_starts, |p| p.end(true))?;
        if subjects.len() > 1 {
            if self.eat("are") {
                if subjects.len() != 2 || subjects.iter().any(|s| s.state.is_some()) {
                    return Err(Diagnostic::error(
                        Code::Syntax,
                        Some(pos),
                        "`... are tag` takes exactly two Things",
                    ));
                }
                let tag = self.tag()?;
                let mut it = subjects.into_iter();
                let (a, b) = (it.next().unwrap(), it.next().unwrap());
                let mut l = LinkSpec::new(LinkKind::Tagged, a, b, pos);
                l.backward_tag = Some(tag.clone());
                l.tag = Some(tag);
                return Ok(Sentence::Link(Box::new(l)));
            }
            if self.eat("handles") || self.eat("handle") {
                let p = self.end(false)?;
                return Ok(Sentence::Links(
                    subjects
                        .into_iter()
                        .map(|e| LinkSpec::new(LinkKind::Agent, e, p.clone(), pos))
                        .collect(),
                ));
            }
            return Err(self.unexpected("`handles` or `are`"));
        }
        let subject = subjects.into_iter().next().unwrap();
        self.after_subject(subject, pos)
    }

    fn plain(&self, e: &End, what: &str) -> R<()> {
        if e.mult.is_some() || e.state.is_some() {
            return Err(Diagnostic::error(
                Code::Syntax,
                Some(e.thing.pos),
                format!("{} takes a Thing name only", what),
            ));
        }
        Ok(())
    }

    fn no_state(&self, e: &End) -> R<()> {
        if let Some(s) = &e.state {
            return Err(Diagnostic::error(
                Code::Syntax,
                Some(s.pos),
                "a State cannot stand before the Thing here",
            ));
        }
        Ok(())
    }

    fn after_subject(&mut self, subject: End, pos: Pos) -> R<Sentence> {
        let verb = match self.peek() {
            Tok::Word(w) => w.clone(),
            Tok::Quoted(_) => return self.tagged(subject, pos),
            _ => return Err(self.unexpected("a verb")),
        };
        match verb.as_str() {
            "is" => self.after_is(subject, pos),
            "can" => {
                self.plain(&subject, "`can be`")?;
                self.bump();
                self.expect("be")?;
                let states = self.states()?;
                Ok(Sentence::StateList {
                    thing: subject.thing,
                    states,
                })
            }
            "zooms" => {
                self.plain(&subject, "`zooms into`")?;
                self.bump();
                self.expect("into")?;
                let body = self.zoom_body()?;
                Ok(Sentence::Zoom {
                    thing: subject.thing,
                    body,
                })
            }
            "consumes" | "yields" | "requires" | "affects" | "invokes" => {
                self.no_state(&subject)?;
                self.bump();
                let kind = match verb.as_str() {
                    "consumes" => LinkKind::Consumption,
                    "yields" => LinkKind::Result,
                    "requires" => LinkKind::Instrument,
                    "affects" => LinkKind::Effect,
                    _ => LinkKind::Invocation,
                };
                self.object_list(kind, subject, pos)
            }
            "changes" => {
                self.no_state(&subject)?;
                self.bump();
                let mut l = self.change(subject, pos)?;
                l.pos = pos;
                Ok(Sentence::Link(Box::new(l)))
            }
            "handles" | "handle" => {
                self.bump();
                let p = self.end(false)?;
                if self.eat("if") {
                    self.plain(&p, "the Process")?;
                    let (ct, cs) = self.condition()?;
                    self.expect_comma()?;
                    self.skip_clause(&p.thing)?;
                    self.multiplicity_on_modifier(&[&subject, &p]);
                    let cond = End {
                        mult: None,
                        state: cs,
                        thing: ct,
                    };
                    if !cond.same(&subject) {
                        self.o(
                            5,
                            cond.thing.pos,
                            format!(
                                "the condition names {:?}, but the agent is {:?}",
                                cond.show(),
                                subject.show()
                            ),
                        );
                    }
                    let mut l = LinkSpec::new(LinkKind::Agent, subject, p, pos);
                    l.modifier = Some(Modifier::Condition);
                    return Ok(Sentence::Link(Box::new(l)));
                }
                Ok(Sentence::Link(Box::new(LinkSpec::new(
                    LinkKind::Agent,
                    subject,
                    p,
                    pos,
                ))))
            }
            "initiates" => {
                self.bump();
                self.event(subject, pos)
            }
            "occurs" => {
                self.plain(&subject, "`occurs`")?;
                self.bump();
                self.expect("if")?;
                self.occurs(subject.thing, pos)
            }
            "consists" => {
                self.no_state(&subject)?;
                self.bump();
                self.expect("of")?;
                self.structural_list(LinkKind::Aggregation, subject, pos)
            }
            "exhibits" => {
                self.no_state(&subject)?;
                self.bump();
                self.structural_list(LinkKind::Exhibition, subject, pos)
            }
            _ => self.tagged(subject, pos),
        }
    }

    fn after_is(&mut self, subject: End, pos: Pos) -> R<Sentence> {
        let a = self.word_at(1).map(str::to_string);
        let b = self.word_at(2).map(str::to_string);
        let is_article = matches!(a.as_deref(), Some("a" | "an"));
        if a.as_deref() == Some("an")
            && b.as_deref() == Some("instance")
            && self.word_at(3) == Some("of")
        {
            self.plain(&subject, "a classification")?;
            self.expect_words(&["is", "an", "instance", "of"])?;
            let ty = self.thing()?;
            return Ok(Sentence::Link(Box::new(LinkSpec::new(
                LinkKind::Classification,
                End::plain(ty),
                End::plain(subject.thing),
                pos,
            ))));
        }
        let qualifier_or_kind = |w: &str| {
            matches!(
                w,
                "physical" | "informatical" | "environmental" | "systemic" | "object" | "process"
            )
        };
        if is_article && b.as_deref().is_some_and(qualifier_or_kind) {
            self.plain(&subject, "a declaration")?;
            self.bump();
            self.bump();
            let mut qualifiers = Vec::new();
            loop {
                let qpos = self.pos();
                match self.word_at(0) {
                    Some(w @ ("physical" | "informatical" | "environmental" | "systemic")) => {
                        qualifiers.push(Name {
                            text: w.to_string(),
                            pos: qpos,
                        });
                        self.bump();
                    }
                    Some("object") => {
                        self.bump();
                        return Ok(self.decl(subject.thing, false, qualifiers));
                    }
                    Some("process") => {
                        self.bump();
                        return Ok(self.decl(subject.thing, true, qualifiers));
                    }
                    _ => return Err(self.unexpected("`object` or `process`")),
                }
            }
        }
        if matches!(a.as_deref(), Some("initially" | "finally")) {
            self.plain(&subject, "`is initially`")?;
            self.bump();
            let mut initial = Vec::new();
            let mut final_ = Vec::new();
            if self.eat("initially") {
                initial = self.states()?;
                if self.eat("and") {
                    self.expect("finally")?;
                    final_ = self.states()?;
                }
            } else {
                self.expect("finally")?;
                final_ = self.states()?;
            }
            return Ok(Sentence::StateEnds {
                thing: subject.thing,
                initial,
                final_,
            });
        }
        if is_article && self.is_thing_start_at(2) {
            self.plain(&subject, "a generalization")?;
            self.bump();
            self.bump();
            let general = self.thing()?;
            return Ok(Sentence::Link(Box::new(LinkSpec::new(
                LinkKind::Generalization,
                End::plain(general),
                End::plain(subject.thing),
                pos,
            ))));
        }
        self.tagged(subject, pos)
    }

    fn decl(&mut self, thing: Name, process: bool, qualifiers: Vec<Name>) -> Sentence {
        let mut seen: Vec<&str> = Vec::new();
        for q in &qualifiers {
            let clash = match q.text.as_str() {
                "physical" => "informatical",
                "informatical" => "physical",
                "environmental" => "systemic",
                _ => "environmental",
            };
            if seen.contains(&q.text.as_str()) {
                self.diags.push(Diagnostic::error(
                    Code::O(11),
                    Some(q.pos),
                    format!("`{}` is written twice", q.text),
                ));
            } else if seen.contains(&clash) {
                self.diags.push(Diagnostic::error(
                    Code::O(11),
                    Some(q.pos),
                    format!("`{}` and `{}` cannot both be written", clash, q.text),
                ));
            }
            seen.push(&q.text);
        }
        Sentence::Decl {
            thing,
            process,
            qualifiers,
        }
    }

    fn states(&mut self) -> R<Vec<Name>> {
        self.list(&["or"], Self::state_starts, |p| p.state())
    }

    fn state_sentence(&mut self) -> R<Sentence> {
        self.expect("State")?;
        let state = self.state()?;
        self.expect("of")?;
        let thing = self.thing()?;
        let mut flags = Vec::new();
        if self.eat("is") {
            flags = self.list(&["and"], Self::flag_starts, |p| {
                let pos = p.pos();
                match p.word_at(0) {
                    Some(w @ ("initial" | "final" | "default")) => {
                        let w = w.to_string();
                        p.bump();
                        Ok(Name { text: w, pos })
                    }
                    _ => Err(p.unexpected("`initial`, `final` or `default`")),
                }
            })?;
            for (k, f) in flags.iter().enumerate() {
                if flags[..k].iter().any(|g| g.text == f.text) {
                    self.diags.push(Diagnostic::error(
                        Code::O(3),
                        Some(f.pos),
                        format!("`{}` is written twice", f.text),
                    ));
                }
            }
        }
        Ok(Sentence::StateSentence {
            state,
            thing,
            flags,
        })
    }

    fn zoom_body(&mut self) -> R<ZoomBody> {
        let mut body = ZoomBody {
            nothing: false,
            steps: Vec::new(),
            sequenced: false,
            parallel: false,
            parts: Vec::new(),
        };
        if self.eat("nothing") {
            body.nothing = true;
        } else {
            loop {
                let step = self.step(&mut body)?;
                body.steps.push(step);
                // `, in that sequence`
                if matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("in") {
                    self.bump();
                    self.expect_words(&["in", "that", "sequence"])?;
                    body.sequenced = true;
                    break;
                }
                // `, and step` / `and step`
                let comma_and = matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("and");
                if comma_and || self.is_word("and") {
                    if comma_and {
                        self.bump();
                    }
                    self.bump();
                    let step = self.step(&mut body)?;
                    body.steps.push(step);
                    if matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("in") {
                        self.bump();
                        self.expect_words(&["in", "that", "sequence"])?;
                        body.sequenced = true;
                    } else if self.is_word("in") {
                        self.expect_words(&["in", "that", "sequence"])?;
                        body.sequenced = true;
                    }
                    break;
                }
                // `, step`
                if matches!(self.peek(), Tok::Comma)
                    && (self.is_thing_start_at(1) || self.word_at(1) == Some("parallel"))
                {
                    self.bump();
                    continue;
                }
                break;
            }
        }
        if matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("as") {
            self.bump();
            self.expect_words(&["as", "well", "as"])?;
            body.parts = self.list(&["and"], Self::thing_starts, |p| p.thing())?;
        }
        Ok(body)
    }

    /// A Thing, or `parallel` and names up to and including the one after `and`.
    fn step(&mut self, body: &mut ZoomBody) -> R<Vec<Name>> {
        if !self.eat("parallel") {
            return Ok(vec![self.thing()?]);
        }
        body.parallel = true;
        let mut out = vec![self.thing()?];
        loop {
            if self.is_word("and") {
                self.bump();
                out.push(self.thing()?);
                return Ok(out);
            }
            if matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("and") {
                self.bump();
                self.bump();
                out.push(self.thing()?);
                return Ok(out);
            }
            if matches!(self.peek(), Tok::Comma) && self.is_thing_start_at(1) {
                self.bump();
                out.push(self.thing()?);
                continue;
            }
            return Err(self.unexpected("`and` to end the parallel step"));
        }
    }

    fn is_fan_start(&self) -> bool {
        self.is_word("either")
            || self.is_words(&["exactly", "one", "of"])
            || self.is_words(&["at", "least", "one", "of"])
    }

    /// `either A or B`, `exactly one of A, B, or C`, `at least one of A or B`.
    fn fan_ends(&mut self, capital: bool) -> R<(FanOperator, Vec<End>)> {
        let w = |s: &str| -> String {
            if capital {
                let mut c = s.chars();
                c.next()
                    .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                    .unwrap_or_default()
            } else {
                s.to_string()
            }
        };
        if self.eat(&w("either")) {
            let a = self.end(true)?;
            self.expect("or")?;
            let b = self.end(true)?;
            return Ok((FanOperator::Xor, vec![a, b]));
        }
        let (op, conj): (FanOperator, &[&str]) = if self.eat(&w("exactly")) {
            self.expect_words(&["one", "of"])?;
            (FanOperator::Xor, &["or"])
        } else {
            self.expect(&w("at"))?;
            self.expect_words(&["least", "one", "of"])?;
            (FanOperator::Or, &["or", "and"])
        };
        let ends = self.list(conj, Self::end_starts, |p| p.end(true))?;
        if ends.len() < 2 {
            return Err(Diagnostic::error(
                Code::O(9),
                Some(self.pos()),
                "a Fan needs two or more ends",
            ));
        }
        Ok((op, ends))
    }

    /// The links of `P verb ...` with P the subject.
    fn spec(kind: LinkKind, subject: &End, object: End, pos: Pos) -> LinkSpec {
        if kind.is_enabling_or_consuming() {
            LinkSpec::new(kind, object, subject.clone(), pos)
        } else {
            LinkSpec::new(kind, subject.clone(), object, pos)
        }
    }

    fn object_list(&mut self, kind: LinkKind, subject: End, pos: Pos) -> R<Sentence> {
        if kind == LinkKind::Invocation && self.eat("itself") {
            let mut l = LinkSpec::new(kind, subject.clone(), subject, pos);
            l.target.mult = None;
            return Ok(Sentence::Link(Box::new(l)));
        }
        let with_state = !matches!(kind, LinkKind::Effect | LinkKind::Invocation);
        if self.is_fan_start() {
            let (operator, ends) = self.fan_ends(false)?;
            if !with_state {
                for e in &ends {
                    self.no_state(e)?;
                }
            }
            return Ok(Sentence::Fan {
                operator,
                links: ends
                    .into_iter()
                    .map(|e| Self::spec(kind, &subject, e, pos))
                    .collect(),
            });
        }
        let ends = self.list(&["and"], Self::end_starts, |p| p.end(with_state))?;
        let mut links: Vec<LinkSpec> = ends
            .into_iter()
            .map(|e| Self::spec(kind, &subject, e, pos))
            .collect();
        if links.len() == 1 {
            Ok(Sentence::Link(Box::new(links.pop().unwrap())))
        } else {
            Ok(Sentence::Links(links))
        }
    }

    fn structural_list(&mut self, kind: LinkKind, subject: End, pos: Pos) -> R<Sentence> {
        let ends = self.list(&["and"], Self::end_starts, |p| p.end(false))?;
        let mut links: Vec<LinkSpec> = ends
            .into_iter()
            .map(|e| LinkSpec::new(kind, subject.clone(), e, pos))
            .collect();
        if links.len() == 1 {
            Ok(Sentence::Link(Box::new(links.pop().unwrap())))
        } else {
            Ok(Sentence::Links(links))
        }
    }

    /// `A from s1 to s2` after `changes`; `process` is the changing Process.
    fn change(&mut self, process: End, pos: Pos) -> R<LinkSpec> {
        let a = self.thing()?;
        let mut l = LinkSpec::new(LinkKind::Effect, process, End::plain(a), pos);
        if self.eat("from") {
            l.from_state = Some(self.state()?);
            if self.eat("to") {
                l.to_state = Some(self.state()?);
            }
        } else {
            self.expect("to")?;
            l.to_state = Some(self.state()?);
        }
        Ok(l)
    }

    /// `A exists` or `A is s`.
    fn condition(&mut self) -> R<(Name, Option<Name>)> {
        let a = self.thing()?;
        if self.eat("exists") {
            return Ok((a, None));
        }
        self.expect("is")?;
        let s = self.state()?;
        Ok((a, Some(s)))
    }

    /// `otherwise P is skipped`.
    fn skip_clause(&mut self, p: &Name) -> R<()> {
        self.expect("otherwise")?;
        let q = self.thing()?;
        self.expect_words(&["is", "skipped"])?;
        if q.text != p.text {
            self.o(
                5,
                q.pos,
                format!("`otherwise {} is skipped` must name {:?}", q.text, p.text),
            );
        }
        Ok(())
    }

    fn same_thing(&mut self, a: &Name, b: &Name) {
        if a.text != b.text {
            self.o(5, b.pos, format!("{:?} must be {:?} here", b.text, a.text));
        }
    }

    fn same_end(&mut self, a: &End, b: &End) {
        if !a.same(b) {
            self.o(
                5,
                b.thing.pos,
                format!("{:?} must be {:?} here", b.show(), a.show()),
            );
        }
    }

    fn multiplicity_on_modifier(&mut self, ends: &[&End]) {
        for e in ends {
            if e.mult.is_some() {
                self.o(
                    10,
                    e.thing.pos,
                    "a sentence with event or condition takes no multiplicity",
                );
            }
        }
    }

    /// After `A initiates`.
    fn event(&mut self, a: End, pos: Pos) -> R<Sentence> {
        if self.eat("and") {
            self.expect("handles")?;
            let p = self.end(false)?;
            self.multiplicity_on_modifier(&[&a, &p]);
            let mut l = LinkSpec::new(LinkKind::Agent, a, p, pos);
            l.modifier = Some(Modifier::Event);
            return Ok(Sentence::Link(Box::new(l)));
        }
        let p = self.end(false)?;
        self.expect_comma()?;
        self.expect("which")?;
        let verb = self.word_at(0).map(str::to_string);
        let mut l = match verb.as_deref() {
            Some("consumes") | Some("requires") => {
                self.bump();
                let kind = if verb.as_deref() == Some("consumes") {
                    LinkKind::Consumption
                } else {
                    LinkKind::Instrument
                };
                let b = self.end(true)?;
                self.same_end(&a, &b);
                self.multiplicity_on_modifier(&[&a, &p, &b]);
                LinkSpec::new(kind, a, p, pos)
            }
            Some("affects") => {
                self.bump();
                self.no_state(&a)?;
                let b = self.thing()?;
                self.same_thing(&a.thing, &b);
                self.multiplicity_on_modifier(&[&a, &p]);
                LinkSpec::new(LinkKind::Effect, p, a, pos)
            }
            Some("changes") => {
                self.bump();
                self.no_state(&a)?;
                self.multiplicity_on_modifier(&[&a, &p]);
                let l = self.change(p, pos)?;
                self.same_thing(&a.thing, &l.target.thing);
                l
            }
            _ => return Err(self.unexpected("`consumes`, `requires`, `affects` or `changes`")),
        };
        l.modifier = Some(Modifier::Event);
        Ok(Sentence::Link(Box::new(l)))
    }

    /// After `P occurs if`.
    fn occurs(&mut self, p: Name, pos: Pos) -> R<Sentence> {
        if self.eat("duration") {
            self.expect("of")?;
            let p1 = self.thing()?;
            let kind = if self.eat("exceeds") {
                LinkKind::OvertimeException
            } else {
                self.expect_words(&["falls", "short", "of"])?;
                LinkKind::UndertimeException
            };
            let npos = self.pos();
            let n = match self.bump().tok {
                Tok::Num(n) => n,
                _ => {
                    return Err(Diagnostic::error(
                        Code::Syntax,
                        Some(npos),
                        "expected a number",
                    ))
                }
            };
            let unit = match self.peek().clone() {
                Tok::Word(u) if u.chars().all(|c| c.is_ascii_lowercase()) => {
                    self.bump();
                    u
                }
                _ => return Err(self.unexpected("a unit")),
            };
            let mut l = LinkSpec::new(kind, End::plain(p1), End::plain(p), pos);
            l.exception = Some((names::parse_number(&n).unwrap_or(f64::NAN), unit));
            return Ok(Sentence::Link(Box::new(l)));
        }
        let (ct, cs) = self.condition()?;
        let cond = End {
            mult: None,
            state: cs,
            thing: ct,
        };
        self.expect_comma()?;
        let pe = End::plain(p.clone());
        if self.is_word("otherwise") {
            self.skip_clause(&p)?;
            let mut l = LinkSpec::new(LinkKind::Instrument, cond, pe, pos);
            l.modifier = Some(Modifier::Condition);
            return Ok(Sentence::Link(Box::new(l)));
        }
        self.expect_words(&["in", "which", "case"])?;
        let q = self.thing()?;
        let verb = self.word_at(0).map(str::to_string);
        let mut l = match verb.as_deref() {
            Some("consumes") => {
                self.bump();
                self.same_thing(&p, &q);
                let b = self.end(true)?;
                self.same_end(&cond, &b);
                self.multiplicity_on_modifier(&[&b]);
                LinkSpec::new(LinkKind::Consumption, cond, pe, pos)
            }
            Some("is") => {
                self.bump();
                self.expect("consumed")?;
                self.same_thing(&cond.thing, &q);
                LinkSpec::new(LinkKind::Consumption, cond, pe, pos)
            }
            Some("affects") => {
                self.bump();
                self.same_thing(&p, &q);
                let b = self.thing()?;
                self.same_thing(&cond.thing, &b);
                if let Some(s) = &cond.state {
                    self.o(
                        5,
                        s.pos,
                        "`affects` has no from-state, so the condition is `exists`",
                    );
                }
                LinkSpec::new(LinkKind::Effect, pe, End::plain(cond.thing), pos)
            }
            Some("changes") => {
                self.bump();
                self.same_thing(&p, &q);
                let l = self.change(pe, pos)?;
                self.same_thing(&cond.thing, &l.target.thing);
                match (&cond.state, &l.from_state) {
                    (Some(s), Some(f)) if s.text != f.text => {
                        let (fp, ft, st) = (f.pos, f.text.clone(), s.text.clone());
                        self.o(5, fp, format!("the from-state {:?} must be {:?}", ft, st));
                    }
                    (Some(s), None) => {
                        let sp = s.pos;
                        self.o(5, sp, "without a from-state, the condition is `exists`");
                    }
                    (None, Some(f)) => {
                        let fp = f.pos;
                        self.o(
                            5,
                            fp,
                            "with a from-state, the condition is `is <from-state>`",
                        );
                    }
                    _ => {}
                }
                l
            }
            _ => return Err(self.unexpected("`consumes`, `affects`, `changes` or `is consumed`")),
        };
        self.expect_comma()?;
        self.skip_clause(&p)?;
        l.modifier = Some(Modifier::Condition);
        Ok(Sentence::Link(Box::new(l)))
    }

    fn tag(&mut self) -> R<String> {
        if let Tok::Quoted(q) = self.peek().clone() {
            self.bump();
            return Ok(q);
        }
        let mut words = Vec::new();
        while let Some(w) = self.word_at(0) {
            if !w.starts_with(|c: char| c.is_ascii_lowercase())
                || !w.chars().all(|c| c.is_ascii_lowercase() || c == '-')
            {
                break;
            }
            words.push(w.to_string());
            self.bump();
        }
        if words.is_empty() {
            return Err(self.unexpected("a tag"));
        }
        Ok(words.join(" "))
    }

    fn tagged(&mut self, mut subject: End, pos: Pos) -> R<Sentence> {
        // `"A" "tag" "B"` reads as State + Thing at first; undo that.
        if let (Some(s), Tok::Quoted(_)) = (&subject.state, self.peek()) {
            if subject.mult.is_none() {
                let tag = subject.thing.text.clone();
                subject.thing = s.clone();
                subject.state = None;
                let target = self.end(false)?;
                return self.tagged_rest(subject, tag, target, pos);
            }
        }
        self.no_state(&subject)?;
        let tag = self.tag()?;
        let target = self.end(false)?;
        self.tagged_rest(subject, tag, target, pos)
    }

    fn tagged_rest(&mut self, source: End, tag: String, target: End, pos: Pos) -> R<Sentence> {
        let mut l = LinkSpec::new(LinkKind::Tagged, source, target, pos);
        l.tag = Some(tag);
        if matches!(self.peek(), Tok::Comma) && self.word_at(1) == Some("and") {
            self.bump();
            self.bump();
            let b = self.thing()?;
            let back = self.tag()?;
            let a = self.thing()?;
            let (t, s) = (l.target.thing.clone(), l.source.thing.clone());
            self.same_thing(&t, &b);
            self.same_thing(&s, &a);
            l.backward_tag = Some(back);
        }
        Ok(Sentence::Link(Box::new(l)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lex::tokenize;

    fn one(s: &str) -> Sentence {
        let (doc, d) = parse(tokenize(s).unwrap());
        assert!(d.is_empty(), "{:?}", d);
        assert_eq!(doc.statements.len(), 1);
        doc.statements.into_iter().next().unwrap().sentence
    }

    fn link(s: &str) -> LinkSpec {
        match one(s) {
            Sentence::Link(l) => *l,
            x => panic!("{:?}", x),
        }
    }

    #[test]
    fn declarations() {
        match one("Barista is an environmental physical object.") {
            Sentence::Decl {
                thing,
                process,
                qualifiers,
            } => {
                assert_eq!(thing.text, "Barista");
                assert!(!process);
                assert_eq!(qualifiers.len(), 2);
            }
            x => panic!("{:?}", x),
        }
    }

    #[test]
    fn procedural() {
        let l = link("Brewing consumes hot Water.");
        assert_eq!(l.kind, LinkKind::Consumption);
        assert_eq!(l.source.state.unwrap().text, "hot");
        assert_eq!(l.target.thing.text, "Brewing");

        let l = link("Main Loop EC Cycle occurs if Main Loop EC is started, otherwise Main Loop EC Cycle is skipped.");
        assert_eq!(l.kind, LinkKind::Instrument);
        assert_eq!(l.modifier, Some(Modifier::Condition));
        assert_eq!(l.source.thing.text, "Main Loop EC");

        let l =
            link("\"started\" \"Main Loop EC\" initiates P, which requires started Main Loop EC.");
        assert_eq!(l.modifier, Some(Modifier::Event));
        assert_eq!(l.source.state.unwrap().text, "started");

        let l = link("Water Heating changes Water from cold to hot.");
        assert_eq!(l.from_state.unwrap().text, "cold");

        let l = link("P2 occurs if duration of P1 exceeds 60.0 min.");
        assert_eq!(l.kind, LinkKind::OvertimeException);
        assert_eq!(l.source.thing.text, "P1");
        assert_eq!(l.exception, Some((60.0, "min".into())));

        let l = link("P invokes itself.");
        assert_eq!(l.source, l.target);

        let l = link("P consumes 2 to many A.");
        assert_eq!(l.source.mult.as_deref(), Some("2..*"));
    }

    #[test]
    fn structural() {
        let l = link("Coffee Machine is located in Kitchen.");
        assert_eq!(l.tag.as_deref(), Some("is located in"));
        let l = link("A is located in B, and B contains A.");
        assert_eq!(l.backward_tag.as_deref(), Some("contains"));
        let l = link("Sarah and Isaac are family.");
        assert_eq!(l.backward_tag.as_deref(), Some("family"));
        let l = link("Dog is an Animal.");
        assert_eq!(l.kind, LinkKind::Generalization);
        assert_eq!(l.source.thing.text, "Animal");
        let l = link("Rex is an instance of Dog.");
        assert_eq!(l.kind, LinkKind::Classification);
        let l = link("\"a\" \"is in\" \"b\".");
        assert_eq!(l.tag.as_deref(), Some("is in"));
        assert_eq!(l.source.thing.text, "a");
        let l = link("Robot consists of an optional Gripper {ordered: true}.");
        assert_eq!(l.target.mult.as_deref(), Some("0..1"));
    }

    #[test]
    fn zooms() {
        match one("P zooms into SP1, parallel SP2 and SP3, SP4, and parallel SP5, SP6, and SP7, in that sequence.") {
            Sentence::Zoom { body, .. } => {
                let sizes: Vec<usize> = body.steps.iter().map(Vec::len).collect();
                assert_eq!(sizes, vec![1, 2, 1, 3]);
                assert!(body.sequenced);
            }
            x => panic!("{:?}", x),
        }
        match one("Blur Gray zooms into Cvt Color Node and Gaussian Blur Node, in that sequence, as well as Code Const and Gray.") {
            Sentence::Zoom { body, .. } => {
                assert_eq!(body.steps.len(), 2);
                assert_eq!(body.parts.len(), 2);
            }
            x => panic!("{:?}", x),
        }
        match one("Car zooms into Engine and Wheel.") {
            Sentence::Zoom { body, .. } => assert!(!body.sequenced && body.steps.len() == 2),
            x => panic!("{:?}", x),
        }
    }

    #[test]
    fn fans() {
        match one("Brewing consumes either Ground Coffee or Instant Coffee.") {
            Sentence::Fan { operator, links } => {
                assert_eq!(operator, FanOperator::Xor);
                assert_eq!(links.len(), 2);
            }
            x => panic!("{:?}", x),
        }
        match one("At least one of Alice and Bob handles Brewing.") {
            Sentence::Fan { operator, links } => {
                assert_eq!(operator, FanOperator::Or);
                assert_eq!(links[0].kind, LinkKind::Agent);
            }
            x => panic!("{:?}", x),
        }
        match one("P yields at least one A.") {
            Sentence::Link(l) => assert_eq!(l.target.mult.as_deref(), Some("1..*")),
            x => panic!("{:?}", x),
        }
    }

    #[test]
    fn errors_recover() {
        let (doc, d) = parse(tokenize("A is a object.\nB flies.\nC is a process.").unwrap());
        assert_eq!(doc.statements.len(), 2, "{:?}", d);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].pos.unwrap().line, 2);
    }

    #[test]
    fn o5_mismatch() {
        let (_, d) = parse(tokenize("A initiates P, which consumes B.").unwrap());
        assert_eq!(d[0].code, Code::O(5));
    }
}
