"""Auto-updating ecosystem tool arsenal generator & drift detector (`scripts/generate_tool_arsenal.py`)."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_ars = os.path.dirname(os.path.abspath(__file__))
    sys.path = [os.path.dirname(_here_ars), *(p for p in sys.path if os.path.abspath(p or ".") != _here_ars)]

import argparse
from typing import Final, Sequence
import re
from pathlib import Path
import json
import hashlib

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT
from scripts.pico_heartbeat_ops import emit_json_payload

DEFAULT_ARSENAL_REL: Final[Path] = (
    Path(".agents") / "skills" / "panadiga-tool-arsenal-catalog" / "references" / "ARSENAL_INDEX.md"
)
_FRONTMATTER_RE: Final[re.Pattern[str]] = re.compile(r"^---\r?\n(.*?)\r?\n---", re.DOTALL)


def _read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def _extract_frontmatter(path: Path) -> tuple[str, str]:
    text = _read_text(path)
    match = _FRONTMATTER_RE.match(text)
    if not match:
        return path.stem, ""
    fm = match.group(1)
    name_m = re.search(r"^name:\s*([^\r\n]+)", fm, re.MULTILINE)
    name = name_m.group(1).strip().strip("\"'") if name_m else path.stem
    desc_m = re.search(r"^description:\s*>-\s*\r?\n((?:\s+.*\r?\n?)*)", fm, re.MULTILINE)
    if desc_m:
        desc = " ".join(line.strip() for line in desc_m.group(1).splitlines() if line.strip())
    else:
        single_m = re.search(r'^description:\s*"?([^"\r\n]+)"?', fm, re.MULTILINE)
        desc = single_m.group(1).strip() if single_m else ""
    return name, desc


def _discover_workspace_crates(root: Path) -> list[dict[str, str]]:
    cargo_text = _read_text(root / "Cargo.toml")
    m = re.search(r"members\s*=\s*\[(.*?)\]", cargo_text, re.DOTALL)
    if not m:
        return []
    out: list[dict[str, str]] = []
    for member_rel in re.findall(r'"([^"]+)"', m.group(1)):
        member_cargo = root / member_rel / "Cargo.toml"
        crate_name = Path(member_rel).name
        if member_cargo.is_file():
            nm = re.search(r'^name\s*=\s*"([^"]+)"', _read_text(member_cargo), re.MULTILINE)
            if nm:
                crate_name = nm.group(1)
        out.append({"name": crate_name, "path": member_rel})
    return out


def _discover_cli_commands(root: Path) -> list[str]:
    text = _read_text(root / "crates" / "panadiga-cli" / "src" / "cli" / "args" / "mod.rs")
    idx = text.find("pub enum Commands")
    if idx < 0:
        return []
    section = text[idx:]
    commands: list[str] = []
    pending_name: str | None = None
    depth = 0
    started = False
    for line in section.splitlines():
        opens = line.count("{")
        closes = line.count("}")
        if not started:
            if opens:
                started = True
                depth = opens - closes
            continue
        if depth == 1:
            cmd_attr = re.search(r'#\[command\([^)]*name\s*=\s*"([^"]+)"', line)
            if cmd_attr:
                pending_name = cmd_attr.group(1)
            else:
                var_m = re.match(r"^\s*([A-Z][A-Za-z0-9]*)\s*(?:\(|,|\{|$)", line)
                if var_m:
                    kebab = re.sub(r"(?<!^)([A-Z])", r"-\1", var_m.group(1)).lower()
                    commands.append(pending_name or kebab)
                    pending_name = None
        depth += opens - closes
        if started and depth <= 0:
            break
    return commands


def _discover_mcp(root: Path) -> tuple[list[str], list[str], list[str]]:
    tools_dir = root / "crates" / "panadiga-mcp" / "src" / "tools"
    tools: list[str] = []
    for path in sorted(tools_dir.glob("defs*.rs")):
        tools.extend(re.findall(r'"name"\s*:\s*"(panadiga_[a-z0-9_]+)"', _read_text(path)))

    res_text = _read_text(root / "crates" / "panadiga-mcp" / "src" / "resources.rs")
    resources: list[str] = []
    for uri in re.findall(r'"uri"\s*:\s*"(panadiga://[^"]+)"', res_text):
        if uri not in resources:
            resources.append(uri)
    for fmt_uri in re.findall(r'"uri"\s*:\s*format!\("(panadiga://[^"]+)"', res_text):
        pretty = (
            fmt_uri.replace("panadiga://profiles/{}", "panadiga://profiles/<model>")
            .replace("{train_id}", "<train_id>")
            .replace("{module}", "<module>")
            .replace("panadiga://{}", "panadiga://docs/<path>")
        )
        if pretty not in resources:
            resources.append(pretty)

    prompts_text = _read_text(root / "crates" / "panadiga-mcp" / "src" / "prompts.rs").split("pub fn get_prompt", 1)[0]
    prompts = re.findall(r'^\s*"name"\s*:\s*"([a-z0-9_]+)"\s*,', prompts_text, re.MULTILINE)
    return tools, resources, prompts


def _discover_tauri_ipc(root: Path) -> list[str]:
    text = _read_text(root / "apps" / "desktop" / "src-tauri" / "src" / "lib.rs")
    cmds: list[str] = []
    for pat in (r"tauri::generate_handler!\[(.*?)\]", r"build_invoke_handler!\((.*?)\)"):
        m = re.search(pat, text, re.DOTALL)
        if not m:
            continue
        for token in re.findall(r"([a-z][a-z0-9_]+)\s*,", m.group(1)):
            if token not in cmds:
                cmds.append(token)
    return cmds


def _discover_pico_catalog(root: Path) -> list[dict[str, str]]:
    cat_path = root / "crates" / "panadiga-core" / "src" / "ssot" / "hardware_catalog.json"
    if not cat_path.is_file():
        return []
    raw = json.loads(_read_text(cat_path))
    items = raw.get("commands", []) if isinstance(raw, dict) else []
    out: list[dict[str, str]] = []
    for entry in items:
        if isinstance(entry, dict) and isinstance(entry.get("id"), str):
            out.append(
                {
                    "id": str(entry["id"]),
                    "category": str(entry.get("category", "")),
                    "syntax": str(entry.get("syntax", "")),
                    "dangerous": "yes" if entry.get("dangerous") else "no",
                }
            )
    return out


def collect_arsenal(root: Path = ROOT) -> JsonDict:
    """Introspect live workspace tooling across Rust, Tauri, Pico, Python, Ghidra, Skills & Agents."""
    crates = _discover_workspace_crates(root)
    cli_cmds = _discover_cli_commands(root)
    mcp_tools, mcp_resources, mcp_prompts = _discover_mcp(root)
    tauri_ipc = _discover_tauri_ipc(root)
    pico_cmds = _discover_pico_catalog(root)

    py_scripts = sorted(
        p.relative_to(root).as_posix()
        for p in (root / "scripts").rglob("*.py")
        if "tests" not in p.parts and "__pycache__" not in p.parts and p.name != "__init__.py"
    )
    ghidra_dir = root / "tools" / "ghidra"
    ghidra_scripts = (
        sorted(p.relative_to(root).as_posix() for p in ghidra_dir.glob("*.py"))
        if ghidra_dir.is_dir()
        else []
    )
    skills = [
        {"name": _extract_frontmatter(md)[0], "path": md.relative_to(root).as_posix()}
        for md in sorted((root / ".agents" / "skills").glob("*/SKILL.md"))
    ]
    agents = [
        {"name": _extract_frontmatter(md)[0], "path": md.relative_to(root).as_posix()}
        for md in sorted((root / ".agents" / "agents").glob("*.md"))
    ]

    counts: JsonDict = {
        "crates": len(crates),
        "cli_commands": len(cli_cmds),
        "mcp_tools": len(mcp_tools),
        "mcp_resource_families": len(mcp_resources),
        "mcp_prompts": len(mcp_prompts),
        "tauri_ipc_commands": len(tauri_ipc),
        "pico_catalog_commands": len(pico_cmds),
        "python_scripts": len(py_scripts),
        "ghidra_scripts": len(ghidra_scripts),
        "skills": len(skills),
        "agents": len(agents),
    }
    inventory: JsonDict = {
        "counts": counts,
        "crates": [dict(c) for c in crates],
        "cli_commands": [str(x) for x in cli_cmds],
        "mcp_tools": [str(x) for x in mcp_tools],
        "mcp_resource_families": [str(x) for x in mcp_resources],
        "mcp_prompts": [str(x) for x in mcp_prompts],
        "tauri_ipc_commands": [str(x) for x in tauri_ipc],
        "pico_catalog_commands": [dict(p) for p in pico_cmds],
        "python_scripts": [str(x) for x in py_scripts],
        "ghidra_scripts": [str(x) for x in ghidra_scripts],
        "skills": [dict(s) for s in skills],
        "agents": [dict(a) for a in agents],
    }
    digest = hashlib.sha256(
        json.dumps(inventory, sort_keys=True, ensure_ascii=False).encode("utf-8")
    ).hexdigest()
    inventory["arsenal_sha256"] = digest
    return inventory


def _jlist(val: JsonValue) -> list[JsonValue]:
    return val if isinstance(val, list) else []


def render_arsenal_markdown(inventory: JsonDict) -> str:
    """Render deterministic Markdown catalog from `collect_arsenal()` output."""
    counts = inventory["counts"]
    assert isinstance(counts, dict)
    sha = str(inventory["arsenal_sha256"])
    crates = [x for x in _jlist(inventory["crates"]) if isinstance(x, dict)]
    cli_cmds = [str(x) for x in _jlist(inventory["cli_commands"]) if isinstance(x, str)]
    mcp_tools = [str(x) for x in _jlist(inventory["mcp_tools"]) if isinstance(x, str)]
    mcp_res = [str(x) for x in _jlist(inventory["mcp_resource_families"]) if isinstance(x, str)]
    mcp_prompts = [str(x) for x in _jlist(inventory["mcp_prompts"]) if isinstance(x, str)]
    tauri_ipc = [str(x) for x in _jlist(inventory["tauri_ipc_commands"]) if isinstance(x, str)]
    pico_cmds = [x for x in _jlist(inventory["pico_catalog_commands"]) if isinstance(x, dict)]
    py_scripts = [str(x) for x in _jlist(inventory["python_scripts"]) if isinstance(x, str)]
    ghidra_scripts = [str(x) for x in _jlist(inventory["ghidra_scripts"]) if isinstance(x, str)]
    skills = [x for x in _jlist(inventory["skills"]) if isinstance(x, dict)]
    agents = [x for x in _jlist(inventory["agents"]) if isinstance(x, dict)]

    lines: list[str] = [
        "# Panadiga Auto-Generated Tool Arsenal Index",
        "",
        "> Gerado deterministicamente por `python scripts/generate_tool_arsenal.py --write`. "
        "Verifique deriva com `--check`.",
        f"> **Arsenal Fingerprint (`SHA-256`):** `{sha}`",
        "",
        "## 1. Resumo Quantitativo",
        "",
        "| Categoria | Total | Fonte de Verdade |",
        "| :--- | ---: | :--- |",
        f"| Crates Rust Workspace | {counts['crates']} | `Cargo.toml` |",
        f"| Subcomandos `panadiga` CLI | {counts['cli_commands']} | `crates/panadiga-cli/src/cli/args/mod.rs` |",
        f"| Tools MCP Rust | {counts['mcp_tools']} | `crates/panadiga-mcp/src/tools/defs*.rs` |",
        f"| Famílias de Resources MCP | {counts['mcp_resource_families']} | `crates/panadiga-mcp/src/resources.rs` |",
        f"| Prompts MCP | {counts['mcp_prompts']} | `crates/panadiga-mcp/src/prompts.rs` |",
        f"| Comandos IPC Tauri 2 | {counts['tauri_ipc_commands']} | `apps/desktop/src-tauri/src/lib.rs` |",
        f"| Comandos Sonda Pico `P7501` | {counts['pico_catalog_commands']} | `crates/panadiga-core/src/ssot/hardware_catalog.json` |",
        f"| Scripts Python Operacionais | {counts['python_scripts']} | `scripts/**/*.py` |",
        f"| Scripts Ghidra Jython | {counts['ghidra_scripts']} | `tools/ghidra/*.py` |",
        f"| Skills `.agents/skills/` | {counts['skills']} | `.agents/skills/*/SKILL.md` |",
        f"| Subagentes `.agents/agents/` | {counts['agents']} | `.agents/agents/*.md` |",
        "",
        "## 2. Crates Rust Workspace",
        "",
        "| Crate | Caminho |",
        "| :--- | :--- |",
        *[f"| `{c['name']}` | `{c['path']}` |" for c in crates],
        "",
        "## 3. CLI `panadiga` & MCP (`Tools`, `Resources`, `Prompts`)",
        "",
        "- **Subcomandos CLI:** " + ", ".join(f"`{c}`" for c in cli_cmds),
        "- **MCP Tools:** " + ", ".join(f"`{t}`" for t in mcp_tools),
        "- **MCP Resources:** " + ", ".join(f"`{r}`" for r in mcp_res),
        "- **MCP Prompts:** " + ", ".join(f"`{p}`" for p in mcp_prompts),
        "",
        "## 4. Comandos IPC Tauri 2 & Catálogo Pico `P7501`",
        "",
        "- **Tauri IPC (`generate_handler!`):** " + ", ".join(f"`{c}`" for c in tauri_ipc),
        "",
        "| Comando `P7501` | Categoria | Sintaxe | Restrito |",
        "| :--- | :--- | :--- | :--- |",
        *[
            f"| `{p['id']}` | `{p['category']}` | `{p['syntax']}` | {p['dangerous']} |"
            for p in pico_cmds
        ],
        "",
        "## 5. Scripts Python, Ghidra Jython, Skills & Subagentes",
        "",
        "- **Scripts Python:** " + ", ".join(f"`{s}`" for s in py_scripts),
        "- **Scripts Ghidra:** " + ", ".join(f"`{g}`" for g in ghidra_scripts),
        "- **Skills:** " + ", ".join(f"`{s['name']}`" for s in skills),
        "- **Subagentes:** " + ", ".join(f"`{a['name']}`" for a in agents),
        "",
    ]
    return "\n".join(lines)


def sync_or_check_arsenal(
    *,
    root: Path = ROOT,
    out_path: Path | None = None,
    write: bool = False,
) -> tuple[int, JsonDict]:
    """Generate arsenal catalog and either write it (`--write`) or check drift (`--check`)."""
    target = out_path if out_path is not None else (root / DEFAULT_ARSENAL_REL)
    inventory = collect_arsenal(root)
    rendered = render_arsenal_markdown(inventory)

    if write:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(rendered, encoding="utf-8")
        return 0, {
            "status": "UPDATED",
            "path": str(target),
            "arsenal_sha256": inventory["arsenal_sha256"],
            "counts": inventory["counts"],
        }

    existing = target.read_text(encoding="utf-8") if target.is_file() else None
    in_sync = existing == rendered
    return (0 if in_sync else 1), {
        "status": "IN_SYNC" if in_sync else "DRIFT_DETECTED",
        "path": str(target),
        "arsenal_sha256": inventory["arsenal_sha256"],
        "counts": inventory["counts"],
    }


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint supporting `--write`, `--check`, and `--json`."""
    parser = argparse.ArgumentParser(description="Generate or verify the Panadiga Tool Arsenal index.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Write deterministic ARSENAL_INDEX.md")
    mode.add_argument("--check", action="store_true", help="Fail non-zero if ARSENAL_INDEX.md drifted")
    mode.add_argument("--json", nargs="?", const="-", default=None, help="Emit full inventory JSON")
    parser.add_argument("--out", default=None, help="Optional custom output Markdown path")
    args = parser.parse_args(argv)

    out_path = Path(args.out) if args.out else None
    if args.json is not None:
        inventory = collect_arsenal(ROOT)
        emit_json_payload(inventory, args.json)
        return 0

    code, summary = sync_or_check_arsenal(root=ROOT, out_path=out_path, write=bool(args.write))
    print(f"[{summary['status']}] arsenal={summary['path']} sha256={summary['arsenal_sha256']}")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
