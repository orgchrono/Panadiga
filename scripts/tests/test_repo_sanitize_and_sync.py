"""Tests for repo sanitization audit, dry-run sync, and read-only 6-wave commit planner."""

from __future__ import annotations

from pathlib import Path
from typing import Sequence

from scripts.panadiga_cli_bridge import JsonDict, as_dict, as_list, req_str
from scripts.panadiga_ops import ROOT
from scripts.repo_sanitize_and_sync import build_commit_plan, run_check, run_sync


def test_commit_plan_maps_six_waves_isolates_excludes_and_never_mutates_git(tmp_path: Path) -> None:
    sample_porcelain = [
        " M Cargo.toml",
        " M profiles/symbols/loader_ver061_toc48_main57/PROG061.json",
        " M crates/panadiga-bench/src/lib.rs",
        " M crates/panadiga-core/src/re/symbol_index/analyzer.rs",
        " M crates/panadiga-core/src/re/sata_bridge_stage2.rs",
        " M crates/panadiga-cli/src/cli/args/re.rs",
        " M crates/panadiga-mcp/src/tools/symbols.rs",
        " M crates/panadiga-rag/src/indexer/symbols.rs",
        " M apps/desktop/src/App.tsx",
        "?? scripts/repo_sanitize_and_sync.py",
        " M AGENTS.md",
        "?? .agents/skills/panadiga-repo-sanitization-and-commit-sync/SKILL.md",
    ]

    rc, plan = build_commit_plan(
        root=tmp_path,
        exclude_paths=["crates/panadiga-core/src/re/sata_bridge_stage2.rs"],
        status_reader=lambda _root: sample_porcelain,
    )
    assert rc == 0
    assert plan["mutated_git"] is False
    assert plan["excluded_concurrent"] == ["crates/panadiga-core/src/re/sata_bridge_stage2.rs"]
    assert plan["unclassified"] == []

    waves = as_list(plan["waves"])
    assert len(waves) == 6
    by_wave: dict[str, JsonDict] = {req_str(as_dict(entry), "wave"): as_dict(entry) for entry in waves}
    assert "Cargo.toml" in as_list(by_wave["wave-1"]["files"])
    assert "profiles/symbols/loader_ver061_toc48_main57/PROG061.json" in as_list(by_wave["wave-1"]["files"])
    assert "crates/panadiga-bench/src/lib.rs" in as_list(by_wave["wave-2"]["files"])
    assert "crates/panadiga-core/src/re/sata_bridge_stage2.rs" not in as_list(by_wave["wave-2"]["files"])
    assert "crates/panadiga-mcp/src/tools/symbols.rs" in as_list(by_wave["wave-3"]["files"])
    assert "apps/desktop/src/App.tsx" in as_list(by_wave["wave-4"]["files"])
    assert "scripts/repo_sanitize_and_sync.py" in as_list(by_wave["wave-5"]["files"])
    assert "AGENTS.md" in as_list(by_wave["wave-6"]["files"])


def test_check_flags_prohibited_keystore_and_oversized_module(tmp_path: Path) -> None:
    (tmp_path / "crates" / "demo" / "src").mkdir(parents=True)
    big_rs = tmp_path / "crates" / "demo" / "src" / "god.rs"
    big_rs.write_text("\n".join("let _x = 1;" for _ in range(660)), encoding="utf-8")
    (tmp_path / "release.keystore").write_bytes(b"secret")

    rc, rep = run_check(root=tmp_path, arsenal_checker=lambda _r: True)
    assert rc == 1
    assert rep["status"] == "FAIL"
    artifacts = as_list(rep["prohibited_artifacts"])
    gods = as_list(rep["god_modules"])
    assert len(artifacts) == 1
    assert req_str(as_dict(artifacts[0]), "path") == "release.keystore"
    assert len(gods) == 1
    assert req_str(as_dict(gods[0]), "severity") == "fail"


def test_sync_executes_ssot_ghidra_and_arsenal_in_order(tmp_path: Path) -> None:
    (tmp_path / "scripts").mkdir(parents=True)
    (tmp_path / "scripts" / "generate_tool_arsenal.py").write_text("# dummy\n", encoding="utf-8")
    seen: list[list[str]] = []

    def fake_runner(cmd: Sequence[str], _cwd: Path) -> tuple[int, str, str]:
        seen.append(list(cmd))
        return 0, "ok", ""

    rc, rep = run_sync(root=tmp_path, step_runner=fake_runner)
    assert rc == 0
    assert rep["status"] == "SYNCED"
    assert len(seen) == 3
    assert seen[0][-4:] == ["ssot", "export-all", "--root", str(tmp_path)]
    assert seen[1][-4:] == ["ghidra", "export-all", "--dir", str(tmp_path / "tools" / "ghidra")]
    assert seen[2][-1] == "--write"


def test_live_workspace_has_zero_prohibited_artifacts_or_fail_god_modules() -> None:
    rc, rep = run_check(root=ROOT)
    assert rc == 0, f"Live repo sanitize check failed: {rep}"
    assert as_list(rep["prohibited_artifacts"]) == []
    gods = [as_dict(item) for item in as_list(rep["god_modules"])]
    assert [g for g in gods if req_str(g, "severity") == "fail"] == []
