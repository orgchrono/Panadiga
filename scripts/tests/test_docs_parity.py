"""Automated documentation parity, log inventory, stale-hash, and link integrity checks."""

from __future__ import annotations

import json
import re
import subprocess
from pathlib import Path
from urllib.parse import unquote

from scripts.panadiga_ops import ROOT, sha256_file

DOCS = ROOT / "docs"
STALE_PANA_SD_PREFIX = "bc97b514"
STALE_PANA_SD_HASH = "bc97b514ec9bce895049cac0681b58de5fd294a700291eb34fad7c5910c3243e"
_PT_MATURITY_BADGES = (
    "[provado em hardware real]",
    "[provado em emulador mn103vm]",
    "[evidência estática ghidra / estágio 0]",
    "[em investigação / bloqueio físico ch1]",
)
EXPECTED_PAIRS = 43
EXPECTED_LOGS = 28

_INDEX_ROW_RE = re.compile(
    r"^\|\s*\d+\s*\|\s*\[([^\]]+\.md)\]\((en-US|pt-BR)/([^)\s]+)\)"
    r"\s*\|\s*\[(?:PT-BR|EN-US)\]\((pt-BR|en-US)/([^)\s]+)\)\s*\|",
    re.MULTILINE,
)
_MD_LINK_RE = re.compile(r"\[[^\]]+\]\(([^)\s]+?)(?:#[^)\s]*)?\)")


def _load_translation_manifest() -> dict[str, object]:
    payload: object = json.loads((DOCS / "translation_pairs.json").read_text(encoding="utf-8"))
    assert isinstance(payload, dict)
    return {str(k): v for k, v in payload.items()}


def _load_pairs() -> dict[str, str]:
    raw_pairs = _load_translation_manifest().get("pairs")
    assert isinstance(raw_pairs, dict)
    return {str(k): str(v) for k, v in raw_pairs.items()}


def test_bilingual_pair_parity() -> None:
    """Every pt-BR dossier pairs 1:1 with en-US and translation_pairs.json."""
    pairs = _load_pairs()
    assert len(pairs) == EXPECTED_PAIRS

    pt_files = {path.name for path in (DOCS / "pt-BR").glob("*.md")}
    en_files = {path.name for path in (DOCS / "en-US").glob("*.md")}
    assert set(pairs.keys()) == pt_files
    assert len(set(pairs.values())) == EXPECTED_PAIRS
    assert set(pairs.values()) == en_files

    manifest = _load_translation_manifest()
    root_pairs = manifest.get("root_pairs")
    root_docs = manifest.get("root_docs")
    assert isinstance(root_pairs, dict) and isinstance(root_docs, list)
    for pt_root, en_root in root_pairs.items():
        assert (ROOT / str(pt_root)).is_file()
        assert (ROOT / str(en_root)).is_file()
    for doc_name in root_docs:
        assert (ROOT / str(doc_name)).is_file()


def test_index_bidirectional_tables_and_counts() -> None:
    """Both English and Portuguese tables in docs/INDEX.md match translation_pairs.json 1:1."""
    pairs = _load_pairs()
    index_text = (DOCS / "INDEX.md").read_text(encoding="utf-8")

    en_table: dict[str, str] = {}
    pt_table: dict[str, str] = {}
    for label, left_lang, left_file, right_lang, right_file in _INDEX_ROW_RE.findall(index_text):
        assert label == left_file
        if left_lang == "en-US" and right_lang == "pt-BR":
            en_table[left_file] = right_file
        elif left_lang == "pt-BR" and right_lang == "en-US":
            pt_table[left_file] = right_file
        else:
            raise AssertionError(f"Unexpected table language direction: {left_lang} -> {right_lang}")

    assert len(en_table) == EXPECTED_PAIRS
    assert len(pt_table) == EXPECTED_PAIRS
    assert pt_table == pairs
    assert {pt_name: en_name for en_name, pt_name in en_table.items()} == pairs
    assert f"All **{EXPECTED_PAIRS}** primary" in index_text
    assert f"(`docs/en-US/`) — {EXPECTED_PAIRS} Dossiers" in index_text
    assert f"(`docs/pt-BR/`) — {EXPECTED_PAIRS} Dossiês" in index_text


