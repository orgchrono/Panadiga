"""Auto-updating Forensic Atlas for 100%-disassembled Ghidra MN103S firmware & SSOT contracts."""

from __future__ import annotations

import os
import sys

if __package__ in (None, ""):
    _here_atlas = os.path.dirname(os.path.abspath(__file__))
    sys.path = [os.path.dirname(_here_atlas), *(p for p in sys.path if os.path.abspath(p or ".") != _here_atlas)]

import hashlib
import re
from pathlib import Path
from dataclasses import asdict, dataclass
import json
import argparse
from typing import Final, Mapping, Sequence, cast

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT
from scripts.pico_heartbeat_ops import emit_json_payload

ATLAS_JSON_REL: Final[Path] = Path("profiles") / "forensic_atlas" / "firmware_forensic_atlas.json"
ATLAS_MD_REL: Final[Path] = (
    Path(".agents") / "skills" / "panadiga-firmware-forensic-atlas" / "references" / "FORENSIC_ATLAS_INDEX.md"
)
_DOC_FUNC_HDR_RE: Final[re.Pattern[str]] = re.compile(
    r"^=== FUNC (\S+) \(([^)]+)\) @ (0x[0-9A-Fa-f]+) \(size=(\d+) B"
)
_BARE_FUNC_RE: Final[re.Pattern[str]] = re.compile(
    r"^(?:undefined\d*|void|int|uint|ushort|short|byte|char)\s+(FUN_([0-9A-Fa-f]{8}))\("
)
_CALLER_RE: Final[re.Pattern[str]] = re.compile(r"^// Callers \(\d+\):\s*(.*)$")
_HEX_CONST_RE: Final[re.Pattern[str]] = re.compile(r"\b0x[0-9A-Fa-f]{4,8}\b")
_CALLEE_RE: Final[re.Pattern[str]] = re.compile(r"\b(?:FUN_[0-9A-Fa-f]{8}|func_0x[0-9A-Fa-f]+)\b")
_BITWISE_RE: Final[re.Pattern[str]] = re.compile(
    r"(?:[&|^]\s*0x[0-9A-Fa-f]+|(?:<<|>>)\s*\d+|~\s*\([^)]+\)|~\s*[A-Za-z0-9_]+)"
)


@dataclass(frozen=True)
class ParsedDocFunction:
    """One Ghidra-disassembled function extracted from `docs/pseudocode/raw/doc_*.txt`."""

    program: str
    source_file: str
    va: str
    name: str
    alias: str
    size_bytes: int
    callers: list[str]
    callees: list[str]
    hex_constants: list[str]
    bitwise_ops: list[str]


def classify_hex_constant(hex_str: str) -> str:
    """Map an MN103S / AM34 address or hex constant into its physical bus region."""
    val = int(hex_str, 16)
    if 0x0000_0000 <= val <= 0x0003_FFFF:
        return "SRAM_OR_OFFSET"
    if 0x4000_0000 <= val <= 0x43FF_FFFF or 0x5000_0000 <= val <= 0x5FFF_FFFF or 0x9000_0000 <= val <= 0x97FF_FFFF:
        return "SDRAM"
    if 0x8000_0000 <= val <= 0x80FF_FFFF:
        return "FLASH_CS0"
    if 0x8400_0000 <= val <= 0x84FF_FFFF:
        return "FLASH_CS1_UNCACHED"
    if 0x8900_0400 <= val <= 0x8900_1000:
        return "MMIO_DAVIS"
    if 0xD400_2000 <= val <= 0xD400_20FF:
        return "MMIO_SC0"
    if 0xC000_0000 <= val <= 0xFFFF_FFFF:
        return "MMIO_SYS"
    return "CONSTANT"


