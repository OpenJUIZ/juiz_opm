// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Reading the interchange XML (spec/xml.md) into a [`Model`].
//!
//! The reader checks the shape of each element (known element and attribute names, required
//! attributes, the values of enumerations and numbers) and fails on the first problem. It does
//! not resolve references or check the well-formedness rules; that is [`crate::check`].
//! The order of child elements is not checked (the XSD does that).

use std::collections::HashMap;
use std::fmt;

use roxmltree::{Document, Node, NodeType, ParsingOptions};

use crate::ext::{ExtAttribute, ExtElement, ExtNode, Extensions, XmlAttribute};
use crate::model::*;

/// A document that is not XML, or not shaped like the interchange format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadError {
    pub line: u32,
    pub column: u32,
    pub message: String,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ReadError {}

/// Read a model from the text of an interchange document.
pub fn from_str(text: &str) -> Result<Model, ReadError> {
    let opt = ParsingOptions {
        allow_dtd: false,
        ..ParsingOptions::default()
    };
    let doc = Document::parse_with_options(text, opt).map_err(|e| {
        let pos = e.pos();
        ReadError {
            line: pos.row,
            column: pos.col,
            message: e.to_string(),
        }
    })?;
    Reader { doc: &doc }.model(doc.root_element())
}

/// Read a model from a file.
pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Model, ReadError> {
    let text = std::fs::read_to_string(path.as_ref()).map_err(|e| ReadError {
        line: 0,
        column: 0,
        message: format!("{}: {}", path.as_ref().display(), e),
    })?;
    from_str(&text)
}

struct Reader<'a, 'input> {
    doc: &'a Document<'input>,
}

/// What every element with an id has (spec/xml.md §5).
struct Common<'a, 'input> {
    id: String,
    attrs: HashMap<&'a str, &'a str>,
    description: Option<String>,
    extensions: Extensions,
    children: Vec<Node<'a, 'input>>,
}

impl<'a, 'input> Common<'a, 'input> {
    fn get(&self, name: &str) -> Option<String> {
        self.attrs.get(name).map(|v| v.to_string())
    }
}

