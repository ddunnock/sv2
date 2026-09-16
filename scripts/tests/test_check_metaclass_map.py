# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The scope boundary between the pinned OMG metamodel and the other metamodels the
grammars import. Aliases and URIs are those the vendored grammars actually declare:

    import "http://www.eclipse.org/emf/2002/Ecore" as Ecore
    import "https://www.omg.org/spec/SysML/20250201" as SysML
"""

import pytest

from check_metaclass_map import omg_aliases, partition

GRAMMARS = [
    {
        "file": "KerML.xtext",
        "imports": [
            {"uri": "http://www.eclipse.org/emf/2002/Ecore", "alias": "Ecore"},
            {"uri": "https://www.omg.org/spec/SysML/20250201", "alias": "SysML"},
        ],
    }
]


def test_only_the_omg_import_is_in_scope():
    assert omg_aliases(GRAMMARS) == {"SysML"}


def test_an_import_without_an_alias_is_not_an_alias():
    grammars = [{"file": "g", "imports": [{"uri": "https://www.omg.org/spec/SysML/20250201"}]}]
    assert omg_aliases(grammars) == set()


@pytest.mark.parametrize(
    ("qualified", "in_scope"),
    [
        # SysML is the pinned metamodel: its metaclasses must be in the XMI.
        ("SysML::PartDefinition", True),
        # Ecore is EMF infrastructure, deliberately not vendored. EString is absent
        # from SysML.xmi by construction, not by drift.
        ("Ecore::EString", False),
        ("Ecore::EBoolean", False),
        # An unqualified name names no foreign metamodel, so the pinned XMI is the
        # only place it could come from. Defaulting it out of scope would let a
        # genuinely missing metaclass through unchecked.
        ("PartUsage", True),
        # An alias the grammars never import is not quietly trusted as OMG.
        ("Unknown::Thing", False),
    ],
)
def test_partition_splits_on_the_declared_omg_alias(qualified, in_scope):
    got_in, got_out = partition({qualified: ["SomeRule"]}, omg_aliases(GRAMMARS))
    assert bool(got_in) is in_scope
    assert bool(got_out) is not in_scope


def test_a_missing_omg_metaclass_is_still_in_scope():
    # The regression this check exists for: the Xtext naming a metaclass the pinned
    # metamodel does not define. Narrowing the scope to OMG aliases must not let it
    # through — it is the failure the whole check is for.
    in_scope, out_of_scope = partition(
        {"SysML::DriftedAwayMetaclass": ["SomeRule"]}, omg_aliases(GRAMMARS)
    )
    assert in_scope == [("SysML::DriftedAwayMetaclass", ["SomeRule"])]
    assert out_of_scope == []


def test_every_entry_lands_in_exactly_one_bucket():
    metaclass_map = {
        "SysML::PartDefinition": ["PartDefinition"],
        "SysML::PartUsage": ["PartUsage"],
        "Ecore::EString": ["STRING_VALUE", "UNRESTRICTED_NAME"],
        "Bare": ["R"],
    }
    in_scope, out_of_scope = partition(metaclass_map, omg_aliases(GRAMMARS))
    assert len(in_scope) + len(out_of_scope) == len(metaclass_map)
    assert dict(in_scope).keys() | dict(out_of_scope).keys() == metaclass_map.keys()