def test_root_and_index_counts_synchronized() -> None:
    """Root documentation, AGENTS.md, release notes, and debug log counts stay synchronized."""
    checks: tuple[tuple[Path, tuple[str, ...]], ...] = (
        (
            ROOT / "README.md",
            (
                f"{EXPECTED_PAIRS} paired bilingual",
                f"{EXPECTED_PAIRS} synchronized bilingual",
                f"`{EXPECTED_PAIRS}` EN + `{EXPECTED_PAIRS}` PT-BR",
                f"{EXPECTED_LOGS} deduplicated live",
                "All 26 Native Tools",
                "**26 native tools**",
            ),
        ),
        (
            ROOT / "README.pt-BR.md",
            (
                f"{EXPECTED_PAIRS} dossiês técnicos bilíngues",
                f"`{EXPECTED_PAIRS}` PT-BR + `{EXPECTED_PAIRS}` EN-US",
                f"{EXPECTED_LOGS} capturas e logs únicos",
                "Expondo 26 Ferramentas Nativas",
                "**26 ferramentas nativas**",
            ),
        ),
        (
            ROOT / "AGENTS.md",
            (
                f"**{EXPECTED_PAIRS} dossiês técnicos bilíngues canônicos**",
                f"Raiz + {EXPECTED_PAIRS} Pares Bilíngues",
                f"**{EXPECTED_LOGS} capturas/logs reais de bancada**",
                "**26 tools**",
            ),
        ),
        (
            DOCS / "debug_logs" / "README.md",
            (
                f"**{EXPECTED_LOGS} capturas e transcrições verificadas**",
                f"**{EXPECTED_LOGS} verified bench captures and transcripts**",
            ),
        ),
        (
            ROOT / ".github" / "RELEASE_NOTES.md",
            (
                f"`{EXPECTED_PAIRS}` bilingual dossiers & `{EXPECTED_LOGS}` curated",
                f"({EXPECTED_PAIRS} EN/PT-BR Dossier Pairs)",
                "**26** JSON-RPC 2.0 stdio tools",
            ),
        ),
    )
    for path, needles in checks:
        text = path.read_text(encoding="utf-8")
        for needle in needles:
            assert needle in text, f"Missing {needle!r} in {path.relative_to(ROOT)}"


def test_debug_logs_inventory_and_sha256() -> None:
    """Every curated .log file in docs/debug_logs exists, is indexed, and matches README SHA-256."""
    logs_dir = DOCS / "debug_logs"
    log_files = sorted(logs_dir.glob("*.log"))
    assert len(log_files) == EXPECTED_LOGS

    readme_upper = (logs_dir / "README.md").read_text(encoding="utf-8").upper()
    index_text = (DOCS / "INDEX.md").read_text(encoding="utf-8")
    for log_path in log_files:
        assert log_path.name.upper() in readme_upper, f"{log_path.name} missing from debug_logs/README.md"
        assert log_path.name in index_text, f"{log_path.name} missing from docs/INDEX.md"
        digest = sha256_file(log_path).upper()
        assert digest in readme_upper, f"SHA-256 {digest} missing for {log_path.name}"


def _assert_not_git_ignored(paths: list[Path]) -> None:
    """Fail if any documentation link target or required ledger path is ignored by .gitignore."""
    if not (ROOT / ".git").exists():
        return
    rels = sorted({p.resolve().relative_to(ROOT.resolve()).as_posix() for p in paths})
    if not rels:
        return
    proc = subprocess.run(
        ["git", "check-ignore", "--stdin"],
        cwd=ROOT,
        input="\n".join(rels) + "\n",
        capture_output=True,
        text=True,
        check=False,
    )
    ignored = [line.strip() for line in proc.stdout.splitlines() if line.strip()]
    assert not ignored, f"Doc links / required ledger paths ignored by .gitignore: {ignored}"


def test_stale_hash_absent_and_relative_links_resolve() -> None:
    """Reject stale PANA__SD.FRM hash in active docs and verify relative links resolve on disk."""
    active_files: list[Path] = [
        ROOT / "README.md",
        ROOT / "README.pt-BR.md",
        ROOT / "AUDITORIA_COMPLETA.md",
        ROOT / "COMPLETE_AUDIT.md",
        ROOT / "AGENTS.md",
        ROOT / ".github" / "RELEASE_NOTES.md",
        *[
            md
            for md in DOCS.rglob("*.md")
            if "audits" not in md.relative_to(DOCS).parts
        ],
    ]
    for path in active_files:
        lower = path.read_text(encoding="utf-8").lower()
        assert STALE_PANA_SD_HASH not in lower, f"Stale hash found in {path.relative_to(ROOT)}"
        assert STALE_PANA_SD_PREFIX not in lower, f"Stale hash prefix found in {path.relative_to(ROOT)}"
        if path.parent.name == "en-US" or path.name in ("COMPLETE_AUDIT.md", "README.md", "RELEASE_NOTES.md"):
            for badge in _PT_MATURITY_BADGES:
                assert badge not in lower, f"Portuguese badge {badge} found in English doc {path.relative_to(ROOT)}"

    link_sources = (
        DOCS / "INDEX.md",
        DOCS / "STATUS.md",
        ROOT / "AUDITORIA_COMPLETA.md",
        ROOT / "COMPLETE_AUDIT.md",
    )
    checked_targets: list[Path] = []
    for src in link_sources:
        text = src.read_text(encoding="utf-8")
        for target in _MD_LINK_RE.findall(text):
            if not target or target.startswith("#") or "://" in target or "<" in target or ">" in target:
                continue
            resolved = (src.parent / unquote(target)).resolve()
            assert resolved.exists(), f"Broken link {target!r} in {src.relative_to(ROOT)}"
            checked_targets.append(resolved)
    _assert_not_git_ignored(checked_targets)