def _parse_doc_capture(path: Path) -> tuple[str, list[ParsedDocFunction]]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    program = path.name
    for ln in lines[:10]:
        if ln.startswith("PROGRAM:"):
            program = ln.split(":", 1)[1].strip()
            break

    blocks: list[tuple[str, str, str, int, list[str]]] = []
    cur_hdr: tuple[str, str, str, int] | None = None
    cur_body: list[str] = []

    def _flush() -> None:
        if cur_hdr is not None:
            blocks.append((*cur_hdr, cur_body[:]))

    for ln in lines:
        hdr_m = _DOC_FUNC_HDR_RE.match(ln)
        if hdr_m:
            _flush()
            cur_hdr = (
                hdr_m.group(1),
                hdr_m.group(2),
                "0x" + hdr_m.group(3)[2:].upper(),
                int(hdr_m.group(4)),
            )
            cur_body = []
            continue
        bare_m = _BARE_FUNC_RE.match(ln)
        if bare_m and (cur_hdr is None or cur_hdr[0] != bare_m.group(1)):
            if path.name == "doc_boot_fat_sd_java.txt":
                _flush()
                cur_hdr = (bare_m.group(1), bare_m.group(1), "0x" + bare_m.group(2).upper(), 0)
                cur_body = []
                continue
        if cur_hdr is not None:
            cur_body.append(ln)
    _flush()

    parsed: list[ParsedDocFunction] = []
    for name, alias, va, size_b, body in blocks:
        callers: list[str] = []
        for bline in body[:12]:
            cm = _CALLER_RE.match(bline)
            if cm:
                callers = [c.strip() for c in cm.group(1).split(",") if c.strip()]
                break
        body_text = "\n".join(body)
        hex_consts = sorted({"0x" + h[2:].upper() for h in _HEX_CONST_RE.findall(body_text)})
        callees = sorted({c for c in _CALLEE_RE.findall(body_text) if c != name})
        bitwise = sorted({re.sub(r"\s+", " ", bw.strip()) for bw in _BITWISE_RE.findall(body_text)})
        parsed.append(
            ParsedDocFunction(
                program=program,
                source_file=path.name,
                va=va,
                name=name,
                alias=alias,
                size_bytes=size_b,
                callers=callers,
                callees=callees[:24],
                hex_constants=hex_consts[:32],
                bitwise_ops=bitwise[:24],
            )
        )
    return program, parsed


def _collect_headers_and_footers(root: Path) -> list[dict[str, str]]:
    records: list[dict[str, str]] = [
        {"artifact": "EXTRHEAD", "kind": "container_header", "signature": "4558545248454144 ('EXTRHEAD')", "notes": "Encrypted/compressed stage header paired with EXTRFOOT footer"},
        {"artifact": "EXTRFOOT", "kind": "container_footer", "signature": "45585452464F4F54 ('EXTRFOOT')", "notes": "Trailing EXTRHEAD container footer"},
        {"artifact": "MainListHeader", "kind": "toc_header", "signature": "0x2800 (v2/HDD) / 0x1400 (CD/SD)", "notes": "Firmware TOC module table header preceding fmaX partitions"},
    ]
    for path in sorted((root / "profiles" / "media_formats").glob("*.json")):
        raw = json.loads(path.read_text(encoding="utf-8"))
        records.append(
            {
                "artifact": str(raw.get("id", path.stem)),
                "kind": str(raw.get("media_kind", "")),
                "signature": f"magic_hex=0x{raw.get('magic_hex', '')} @ start_lba={raw.get('start_lba', 0)}",
                "notes": f"fs={raw.get('filesystem', '')}, sector={raw.get('sector_size', 512)} B",
            }
        )
    for path in sorted((root / "profiles" / "firmware_trains").glob("*.json")):
        raw = json.loads(path.read_text(encoding="utf-8"))
        frm = raw.get("frm")
        fp = raw.get("loader_fingerprint")
        if isinstance(frm, dict) and frm.get("toc_entry_bytes"):
            records.append(
                {
                    "artifact": f"{raw['id']}:FRM_TOC",
                    "kind": str(frm.get("cipher", "frm")),
                    "signature": f"entry={frm.get('toc_entry_bytes')}B toc_size={frm.get('toc_size')}",
                    "notes": f"key_id={frm.get('key_id', 'none')}",
                }
            )
        if isinstance(fp, dict) and fp.get("ascii_marker"):
            records.append(
                {
                    "artifact": f"{raw['id']}:IPL_BANNER",
                    "kind": "bootloader_header",
                    "signature": f"ascii={fp['ascii_marker']!r} [{fp['offset_start']}..{fp['offset_end']}]",
                    "notes": f"loader_profile_key={raw.get('loader_profile_key', '')}",
                }
            )
    for path in sorted((root / "profiles" / "nor_layouts").glob("*.json")):
        raw = json.loads(path.read_text(encoding="utf-8"))
        for reg in raw.get("regions", []):
            if reg.get("kind") in {"bgo", "eemul"}:
                sig = reg.get("signature")
                sig_note = f" marker[+{sig['marker_byte_offset']}]={sig['marker_value']}" if isinstance(sig, dict) else ""
                records.append(
                    {
                        "artifact": f"{raw['id']}:{reg['kind']}",
                        "kind": f"nor_{reg['kind']}",
                        "signature": f"[{reg['start_offset']}..={reg['end_offset']}]{sig_note}",
                        "notes": str(reg.get("tag", "")),
                    }
                )
    return records


