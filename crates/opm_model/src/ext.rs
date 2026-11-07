// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Attributes and elements of other namespaces (spec/xml.md §8).
//!
//! Every element with an id may carry them. opm_model keeps them as they are and writes them
//! back, so the juiz annotations survive a round trip through tools that only know the core.

/// The foreign attributes and elements of one element, in document order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Extensions {
    pub attributes: Vec<ExtAttribute>,
    pub elements: Vec<ExtElement>,
}

impl Extensions {
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty() && self.elements.is_empty()
    }

    /// The value of the attribute `{namespace}name`.
    pub fn attribute(&self, namespace: &str, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace == namespace && a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Set (or replace) the attribute `{namespace}name`.
    pub fn set_attribute(
        &mut self,
        namespace: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<String>,
    ) {
        let (namespace, name, value) = (namespace.into(), name.into(), value.into());
        match self
            .attributes
            .iter_mut()
            .find(|a| a.namespace == namespace && a.name == name)
        {
            Some(a) => a.value = value,
            None => self.attributes.push(ExtAttribute {
                namespace,
                name,
                value,
            }),
        }
    }

    /// Remove the attribute `{namespace}name` and return its value.
    pub fn remove_attribute(&mut self, namespace: &str, name: &str) -> Option<String> {
        let i = self
            .attributes
            .iter()
            .position(|a| a.namespace == namespace && a.name == name)?;
        Some(self.attributes.remove(i).value)
    }

    /// The first child element `{namespace}name`.
    pub fn element(&self, namespace: &str, name: &str) -> Option<&ExtElement> {
        self.elements
            .iter()
            .find(|e| e.namespace == namespace && e.name == name)
    }

    /// Every namespace used here, nested elements and their attributes included.
    pub(crate) fn namespaces<'a>(&'a self, out: &mut Vec<&'a str>) {
        for a in &self.attributes {
            out.push(&a.namespace);
        }
        for e in &self.elements {
            e.namespaces(out);
        }
    }
}

/// An attribute of another namespace on an OPM element. It is always qualified.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtAttribute {
    pub namespace: String,
    pub name: String,
    pub value: String,
}

/// An element of another namespace, kept as a tree.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtElement {
    pub namespace: String,
    pub name: String,
    pub attributes: Vec<XmlAttribute>,
    pub children: Vec<ExtNode>,
}

impl ExtElement {
    pub fn new(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        ExtElement {
            namespace: namespace.into(),
            name: name.into(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    /// An element holding only text, such as `<juiz:value>"COLOR_BGR2GRAY"</juiz:value>`.
    pub fn with_text(
        namespace: impl Into<String>,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut e = Self::new(namespace, name);
        e.children.push(ExtNode::Text(text.into()));
        e
    }

    /// The concatenated text of the direct children.
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|c| match c {
                ExtNode::Text(t) => Some(t.as_str()),
                ExtNode::Element(_) => None,
            })
            .collect()
    }

    /// The value of an attribute; `namespace` is `None` for an unqualified one.
    pub fn attribute(&self, namespace: Option<&str>, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace.as_deref() == namespace && a.name == name)
            .map(|a| a.value.as_str())
    }

    fn namespaces<'a>(&'a self, out: &mut Vec<&'a str>) {
        out.push(&self.namespace);
        for a in &self.attributes {
            if let Some(ns) = &a.namespace {
                out.push(ns);
            }
        }
        for c in &self.children {
            if let ExtNode::Element(e) = c {
                e.namespaces(out);
            }
        }
    }
}

/// An attribute inside a foreign element. It may be unqualified (`<juiz:translation x="1"/>`).
#[derive(Debug, Clone, PartialEq)]
pub struct XmlAttribute {
    pub namespace: Option<String>,
    pub name: String,
    pub value: String,
}

/// A child of a foreign element.
#[derive(Debug, Clone, PartialEq)]
pub enum ExtNode {
    Element(ExtElement),
    Text(String),
}