impl<'a, 'input: 'a> Reader<'a, 'input> {
    fn err(&self, node: Node, message: impl Into<String>) -> ReadError {
        let pos = self.doc.text_pos_at(node.range().start);
        ReadError {
            line: pos.row,
            column: pos.col,
            message: message.into(),
        }
    }

    fn is_opm(node: Node, name: &str) -> bool {
        node.tag_name().namespace() == Some(OPM_NS) && node.tag_name().name() == name
    }

    fn label(node: Node) -> String {
        match node.tag_name().namespace() {
            Some(OPM_NS) => format!("opm:{}", node.tag_name().name()),
            Some(ns) => format!("{{{}}}{}", ns, node.tag_name().name()),
            None => node.tag_name().name().to_string(),
        }
    }

    /// Attributes, description and extensions of an element with an id.
    fn common(
        &self,
        node: Node<'a, 'input>,
        allowed: &[&str],
    ) -> Result<Common<'a, 'input>, ReadError> {
        let mut attrs = HashMap::new();
        let mut extensions = Extensions::default();
        for a in node.attributes() {
            match a.namespace() {
                None => {
                    if a.name() != "id" && !allowed.contains(&a.name()) {
                        return Err(self.err(
                            node,
                            format!("{} has no attribute {}", Self::label(node), a.name()),
                        ));
                    }
                    attrs.insert(a.name(), a.value());
                }
                Some(OPM_NS) => {
                    return Err(self.err(
                        node,
                        format!(
                            "attribute opm:{} must not be qualified on {}",
                            a.name(),
                            Self::label(node)
                        ),
                    ))
                }
                Some(ns) => extensions.attributes.push(ExtAttribute {
                    namespace: ns.to_string(),
                    name: a.name().to_string(),
                    value: a.value().to_string(),
                }),
            }
        }
        let id = attrs
            .remove("id")
            .ok_or_else(|| self.err(node, format!("{} needs an id", Self::label(node))))?
            .to_string();

        let mut description = None;
        let mut children = Vec::new();
        for c in node.children() {
            match c.node_type() {
                NodeType::Element => match c.tag_name().namespace() {
                    Some(OPM_NS) if c.tag_name().name() == "description" => {
                        if description.is_some() {
                            return Err(self.err(c, "more than one opm:description"));
                        }
                        description = Some(self.text_only(c)?);
                    }
                    Some(OPM_NS) => children.push(c),
                    Some(_) => extensions.elements.push(self.ext_element(c)),
                    None => {
                        return Err(self.err(
                            c,
                            format!("element {} has no namespace", c.tag_name().name()),
                        ))
                    }
                },
                NodeType::Text => self.no_text(c)?,
                _ => {}
            }
        }
        Ok(Common {
            id,
            attrs,
            description,
            extensions,
            children,
        })
    }

    /// Attributes of an element without an id (member, part, duration, layout, ...).
    /// They take no foreign attributes and no child elements.
    fn plain(
        &self,
        node: Node<'a, 'input>,
        allowed: &[&str],
    ) -> Result<HashMap<&'a str, &'a str>, ReadError> {
        let mut attrs = HashMap::new();
        for a in node.attributes() {
            if a.namespace().is_some() || !allowed.contains(&a.name()) {
                return Err(self.err(
                    node,
                    format!("{} has no attribute {}", Self::label(node), a.name()),
                ));
            }
            attrs.insert(a.name(), a.value());
        }
        for c in node.children() {
            match c.node_type() {
                NodeType::Element => {
                    return Err(self.err(
                        c,
                        format!("{} is not allowed in {}", Self::label(c), Self::label(node)),
                    ))
                }
                NodeType::Text => self.no_text(c)?,
                _ => {}
            }
        }
        Ok(attrs)
    }

    fn no_text(&self, node: Node) -> Result<(), ReadError> {
        if node.text().is_some_and(|t| !t.trim().is_empty()) {
            return Err(self.err(node, "text is not allowed here"));
        }
        Ok(())
    }

    fn text_only(&self, node: Node) -> Result<String, ReadError> {
        let mut text = String::new();
        for c in node.children() {
            match c.node_type() {
                NodeType::Text => text.push_str(c.text().unwrap_or_default()),
                NodeType::Element => {
                    return Err(self.err(c, format!("{} takes text only", Self::label(node))))
                }
                _ => {}
            }
        }
        if node.attributes().len() > 0 {
            return Err(self.err(node, format!("{} takes no attributes", Self::label(node))));
        }
        Ok(text)
    }

    fn ext_element(&self, node: Node) -> ExtElement {
        let attributes = node
            .attributes()
            .map(|a| XmlAttribute {
                namespace: a.namespace().map(str::to_string),
                name: a.name().to_string(),
                value: a.value().to_string(),
            })
            .collect();
        let has_elements = node.children().any(|c| c.is_element());
        let mut children = Vec::new();
        for c in node.children() {
            match c.node_type() {
                NodeType::Element => children.push(ExtNode::Element(self.ext_element(c))),
                NodeType::Text => {
                    let t = c.text().unwrap_or_default();
                    // Indentation between child elements is layout, not content.
                    if !(has_elements && t.trim().is_empty()) {
                        children.push(ExtNode::Text(t.to_string()));
                    }
                }
                _ => {}
            }
        }
        ExtElement {
            // Elements without a namespace are rejected before this point at the top level;
            // nested ones keep an empty namespace.
            namespace: node.tag_name().namespace().unwrap_or_default().to_string(),
            name: node.tag_name().name().to_string(),
            attributes,
            children,
        }
    }

    fn required(&self, node: Node, c: &Common, name: &str) -> Result<String, ReadError> {
        c.get(name).ok_or_else(|| {
            self.err(
                node,
                format!("{} needs the attribute {}", Self::label(node), name),
            )
        })
    }

    fn boolean(&self, node: Node, value: Option<&str>) -> Result<bool, ReadError> {
        match value.map(str::trim) {
            None => Ok(false),
            Some("true") | Some("1") => Ok(true),
            Some("false") | Some("0") => Ok(false),
            Some(v) => Err(self.err(node, format!("{:?} is not a boolean", v))),
        }
    }

    fn number(&self, node: Node, value: Option<&str>) -> Result<Option<f64>, ReadError> {
        value
            .map(|v| {
                v.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| self.err(node, format!("{:?} is not a number", v)))
            })
            .transpose()
    }

    fn model(&self, node: Node<'a, 'input>) -> Result<Model, ReadError> {
        if !Self::is_opm(node, "model") {
            return Err(self.err(
                node,
                format!("the root element is {}, not opm:model", Self::label(node)),
            ));
        }
        let c = self.common(node, &["name", "format_version"])?;
        let format_version = self.required(node, &c, "format_version")?;
        if format_version.trim() != FORMAT_VERSION {
            return Err(self.err(
                node,
                format!(
                    "format_version {:?} is not supported (only {})",
                    format_version, FORMAT_VERSION
                ),
            ));
        }
        let mut model = Model::new(c.id.clone());
        model.name = c.get("name");
        model.format_version = FORMAT_VERSION.to_string();
        model.prefixes = node
            .namespaces()
            .filter(|ns| ns.uri() != OPM_NS)
            .filter_map(|ns| ns.name().map(|p| (p.to_string(), ns.uri().to_string())))
            .filter(|(p, _)| p != "xml")
            .collect();

        let mut seen_things = false;
        for child in &c.children {
            let name = child.tag_name().name();
            match name {
                "things" => {
                    seen_things = true;
                    for t in self.elements(*child)? {
                        model.things.push(self.thing(t)?);
                    }
                }
                "links" => {
                    for l in self.elements(*child)? {
                        model.links.push(self.link(l)?);
                    }
                }
                "fans" => {
                    for f in self.elements(*child)? {
                        model.fans.push(self.fan(f)?);
                    }
                }
                "in_zooms" => {
                    for z in self.elements(*child)? {
                        model.in_zooms.push(self.in_zoom(z)?);
                    }
                }
                "diagram" => {
                    if model.diagram.is_some() {
                        return Err(self.err(*child, "more than one root opm:diagram"));
                    }
                    model.diagram = Some(self.diagram(*child, true)?);
                }
                _ => {
                    return Err(
                        self.err(*child, format!("opm:{} is not allowed in opm:model", name))
                    )
                }
            }
        }
        if !seen_things {
            return Err(self.err(node, "opm:model needs opm:things"));
        }
        model.description = c.description;
        model.extensions = c.extensions;
        Ok(model)
    }

    /// The element children of a list element (things, links, fans, in_zooms).
    /// List elements have no id and take no attributes.
    fn elements(&self, node: Node<'a, 'input>) -> Result<Vec<Node<'a, 'input>>, ReadError> {
        if let Some(a) = node.attributes().next() {
            return Err(self.err(
                node,
                format!("{} has no attribute {}", Self::label(node), a.name()),
            ));
        }
        let mut out = Vec::new();
        for c in node.children() {
            match c.node_type() {
                NodeType::Element => {
                    if c.tag_name().namespace() != Some(OPM_NS) {
                        return Err(self.err(
                            c,
                            format!("{} is not allowed in {}", Self::label(c), Self::label(node)),
                        ));
                    }
                    out.push(c);
                }
                NodeType::Text => self.no_text(c)?,
                _ => {}
            }
        }
        Ok(out)
    }

    fn essence(&self, node: Node, v: Option<&str>) -> Result<Essence, ReadError> {
        match v.map(str::trim) {
            None | Some("informatical") => Ok(Essence::Informatical),
            Some("physical") => Ok(Essence::Physical),
            Some(v) => Err(self.err(node, format!("essence {:?} is not known", v))),
        }
    }

    fn affiliation(&self, node: Node, v: Option<&str>) -> Result<Affiliation, ReadError> {
        match v.map(str::trim) {
            None | Some("systemic") => Ok(Affiliation::Systemic),
            Some("environmental") => Ok(Affiliation::Environmental),
            Some(v) => Err(self.err(node, format!("affiliation {:?} is not known", v))),
        }
    }

    fn thing(&self, node: Node<'a, 'input>) -> Result<Thing, ReadError> {
        let is_object = match node.tag_name().name() {
            "object" => true,
            "process" => false,
            other => {
                return Err(self.err(node, format!("opm:{} is not allowed in opm:things", other)))
            }
        };
        let c = self.common(node, &["name", "essence", "affiliation"])?;
        let name = self.required(node, &c, "name")?;
        let kind = if is_object {
            let mut states = Vec::new();
            for s in &c.children {
                if !Self::is_opm(*s, "state") {
                    return Err(self.err(
                        *s,
                        format!("{} is not allowed in opm:object", Self::label(*s)),
                    ));
                }
                states.push(self.state(*s)?);
            }
            ThingKind::Object { states }
        } else {
            let mut duration = None;
            for d in &c.children {
                if !Self::is_opm(*d, "duration") || duration.is_some() {
                    return Err(self.err(
                        *d,
                        format!("{} is not allowed in opm:process", Self::label(*d)),
                    ));
                }
                let a = self.plain(*d, &["min", "expected", "max", "unit"])?;
                duration = Some(Duration {
                    min: self.number(*d, a.get("min").copied())?,
                    expected: self.number(*d, a.get("expected").copied())?,
                    max: self.number(*d, a.get("max").copied())?,
                    unit: a
                        .get("unit")
                        .ok_or_else(|| self.err(*d, "opm:duration needs the attribute unit"))?
                        .to_string(),
                });
            }
            ThingKind::Process { duration }
        };
        Ok(Thing {
            essence: self.essence(node, c.attrs.get("essence").copied())?,
            affiliation: self.affiliation(node, c.attrs.get("affiliation").copied())?,
            id: c.id,
            name,
            description: c.description,
            extensions: c.extensions,
            kind,
        })
    }

    fn state(&self, node: Node<'a, 'input>) -> Result<State, ReadError> {
        let c = self.common(node, &["name", "initial", "final", "default"])?;
        if let Some(x) = c.children.first() {
            return Err(self.err(
                *x,
                format!("{} is not allowed in opm:state", Self::label(*x)),
            ));
        }
        Ok(State {
            name: self.required(node, &c, "name")?,
            initial: self.boolean(node, c.attrs.get("initial").copied())?,
            final_: self.boolean(node, c.attrs.get("final").copied())?,
            default: self.boolean(node, c.attrs.get("default").copied())?,
            id: c.id,
            description: c.description,
            extensions: c.extensions,
        })
    }

    fn link(&self, node: Node<'a, 'input>) -> Result<Link, ReadError> {
        let kind = LinkKind::from_name(node.tag_name().name()).ok_or_else(|| {
            self.err(
                node,
                format!("{} is not allowed in opm:links", Self::label(node)),
            )
        })?;
        let mut allowed = vec![
            "source",
            "target",
            "path",
            "source_multiplicity",
            "target_multiplicity",
        ];
        if kind.takes_modifier() {
            allowed.push("modifier");
        }
        match kind {
            LinkKind::Effect => allowed.extend(["from_state", "to_state"]),
            LinkKind::Aggregation => allowed.push("ordered"),
            LinkKind::Tagged => allowed.extend(["tag", "backward_tag"]),
            _ => {}
        }
        let c = self.common(node, &allowed)?;
        if let Some(x) = c.children.first() {
            return Err(self.err(
                *x,
                format!(
                    "{} is not allowed in {}",
                    Self::label(*x),
                    Self::label(node)
                ),
            ));
        }
        let modifier = match c.attrs.get("modifier").map(|v| v.trim()) {
            None => None,
            Some("event") => Some(Modifier::Event),
            Some("condition") => Some(Modifier::Condition),
            Some(v) => return Err(self.err(node, format!("modifier {:?} is not known", v))),
        };
        if kind == LinkKind::Tagged && !c.attrs.contains_key("tag") {
            return Err(self.err(node, "opm:tagged needs the attribute tag"));
        }
        Ok(Link {
            kind,
            source: self.required(node, &c, "source")?,
            target: self.required(node, &c, "target")?,
            modifier,
            from_state: c.get("from_state"),
            to_state: c.get("to_state"),
            ordered: self.boolean(node, c.attrs.get("ordered").copied())?,
            tag: c.get("tag"),
            backward_tag: c.get("backward_tag"),
            path: c.get("path"),
            source_multiplicity: c.get("source_multiplicity"),
            target_multiplicity: c.get("target_multiplicity"),
            id: c.id,
            description: c.description,
            extensions: c.extensions,
        })
    }

    fn reference(&self, node: Node<'a, 'input>, name: &str) -> Result<String, ReadError> {
        if !Self::is_opm(node, name) {
            return Err(self.err(
                node,
                format!(
                    "{} is not allowed here (expected opm:{})",
                    Self::label(node),
                    name
                ),
            ));
        }
        let a = self.plain(node, &["ref"])?;
        a.get("ref")
            .map(|v| v.to_string())
            .ok_or_else(|| self.err(node, format!("opm:{} needs the attribute ref", name)))
    }

    fn fan(&self, node: Node<'a, 'input>) -> Result<Fan, ReadError> {
        if !Self::is_opm(node, "fan") {
            return Err(self.err(
                node,
                format!("{} is not allowed in opm:fans", Self::label(node)),
            ));
        }
        let c = self.common(node, &["operator"])?;
        let operator = match self.required(node, &c, "operator")?.trim() {
            "xor" => FanOperator::Xor,
            "or" => FanOperator::Or,
            v => return Err(self.err(node, format!("operator {:?} is not known", v))),
        };
        let links = c
            .children
            .iter()
            .map(|l| self.reference(*l, "link_ref"))
            .collect::<Result<_, _>>()?;
        Ok(Fan {
            id: c.id,
            operator,
            links,
            description: c.description,
            extensions: c.extensions,
        })
    }

    fn in_zoom(&self, node: Node<'a, 'input>) -> Result<InZoom, ReadError> {
        if !Self::is_opm(node, "in_zoom") {
            return Err(self.err(
                node,
                format!("{} is not allowed in opm:in_zooms", Self::label(node)),
            ));
        }
        let c = self.common(node, &["refinee"])?;
        let refinee = self.required(node, &c, "refinee")?;
        let mut steps = Vec::new();
        let mut parts = Vec::new();
        for x in &c.children {
            match x.tag_name().name() {
                "step" => {
                    self.plain_attrs_only(*x)?;
                    let mut members = Vec::new();
                    for m in x.children() {
                        match m.node_type() {
                            NodeType::Element => members.push(self.reference(m, "member")?),
                            NodeType::Text => self.no_text(m)?,
                            _ => {}
                        }
                    }
                    steps.push(members);
                }
                "part" => parts.push(self.reference(*x, "part")?),
                other => {
                    return Err(self.err(*x, format!("opm:{} is not allowed in opm:in_zoom", other)))
                }
            }
        }
        Ok(InZoom {
            id: c.id,
            refinee,
            steps,
            parts,
            description: c.description,
            extensions: c.extensions,
        })
    }

    fn plain_attrs_only(&self, node: Node) -> Result<(), ReadError> {
        if let Some(a) = node.attributes().next() {
            return Err(self.err(
                node,
                format!("{} has no attribute {}", Self::label(node), a.name()),
            ));
        }
        Ok(())
    }

    fn diagram(&self, node: Node<'a, 'input>, root: bool) -> Result<Diagram, ReadError> {
        let allowed: &[&str] = if root {
            &["name"]
        } else {
            &["name", "refinement", "refinee"]
        };
        let c = self.common(node, allowed)?;
        let refinement = if root {
            Refinement::Root
        } else {
            match self.required(node, &c, "refinement")?.trim() {
                "in_zoom" => Refinement::InZoom,
                "unfold" => Refinement::Unfold,
                "view" => Refinement::View,
                v => return Err(self.err(node, format!("refinement {:?} is not known", v))),
            }
        };
        let mut d = Diagram::new(c.id.clone(), self.required(node, &c, "name")?, refinement);
        d.refinee = c.get("refinee");
        for x in &c.children {
            match x.tag_name().name() {
                "appearance" => d.appearances.push(self.appearance(*x)?),
                "diagram" => d.children.push(self.diagram(*x, false)?),
                other => {
                    return Err(self.err(*x, format!("opm:{} is not allowed in opm:diagram", other)))
                }
            }
        }
        d.description = c.description;
        d.extensions = c.extensions;
        Ok(d)
    }

    fn appearance(&self, node: Node<'a, 'input>) -> Result<Appearance, ReadError> {
        let c = self.common(node, &["thing"])?;
        let mut a = Appearance::new(c.id.clone(), self.required(node, &c, "thing")?);
        for x in &c.children {
            match x.tag_name().name() {
                "suppressed_state" => a
                    .suppressed_states
                    .push(self.reference(*x, "suppressed_state")?),
                "layout" if a.layout.is_none() => {
                    let l = self.plain(*x, &["x", "y", "width", "height"])?;
                    let need = |n: &str| {
                        self.number(*x, l.get(n).copied())?.ok_or_else(|| {
                            self.err(*x, format!("opm:layout needs the attribute {}", n))
                        })
                    };
                    a.layout = Some(Layout {
                        x: need("x")?,
                        y: need("y")?,
                        width: self.number(*x, l.get("width").copied())?,
                        height: self.number(*x, l.get("height").copied())?,
                    });
                }
                other => {
                    return Err(self.err(
                        *x,
                        format!("opm:{} is not allowed in opm:appearance", other),
                    ))
                }
            }
        }
        a.description = c.description;
        a.extensions = c.extensions;
        Ok(a)
    }
}
