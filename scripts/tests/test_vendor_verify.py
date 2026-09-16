# SPDX-License-Identifier: MIT
# Copyright (c) 2026 David Dunnock <dunnoda@gmail.com>
import vendor_verify


def test_vendor_verify_is_inert_without_a_lockfile(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(vendor_verify, "ROOT", tmp_path)
    assert vendor_verify.main([]) == 0
    assert "no lockfile" in capsys.readouterr().out
