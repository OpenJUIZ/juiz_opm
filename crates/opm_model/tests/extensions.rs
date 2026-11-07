// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) ysuga

//! Foreign namespaces are kept as they are (spec/xml.md §8).

use opm_model::{check, read, write, ExtElement, ExtNode, Model, Thing, JUIZ_NS};

const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0"
           xmlns:x="urn:example:other" format_version="0.1" id="m" name="A &amp; B">
  <opm:description>line 1
line 2 &lt;tag&gt;</opm:description>
  <x:meta x:k="1" plain="v">
    <x:inner>text &amp; more</x:inner>
    <x:empty/>
  </x:meta>
  <opm:things>
    <opm:object id="a" name="A" x:note="say &quot;hi&quot;">
      <x:mixed>before <x:b>bold</x:b> after</x:mixed>
      <opm:state id="s" name="s"/>
    </opm:object>
  </opm:things>
</opm:model>
"#;

#[test]
fn foreign_namespaces_round_trip() {
    let m1 = read::from_str(DOC).unwrap();
    assert_eq!(m1.name.as_deref(), Some("A & B"));
    assert_eq!(m1.description.as_deref(), Some("line 1\nline 2 <tag>"));
    let meta = &m1.extensions.elements[0];
    assert_eq!(meta.namespace, "urn:example:other");
    assert_eq!(meta.attribute(None, "plain"), Some("v"));
    assert_eq!(meta.attribute(Some("urn:example:other"), "k"), Some("1"));
    assert_eq!(meta.children.len(), 2, "indentation is not content");
    let a = m1.thing("a").unwrap();
    assert_eq!(
        a.extensions.attribute("urn:example:other", "note"),
        Some("say \"hi\"")
    );
    assert_eq!(
        a.extensions.elements[0].children.len(),
        3,
        "mixed content is kept"
    );

    let s1 = write::to_string(&m1);
    let m2 = read::from_str(&s1).unwrap();
    assert_eq!(m1, m2, "{}", s1);
    assert!(s1.contains("xmlns:x=\"urn:example:other\""), "{}", s1);
    assert!(
        s1.contains("<x:mixed>before <x:b>bold</x:b> after</x:mixed>"),
        "{}",
        s1
    );
}

#[test]
fn undeclared_namespaces_get_a_prefix() {
    let mut m = Model::new("m");
    let mut t = Thing::object("a", "A");
    t.extensions.set_attribute(JUIZ_NS, "role", "container");
    t.extensions.set_attribute("urn:example:tool", "x", "1");
    let mut e = ExtElement::new("urn:example:tool", "data");
    e.children.push(ExtNode::Text("42".into()));
    t.extensions.elements.push(e);
    m.things.push(t);
    assert!(check(&m).is_ok());

    let s = write::to_string(&m);
    assert!(
        s.contains("xmlns:juiz=\"https://openjuiz.github.io/ns/juiz/v0\""),
        "{}",
        s
    );
    assert!(s.contains("xmlns:ns1=\"urn:example:tool\""), "{}", s);
    assert!(s.contains("juiz:role=\"container\" ns1:x=\"1\""), "{}", s);
    let mut back = read::from_str(&s).unwrap();
    back.prefixes.clear();
    assert_eq!(back, m);
}

#[test]
fn reader_rejects_unknown_core_parts() {
    let base = |inner: &str| {
        format!(
            r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0" format_version="0.1" id="m"><opm:things>{}</opm:things></opm:model>"#,
            inner
        )
    };
    for bad in [
        r#"<opm:thing id="a" name="A"/>"#,
        r#"<opm:object id="a" name="A" colour="red"/>"#,
        r#"<opm:object id="a" name="A" essence="spiritual"/>"#,
        r#"<opm:object id="a"/>"#,
        r#"<opm:object name="A"/>"#,
        r#"<opm:object id="a" name="A">text</opm:object>"#,
        r#"<opm:object id="a" name="A"><plain/></opm:object>"#,
        r#"<opm:process id="p" name="P"><opm:state id="s" name="s"/></opm:process>"#,
    ] {
        assert!(read::from_str(&base(bad)).is_err(), "{}", bad);
    }
    assert!(read::from_str(
        r#"<opm:model xmlns:opm="https://openjuiz.github.io/ns/opm/v0" format_version="0.2" id="m"><opm:things/></opm:model>"#
    )
    .is_err());
    assert!(read::from_str(
        r#"<model xmlns="https://openjuiz.github.io/ns/opm/v0" format_version="0.1" id="m"><things/></model>"#
    )
    .is_ok(), "a default namespace is fine");
}
