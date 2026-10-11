# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import pytest

from check_ast_leaks import findings


def _messages(text):
    return [f.message for f in findings(text, "src/a.rs")]


def test_an_accessor_wrapping_one_syntax_node_is_clean():
    text = "// node: RootNamespace\npub struct RootNamespace {\n    node: SyntaxNode,\n}\n"
    assert _messages(text) == []


def test_a_tuple_struct_accessor_is_clean():
    # The idiomatic rowan shape. A checker that only understood braced structs would pass
    # every accessor in the crate without reading one of them.
    assert _messages("// node: RootNamespace\npub struct RootNamespace(SyntaxNode);\n") == []


def test_a_tuple_struct_owning_text_is_a_leak():
    text = "// node: Part\npub struct Part(SyntaxNode, String);\n"
    assert any("owns text" in m for m in _messages(text))


def test_a_tuple_struct_with_two_fields_is_a_leak():
    text = "// node: Part\npub struct Part(SyntaxNode, usize);\n"
    assert any("one SyntaxNode" in m for m in _messages(text))


def test_a_tuple_struct_without_its_marker_is_rejected():
    assert any("no `// node:` marker" in m for m in _messages("pub struct Part(SyntaxNode);\n"))


@pytest.mark.parametrize(
    "field",
    ["name: String", "names: Vec<String>", "text: Box<str>", "owned: std::string::String"],
)
def test_an_owned_text_field_is_a_leak(field):
    text = f"pub struct Part {{\n    node: SyntaxNode,\n    {field},\n}}\n"
    assert any("owns text" in m for m in _messages(text))


def test_a_second_field_is_a_leak_even_without_text():
    text = "pub struct Part {\n    node: SyntaxNode,\n    index: usize,\n}\n"
    assert any("one SyntaxNode" in m for m in _messages(text))


def test_a_public_fn_returning_string_is_a_leak():
    assert any("returns String" in m for m in _messages("pub fn name(&self) -> String {\n"))


def test_a_private_fn_returning_string_is_allowed():
    assert _messages("fn name(&self) -> String {\n") == []


@pytest.mark.parametrize("name", ["test_tree", "dump_node", "debug_print", "fixture_part"])
def test_scaffolding_in_the_public_api_is_rejected(name):
    assert any("scaffolding" in m for m in _messages(f"pub fn {name}() {{\n"))


def test_a_public_item_gated_on_cfg_test_is_rejected():
    text = "#[cfg(test)]\npub fn helper_for_tests() {}\n"
    assert any("cfg(test)" in m for m in _messages(text))


def test_a_private_item_gated_on_cfg_test_is_allowed():
    assert _messages("#[cfg(test)]\nmod tests {}\n") == []


def test_an_accessor_type_without_its_marker_is_rejected():
    text = "pub struct PartUsage {\n    node: SyntaxNode,\n}\n"
    assert any("no `// node:` marker" in m for m in _messages(text))


def test_a_finding_carries_its_line():
    text = "pub struct Part {\n    node: SyntaxNode,\n    name: String,\n}\n"
    assert [f.line for f in findings(text, "src/a.rs") if "owns text" in f.message] == [3]
