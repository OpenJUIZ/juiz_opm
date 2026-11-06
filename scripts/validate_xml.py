#!/usr/bin/env python3
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) ysuga
"""Validate the XML examples and tests against the XSD (spec/xml.md §10).

- examples/xml/*.xml and tests/xml/valid/*.xml must be valid against juiz.xsd.
  With the juiz namespace removed, they must also be valid against opm.xsd
  (dropping the annotations leaves a plain OPM model, metamodel §9.1).
- tests/xml/invalid/*.xml must be rejected by juiz.xsd.

Needs lxml (pip install lxml).
"""

import copy
import sys
from pathlib import Path

from lxml import etree

ROOT = Path(__file__).resolve().parent.parent
SCHEMA_DIR = ROOT / "spec" / "schema"
JUIZ_NS = "https://openjuiz.github.io/ns/juiz/v0"


def load_schema(name):
    return etree.XMLSchema(etree.parse(str(SCHEMA_DIR / name)))


def strip_juiz(doc):
    """Return a copy of doc without elements and attributes of the juiz namespace."""
    doc = copy.deepcopy(doc)
    root = doc.getroot()
    for el in list(root.iter()):
        if not isinstance(el.tag, str):
            continue
        if etree.QName(el).namespace == JUIZ_NS:
            el.getparent().remove(el)
            continue
        for key in list(el.attrib):
            if key.startswith("{%s}" % JUIZ_NS):
                del el.attrib[key]
    etree.cleanup_namespaces(root)
    return doc


def errors(schema):
    return "\n".join("    line %d: %s" % (e.line, e.message) for e in schema.error_log)


def main():
    juiz = load_schema("juiz.xsd")
    opm = load_schema("opm.xsd")
    failed = 0

    valid = sorted((ROOT / "examples" / "xml").glob("*.xml")) + sorted(
        (ROOT / "tests" / "xml" / "valid").glob("*.xml")
    )
    for path in valid:
        rel = path.relative_to(ROOT)
        doc = etree.parse(str(path))
        if not juiz.validate(doc):
            print("NG  %s (juiz.xsd)\n%s" % (rel, errors(juiz)))
            failed += 1
            continue
        plain = strip_juiz(doc)
        if not opm.validate(plain):
            print("NG  %s (opm.xsd without juiz)\n%s" % (rel, errors(opm)))
            failed += 1
            continue
        print("ok  %s" % rel)

    for path in sorted((ROOT / "tests" / "xml" / "invalid").glob("*.xml")):
        rel = path.relative_to(ROOT)
        doc = etree.parse(str(path))
        if juiz.validate(doc):
            print("NG  %s was accepted but must be rejected" % rel)
            failed += 1
        else:
            first = juiz.error_log[0]
            print("ok  %s rejected: line %d: %s" % (rel, first.line, first.message))

    if failed:
        print("%d file(s) failed" % failed)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
