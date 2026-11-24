"""Live read-only governance, frontmatter, link integrity, and safety-rule consistency tests."""

from __future__ import annotations

import json
from pathlib import Path
import re
from typing import Final

from scripts.panadiga_ops import ROOT

_MD_LINK_RE: Final[re.Pattern[str]] = re.compile(r"\[[^\]]+\]\(([^)]+)\)")
_REQUIRED_RULE_TOKENS: Final[tuple[str, ...]] = (
    "3.3V",
    "NSW5V",
    "XRESET",
    "SA0..SA10",
    "CONFIRM_UPPER_NOR",
    "0xC000100C",
    "0x807F0000",
)


def _extract_frontmatter(md_path: Path) -> str:
    lines = md_path.read_text(encoding="utf-8").splitlines()
    assert lines and lines[0].strip() == "---", f"{md_path}: missing opening YAML frontmatter delimiter"
    end_idx = next((idx for idx in range(1, len(lines)) if lines[idx].strip() == "---"), None)
    assert end_idx is not None, f"{md_path}: missing closing YAML frontmatter delimiter"
    return "\n".join(lines[1:end_idx])


def test_all_29_skills_have_valid_frontmatter_and_zero_broken_links() -> None:
    skill_files = sorted((ROOT / ".agents" / "skills").glob("*/SKILL.md"))
    assert len(skill_files) == 29
    broken_links: list[str] = []

    for skill_md in skill_files:
        fm = _extract_frontmatter(skill_md)
        name_match = re.search(r"^name:\s*(.+)$", fm, re.MULTILINE)
        desc_match = re.search(r"^description:\s*(.+)$", fm, re.MULTILINE)
        assert name_match is not None, f"{skill_md}: missing name field"
        assert desc_match is not None and desc_match.group(1).strip(), f"{skill_md}: empty description"
        declared_name = name_match.group(1).strip().strip('"').strip("'")
        assert declared_name == skill_md.parent.name, f"{skill_md}: name {declared_name!r} != folder"

        content = skill_md.read_text(encoding="utf-8")
        for match in _MD_LINK_RE.finditer(content):
            href = match.group(1).split("#", 1)[0].strip()
            if not href or href.startswith(("http://", "https://", "mailto:", "panadiga://")):
                continue
            resolved = (skill_md.parent / href).resolve()
            if not resolved.exists():
                broken_links.append(f"{skill_md.relative_to(ROOT).as_posix()} -> {href}")

    assert broken_links == [], f"Broken local skill links found: {broken_links}"


def test_all_6_agents_valid_and_qa_auditor_is_read_only() -> None:
    agent_files = sorted((ROOT / ".agents" / "agents").glob("*.md"))
    assert len(agent_files) == 6
    for agent_md in agent_files:
        fm = _extract_frontmatter(agent_md)
        assert re.search(r"^name:\s*\S+", fm, re.MULTILINE), f"{agent_md}: missing agent name"

    qa_fm = _extract_frontmatter(ROOT / ".agents" / "agents" / "panadiga-qa-auditor.md")
    assert "write_to_file" not in qa_fm
    assert "replace_file_content" not in qa_fm
    assert re.search(r"^commandExecutionPolicy:\s*safe\s*$", qa_fm, re.MULTILINE)


def test_rule_files_enforce_consistent_physical_safety_invariants() -> None:
    for rel in ("AGENTS.md", "GEMINI.md", ".cursorrules"):
        text = (ROOT / rel).read_text(encoding="utf-8")
        missing = [tok for tok in _REQUIRED_RULE_TOKENS if tok not in text]
        assert missing == [], f"{rel} missing required physical safety invariants: {missing}"


def test_mcp_config_defines_native_and_ghidra_servers() -> None:
    raw = json.loads((ROOT / ".mcp.json").read_text(encoding="utf-8"))
    servers = raw["mcpServers"]
    assert set(servers.keys()) == {"panadiga-tools", "ghidra"}
    for name, cfg in servers.items():
        args = cfg["args"]
        assert "scripts.panadiga_cli_bridge" in args, f"{name}: must route via scripts.panadiga_cli_bridge"