def collect_forensic_atlas(root: Path = ROOT) -> JsonDict:
    """Build deterministic Ghidra zero-undef + SSOT forensic dictionary across workspace artifacts."""
    fleet_path = root / "docs" / "audits" / "zero_undef" / "fleet_verify.json"
    fleet = json.loads(fleet_path.read_text(encoding="utf-8")) if fleet_path.is_file() else {}
    fleet_progs: list[JsonValue] = [
        cast(
            JsonValue,
            {
                "program": str(p.get("program", "")),
                "model_id": str(p.get("model_id", "")),
                "stage_kind": str(p.get("stage_kind", "")),
                "sha256": str(p.get("sha256", "")),
            },
        )
        for p in fleet.get("programs", [])
        if isinstance(p, dict)
    ]

    functions: list[ParsedDocFunction] = []
    hex_index: dict[str, set[str]] = {}
    bitwise_index: dict[str, list[str]] = {}
    raw_dir = root / "docs" / "pseudocode" / "raw"
    for doc_path in sorted(raw_dir.glob("doc_*.txt")):
        _, doc_funcs = _parse_doc_capture(doc_path)
        for fn in doc_funcs:
            functions.append(fn)
            for hx in fn.hex_constants:
                hex_index.setdefault(hx, set()).add(fn.va)
            for bw in fn.bitwise_ops:
                vas = bitwise_index.setdefault(bw, [])
                if fn.va not in vas and len(vas) < 12:
                    vas.append(fn.va)

    symbol_indexes: list[JsonValue] = []
    sym_dir = root / "profiles" / "symbols"
    if sym_dir.is_dir():
        for sp in sorted(sym_dir.rglob("*.json")):
            sdata = json.loads(sp.read_text(encoding="utf-8"))
            funcs_list = sdata.get("functions", [])
            destructive: list[JsonValue] = [
                cast(
                    JsonValue,
                    {
                        "va": str(f.get("va", "")),
                        "name": str(f.get("name", "")),
                        "side_effects": str(f.get("side_effects", "")),
                        "callers": [str(c) for c in f.get("callers", [])],
                    },
                )
                for f in funcs_list
                if isinstance(f, dict)
                and str(f.get("side_effects", "read_only")) != "read_only"
                and f.get("name") != f"FUN_{str(f.get('va', ''))[2:].lower()}"
            ]
            symbol_indexes.append(
                cast(
                    JsonValue,
                    {
                        "train_id": str(sdata.get("firmware_train_id", "")),
                        "module": str(sdata.get("module", "")),
                        "base_va": str(sdata.get("base_va", "")),
                        "total_functions": len(funcs_list),
                        "curated_destructive": destructive[:30],
                    },
                )
            )

    hex_catalog: list[JsonValue] = [
        cast(
            JsonValue,
            {
                "hex": hx,
                "category": classify_hex_constant(hx),
                "function_count": len(vas),
                "sample_vas": [str(v) for v in sorted(vas)[:8]],
            },
        )
        for hx, vas in sorted(hex_index.items(), key=lambda kv: (-len(kv[1]), kv[0]))[:160]
    ]
    bitwise_catalog: list[JsonValue] = [
        cast(
            JsonValue,
            {"pattern": pat, "occurrence_count": len(vas), "sample_vas": [str(v) for v in vas[:8]]},
        )
        for pat, vas in sorted(bitwise_index.items(), key=lambda kv: (-len(kv[1]), kv[0]))[:120]
    ]
    headers_footers = _collect_headers_and_footers(root)
    traps_file = root / "profiles" / "traps" / "geometry_and_binary_traps.json"
    traps = json.loads(traps_file.read_text(encoding="utf-8")).get("traps", []) if traps_file.is_file() else []

    atlas: JsonDict = {
        "schema_version": 1,
        "summary": {
            "mn103s_zero_undef_programs": int(fleet.get("mn103s_programs_zero_undef_count", 0)),
            "fleet_analyzed_programs": int(fleet.get("analyzed_programs_total", 0)),
            "fleet_total_instructions": int(fleet.get("total_instructions", 0)),
            "fleet_total_functions": int(fleet.get("total_functions", 0)),
            "doc_functions_indexed": len(functions),
            "symbol_index_files": len(symbol_indexes),
            "hex_constants_indexed": len(hex_catalog),
            "bitwise_patterns_indexed": len(bitwise_catalog),
            "headers_and_footers_indexed": len(headers_footers),
            "traps_indexed": len(traps),
        },
        "fleet_programs": fleet_progs,
        "headers_and_footers": [cast(JsonValue, dict(h)) for h in headers_footers],
        "hex_constants": hex_catalog,
        "bitwise_patterns": bitwise_catalog,
        "symbol_indexes": symbol_indexes,
        "functions": [cast(JsonValue, asdict(f)) for f in functions],
    }
    sha = hashlib.sha256(json.dumps(atlas, sort_keys=True, ensure_ascii=False).encode("utf-8")).hexdigest()
    atlas["atlas_sha256"] = sha
    return atlas


