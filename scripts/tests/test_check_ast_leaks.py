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


@pytest.mark.parametrize(
    "text",
    [
        "pub struct Leaky<'a> { node: SyntaxNode, cached: String, extra: &'a str }\n",
        "pub struct Leaky2<T> where T: Clone { node: T, cached: String }\n",
        "pub struct Leaky3 { text: Arc<str> }\n",
    ],
)
def test_a_shape_the_checker_cannot_read_is_reported_not_skipped(text):
    """A struct the patterns miss is exempt from EVERY rule, which looks checked.

    Generics and `where` clauses used to defeat the header pattern, so such a struct could
    own a String, hold three fields and carry no marker in silence.
    """
    assert findings(text, "src/a.rs")


def test_a_nested_paren_tuple_struct_is_still_read():
    """`[^)]*` ended the field list at the inner `)`, so the struct vanished."""
    messages = _messages("pub struct Leaky(pub Vec<(String, u32)>);\n")
    assert any("owns text" in m for m in messages)


def test_a_one_field_tuple_struct_is_not_reported_as_holding_two():
    """A top-level split would read `Vec<(String, u32)>` as two fields."""
    messages = _messages("pub struct Leaky(pub Vec<(String, u32)>);\n")
    assert not any("fields" in m for m in messages)


def test_arc_str_owns_text():
    text = "// node: Part\npub struct Part {\n    text: Arc<str>,\n}\n"
    assert any("owns text" in m for m in _messages(text))


def test_a_doc_comment_describing_the_rule_is_not_a_leak():
    """The rules describe themselves in prose; documentation is not a violation."""
    text = (
        "/// Never write `pub fn name(&self) -> String` here.\n"
        "// node: Part\n"
        "pub struct Part(SyntaxNode);\n"
    )
    assert not _messages(text)


def test_a_public_type_that_is_not_a_node_view_may_say_so():
    """Without this a named iterator deadlocks: the marker rule wants `// node: Members`,
    and that marker makes `Members` an absent kind in ast_coverage.py."""
    text = (
        "// not-a-node: a borrowing iterator, not a view of a kind\n"
        "pub struct Members(SyntaxNodeChildren);\n"
    )
    assert not _messages(text)


def test_a_public_type_with_neither_marker_nor_reason_is_reported():
    assert any("not-a-node" in m for m in _messages("pub struct Mystery(SyntaxNode);\n"))


def test_the_marker_need_not_repeat_the_type_name():
    """The accessor type and its node kind may legitimately differ."""
    text = "// node: PackageDeclaration\npub struct PackageDecl(SyntaxNode);\n"
    assert not _messages(text)


def test_cfg_any_test_is_still_a_test_gate():
    assert any(
        "cfg(test)" in m for m in _messages('#[cfg(any(test, feature = "x"))]\npub fn f() {}\n')
    )
