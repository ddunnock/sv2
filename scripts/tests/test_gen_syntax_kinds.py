# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
"""The generated tables are Rust source: a word that closes its own string literal is a defect."""

from gen_syntax_kinds import _reserved_entries


def test_a_plain_word_is_a_rust_string_literal():
    assert _reserved_entries(["about", "abstract"]) == ['    "about",', '    "abstract",']


def test_a_quote_in_a_word_is_escaped():
    assert _reserved_entries(['say"it']) == ['    "say\\"it",']


def test_a_backslash_in_a_word_is_escaped():
    assert _reserved_entries(["back\\slash"]) == ['    "back\\\\slash",']


def test_a_backslash_is_escaped_before_a_quote_is():
    # The other order turns \ into \\ after the quote's escape and re-opens the literal.
    assert _reserved_entries(['\\"']) == ['    "\\\\\\"",']