def _dict_items(val: JsonValue) -> list[Mapping[str, object]]:
    return [x for x in val if isinstance(x, dict)] if isinstance(val, list) else []


def _str_list(val: object) -> list[str]:
    return [str(v) for v in val] if isinstance(val, list) else []


def render_atlas_markdown(atlas: JsonDict) -> str:
    """Render concise deterministic Markdown reference companion to the JSON forensic atlas."""
    s = atlas["summary"]
    assert isinstance(s, dict)
    sha = str(atlas["atlas_sha256"])
    hdrs = _dict_items(atlas["headers_and_footers"])
    hexs = _dict_items(atlas["hex_constants"])
    bws = _dict_items(atlas["bitwise_patterns"])
    funcs = _dict_items(atlas["functions"])

    lines: list[str] = [
        "# Panadiga 100%-Disassembled Ghidra Forensic Atlas Index",
        "",
        "> Gerado deterministicamente por `python scripts/generate_firmware_atlas.py --write`. "
        "Verifique deriva com `--check`.",
        f"> **Atlas Fingerprint (`SHA-256`):** `{sha}`",
        "> **JSON Completo:** `profiles/forensic_atlas/firmware_forensic_atlas.json`",
        "",
        "## 1. Cobertura Zero-Undef & Totais Indexados",
        "",
        "| Métrica | Valor |",
        "| :--- | ---: |",
        f"| Programas MN103S Zero-Undef | {s['mn103s_zero_undef_programs']} / {s['fleet_analyzed_programs']} |",
        f"| Instruções & Funções Frota Ghidra | {s['fleet_total_instructions']:,} insns · {s['fleet_total_functions']:,} funcs |",
        f"| Funções Curadas (`doc_*.txt`) | {s['doc_functions_indexed']} |",
        f"| Índices `profiles/symbols/` | {s['symbol_index_files']} |",
        f"| Constantes Hex / MMIO Catalogadas | {s['hex_constants_indexed']} |",
        f"| Padrões Bitwise Catalogados | {s['bitwise_patterns_indexed']} |",
        f"| Headers, Footers & Magics SSOT | {s['headers_and_footers_indexed']} |",
        f"| Armadilhas Catalogadas (`profiles/traps/`) | {s['traps_indexed']} |",
        "",
        "## 2. Headers, Footers, Magics & Fronteiras Físicas",
        "",
        "| Artefato | Tipo | Assinatura / Janela | Notas |",
        "| :--- | :--- | :--- | :--- |",
        *[f"| `{h['artifact']}` | `{h['kind']}` | `{h['signature']}` | {h['notes']} |" for h in hdrs],
        "",
        "## 3. Top Constantes Hexadecimais & Endereços MMIO/Flash",
        "",
        "| Hex | Barramento / Classe | Ocorrências | Sample VAs |",
        "| :--- | :--- | ---: | :--- |",
        *[
            f"| `{x['hex']}` | `{x['category']}` | {x['function_count']} | "
            + ", ".join(f"`{v}`" for v in _str_list(x.get("sample_vas"))[:4])
            + " |"
            for x in hexs[:40]
        ],
        "",
        "## 4. Top Máscaras & Operações Bitwise (`doc_*.txt`)",
        "",
        "| Padrão Bitwise | Funções | Sample VAs |",
        "| :--- | ---: | :--- |",
        *[
            f"| `{b['pattern']}` | {b['occurrence_count']} | "
            + ", ".join(f"`{v}`" for v in _str_list(b.get("sample_vas"))[:4])
            + " |"
            for b in bws[:30]
        ],
        "",
        "## 5. Funções & Callers Curados (`doc_*.txt` — Amostra Direta)",
        "",
        "| VA | Símbolo / Alias | Tamanho | Callers | Origem |",
        "| :--- | :--- | ---: | :--- | :--- |",
        *[
            f"| `{f['va']}` | `{f['alias']}` | {f['size_bytes']} B | "
            + (", ".join(f"`{c}`" for c in _str_list(f.get("callers"))[:4]) or "—")
            + f" | `{f['source_file']}` |"
            for f in funcs[:60]
        ],
        "",
    ]
    return "\n".join(lines)