def _require_nonempty_str_list(raw: object, label: str) -> list[str]:
    assert isinstance(raw, list) and len(raw) > 0, f"{label} must be a non-empty list"
    out: list[str] = []
    for item in raw:
        assert isinstance(item, str) and item.strip(), f"{label} entries must be non-empty strings"
        out.append(item)
    return out


def test_evidence_ledger_integrity() -> None:
    """Verify docs/evidence/ledger.json schema, paths, SHA-256 digests, and N4 doc coverage."""
    ledger_raw: object = json.loads((DOCS / "evidence" / "ledger.json").read_text(encoding="utf-8"))
    assert isinstance(ledger_raw, dict)
    assert ledger_raw.get("schema_version") == 1
    levels = _require_nonempty_str_list(ledger_raw.get("levels"), "ledger.levels")
    assert set(levels) == {"N1_STATIC", "N2_EMULATED", "N3_LIVE_PROBE", "N4_HARDWARE", "N5_REPRODUCED"}

    entries = ledger_raw.get("entries")
    assert isinstance(entries, list) and len(entries) >= 16

    seen_ids: set[str] = set()
    n4_covered_docs: set[str] = set()
    required_paths: list[Path] = []
    for raw_entry in entries:
        assert isinstance(raw_entry, dict)
        ev_id = raw_entry.get("id")
        level = raw_entry.get("level")
        claim_pt = raw_entry.get("claim_pt")
        claim_en = raw_entry.get("claim_en")
        assert isinstance(ev_id, str) and ev_id.startswith("EV-") and ev_id not in seen_ids
        seen_ids.add(ev_id)
        assert isinstance(level, str) and level in levels
        assert isinstance(claim_pt, str) and claim_pt.strip()
        assert isinstance(claim_en, str) and claim_en.strip()

        _ = _require_nonempty_str_list(raw_entry.get("sites"), f"{ev_id}.sites")
        evidence_files = _require_nonempty_str_list(raw_entry.get("evidence_files"), f"{ev_id}.evidence_files")
        doc_refs = _require_nonempty_str_list(raw_entry.get("docs"), f"{ev_id}.docs")

        for rel_path in [*evidence_files, *doc_refs]:
            ref_file = ROOT / rel_path
            assert ref_file.is_file(), f"Missing file {rel_path!r} referenced by {ev_id}"
            required_paths.append(ref_file)

        if level in ("N4_HARDWARE", "N5_REPRODUCED"):
            n4_covered_docs.update(doc_refs)

        assertions = raw_entry.get("sha256_assertions")
        assert isinstance(assertions, list)
        for item in assertions:
            assert isinstance(item, dict)
            rel = item.get("path")
            expected_sha = item.get("sha256")
            required = item.get("required")
            assert isinstance(rel, str) and rel.strip()
            assert isinstance(expected_sha, str) and len(expected_sha) == 64
            assert isinstance(required, bool)
            target = ROOT / rel
            if required:
                assert target.is_file(), f"Missing required SHA-256 target {rel!r} in {ev_id}"
                required_paths.append(target)
            if target.is_file():
                assert sha256_file(target).upper() == expected_sha.upper(), (
                    f"SHA-256 mismatch for {rel!r} in {ev_id}"
                )

    _assert_not_git_ignored(required_paths)

    for md in sorted((DOCS / "pt-BR").glob("*.md")):
        if "[provado em hardware real]" in md.read_text(encoding="utf-8").lower():
            rel_doc = md.relative_to(ROOT).as_posix()
            assert rel_doc in n4_covered_docs, (
                f"{rel_doc} asserts [PROVADO EM HARDWARE REAL] without an N4_HARDWARE ledger entry"
            )
