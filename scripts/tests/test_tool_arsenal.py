"""Tests for dynamic Panadiga Tool Arsenal introspection, determinism, and drift checks."""

from __future__ import annotations

from pathlib import Path

from scripts.generate_tool_arsenal import (
    collect_arsenal,
    render_arsenal_markdown,
    sync_or_check_arsenal,
)
from scripts.panadiga_cli_bridge import as_dict, as_list, req_int
from scripts.panadiga_ops import ROOT


def test_collect_arsenal_matches_live_workspace_entities() -> None:
    inv = collect_arsenal(ROOT)
    counts = as_dict(inv["counts"])
    assert req_int(counts, "crates") == 11
    assert req_int(counts, "mcp_tools") == 26
    assert req_int(counts, "mcp_resource_families") >= 9
    assert req_int(counts, "mcp_prompts") == 4
    assert req_int(counts, "skills") == 29
    assert req_int(counts, "agents") == 6

    mcp_tools = [str(x) for x in as_list(inv["mcp_tools"])]
    assert "panadiga_symbol_lookup" in mcp_tools
    pico_codecs = [str(x) for x in as_list(inv["pico_c11_codecs"])]
    assert "firmware/picofw/diag/picofw_boottrace_codec.c" in pico_codecs
    assert "firmware/picofw/diag/picofw_tbus_codec.c" in pico_codecs
    digaos_mods = [str(x) for x in as_list(inv["digaos_modules"])]
    assert "firmware/digaos/drivers/avdd_iic_video.c" in digaos_mods
    shell_cmds = set(str(x) for x in as_list(inv["digaos_shell_commands"]))
    assert {
        "status",
        "video br",
        "video ec",
        "video probe",
        "dac on",
        "dac off",
        "colorbar on",
        "colorbar off",
        "provision",
    }.issubset(shell_cmds)
    skill_names = {str(as_dict(s)["name"]) for s in as_list(inv["skills"])}
    assert {
        "panadiga-contracts-stream-and-journal",
        "panadiga-bench-automation-and-headless",
        "panadiga-repo-sanitization-and-commit-sync",
        "panadiga-device-onboarding",
        "panadiga-opensource-firmware-dev",
        "panadiga-functional-math-and-patterns",
        "panadiga-anti-trap-geometry-verification",
        "panadiga-firmware-forensic-atlas",
    }.issubset(skill_names)


def test_render_deterministic_and_check_detects_drift(tmp_path: Path) -> None:
    first = render_arsenal_markdown(collect_arsenal(ROOT))
    second = render_arsenal_markdown(collect_arsenal(ROOT))
    assert first == second

    target = tmp_path / "ARSENAL_INDEX.md"
    rc_w, rep_w = sync_or_check_arsenal(root=ROOT, out_path=target, write=True)
    assert rc_w == 0
    assert rep_w["status"] == "UPDATED"

    rc_ok, rep_ok = sync_or_check_arsenal(root=ROOT, out_path=target, write=False)
    assert rc_ok == 0
    assert rep_ok["status"] == "IN_SYNC"

    target.write_text("# stale drifted index\n", encoding="utf-8")
    rc_drift, rep_drift = sync_or_check_arsenal(root=ROOT, out_path=target, write=False)
    assert rc_drift == 1
    assert rep_drift["status"] == "DRIFT_DETECTED"


def test_live_workspace_arsenal_is_in_sync() -> None:
    rc, rep = sync_or_check_arsenal(root=ROOT, write=False)
    assert rc == 0, f"ARSENAL_INDEX drift detected: {rep}"
    assert rep["status"] == "IN_SYNC"