def sync_or_check_atlas(
    *,
    root: Path = ROOT,
    json_out: Path | None = None,
    md_out: Path | None = None,
    write: bool = False,
) -> tuple[int, JsonDict]:
    """Generate forensic atlas JSON + Markdown reference cleanly and detect drift."""
    target_json = json_out if json_out is not None else (root / ATLAS_JSON_REL)
    target_md = md_out if md_out is not None else (root / ATLAS_MD_REL)
    atlas = collect_forensic_atlas(root)
    json_text = json.dumps(atlas, indent=2, ensure_ascii=False) + "\n"
    md_text = render_atlas_markdown(atlas)

    if write:
        target_json.parent.mkdir(parents=True, exist_ok=True)
        target_md.parent.mkdir(parents=True, exist_ok=True)
        target_json.write_text(json_text, encoding="utf-8")
        target_md.write_text(md_text, encoding="utf-8")
        return 0, {
            "status": "UPDATED",
            "json_path": str(target_json),
            "md_path": str(target_md),
            "atlas_sha256": atlas["atlas_sha256"],
            "summary": atlas["summary"],
        }

    ok_json = target_json.is_file() and target_json.read_text(encoding="utf-8") == json_text
    ok_md = target_md.is_file() and target_md.read_text(encoding="utf-8") == md_text
    in_sync = ok_json and ok_md
    return (0 if in_sync else 1), {
        "status": "IN_SYNC" if in_sync else "DRIFT_DETECTED",
        "json_path": str(target_json),
        "md_path": str(target_md),
        "atlas_sha256": atlas["atlas_sha256"],
        "summary": atlas["summary"],
    }


def main(argv: Sequence[str] | None = None) -> int:
    """CLI entrypoint supporting `--write`, `--check`, and `--json`."""
    parser = argparse.ArgumentParser(description="Generate or verify the Panadiga Ghidra Forensic Atlas.")
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--write", action="store_true", help="Write deterministic JSON & Markdown atlas")
    mode.add_argument("--check", action="store_true", help="Fail non-zero if atlas files drifted")
    mode.add_argument("--json", nargs="?", const="-", default=None, help="Emit atlas JSON payload")
    args = parser.parse_args(argv)

    if args.json is not None:
        emit_json_payload(collect_forensic_atlas(ROOT), args.json)
        return 0
    code, summary = sync_or_check_atlas(root=ROOT, write=bool(args.write))
    print(f"[{summary['status']}] atlas_sha256={summary['atlas_sha256']}")
    return code


if __name__ == "__main__":
    raise SystemExit(main())
