"""Extract PCB, SoC, Flash, EEPROM, and P7501 specs from PDFs into quarantined staging proposals (R1)."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    build_common_discovery_parser,
    can_mutate_at_tier,
    emit_report,
    evidence_tier,
    iter_profile_entities,
    save_json_dict,
    slugify_identifier,
)

DEFAULT_MANUALS_DIR: Final[Path] = ROOT / "manuals_and_schematics"

_TOKEN_PATTERNS: Final[dict[str, re.Pattern[str]]] = {
    "pcb_codes": re.compile(r"\b(?:VEP|REP)[0-9A-Z]{4,8}\b"),
    "socs": re.compile(r"\bMN(?:2DS|103|2WS)[0-9A-Z]+\b"),
    "flash_chips": re.compile(
        r"\b(?:S29[A-Z0-9]+|M29[A-Z0-9]+|MX29[A-Z0-9]+|AM29[A-Z0-9]+|MBM29[A-Z0-9]+|TC58[A-Z0-9]+)\b"
    ),
    "eeproms": re.compile(r"\b(?:C3EB[A-Z0-9]+|RFKFM[0-9A-Z]+|BR24[A-Z0-9]+|24LC[0-9A-Z]+|AT24C[0-9A-Z]+)\b"),
    "manual_codes": re.compile(r"\bDSD[0-9]{7}[A-Z0-9]+\b"),
    "p7501_markers": re.compile(r"\bP7501\b|\bTO JIGU\b"),
}


def extract_specs_from_text(text: str) -> dict[str, list[str]]:
    """Extract sorted unique hardware designators and part numbers from manual text."""
    return {key: sorted(set(pat.findall(text))) for key, pat in _TOKEN_PATTERNS.items()}


def extract_specs_with_citations(pages: list[tuple[int, str]]) -> dict[str, dict[str, list[int]]]:
    """Extract hardware tokens keyed by 1-based page numbers for human-review provenance."""
    out: dict[str, dict[str, list[int]]] = {k: {} for k in _TOKEN_PATTERNS}
    for page_num, page_text in pages:
        for category, pat in _TOKEN_PATTERNS.items():
            bucket = out[category]
            for hit in sorted(set(pat.findall(page_text))):
                bucket.setdefault(hit, [])
                if page_num not in bucket[hit]:
                    bucket[hit].append(page_num)
    return out


def extract_pdf_pages(pdf_path: Path, max_pages: int = 120) -> list[tuple[int, str]]:
    """Read up to `max_pages` searchable PDF pages via PyMuPDF returning 1-based `(page, text)` pairs."""
    import pymupdf

    if hasattr(pymupdf, "TOOLS"):
        pymupdf.TOOLS.mupdf_display_errors(False)  # type: ignore[no-untyped-call]
        pymupdf.TOOLS.mupdf_display_warnings(False)  # type: ignore[no-untyped-call]

    pages: list[tuple[int, str]] = []
    try:
        with pymupdf.open(pdf_path) as doc:  # type: ignore[no-untyped-call]
            limit = min(len(doc), max_pages)
            for idx in range(limit):
                try:
                    page = doc.load_page(idx)
                    pages.append((idx + 1, str(page.get_text("text"))))
                except Exception:
                    continue
    except Exception:
        return []
    return pages


def extract_pdf_text(pdf_path: Path, max_pages: int = 120) -> str:
    """Read up to `max_pages` of searchable text from a PDF via PyMuPDF."""
    return "\n".join(text for _, text in extract_pdf_pages(pdf_path, max_pages=max_pages))


def _citations_as_json(citations: dict[str, list[int]]) -> JsonDict:
    return {tok: [int(p) for p in pg_list] for tok, pg_list in citations.items()}


def _build_quarantine_proposal(
    *,
    target_collection: str,
    entity_id: str,
    source_pdf: str,
    proposed_fields: dict[str, JsonValue],
    page_citations: JsonDict,
) -> JsonDict:
    return {
        "schema_version": 1,
        "status": "quarantined_pending_human_review",
        "requires_human_review": True,
        "target_collection": target_collection,
        "entity_id": entity_id,
        "source_pdf": source_pdf,
        "proposed_tier": "service_manual_only",
        "proposed_fields": dict(proposed_fields),
        "page_citations": page_citations,
        "warning": (
            "Automated PDF/OCR extraction may mix multi-region BOM alternates or confuse similar digits; "
            "verify cited pages manually before promoting into profiles/."
        ),
    }


def harvest_manuals_into_profiles(
    *,
    manuals_dir: Path = DEFAULT_MANUALS_DIR,
    profiles_dir: Path,
    staging_dir: Path | None = None,
    apply: bool = False,
    max_pages: int = 80,
) -> JsonDict:
    """Scan service manual PDFs and emit quarantined `T1` staging proposals without mutating `profiles/*.json`."""
    pdfs = (
        sorted(p for p in manuals_dir.rglob("*.pdf") if "datasheets" not in p.parts)
        if manuals_dir.is_dir()
        else []
    )
    resolved_staging = staging_dir or (profiles_dir / "staging" / "manual_proposals")

    socs = iter_profile_entities(profiles_dir, "socs")
    flashes = iter_profile_entities(profiles_dir, "flash_chips")
    eeproms = iter_profile_entities(profiles_dir, "eeprom_chips")
    boards = iter_profile_entities(profiles_dir, "boards")

    soc_by_part = {str(s.get("part_number", "")).upper(): str(s.get("id", "")) for _, s in socs}
    flash_by_prefix: dict[str, str] = {}
    for _, fl in flashes:
        part = str(fl.get("part_number", "")).upper()
        fid = str(fl.get("id", ""))
        if part and fid:
            flash_by_prefix[part[:9]] = fid

    extracted_manuals: list[JsonValue] = []
    board_proposals: list[JsonValue] = []
    eeprom_proposals: list[JsonValue] = []
    emitted_staging_files: list[str] = []

    for pdf_path in pdfs:
        text = extract_pdf_text(pdf_path, max_pages=max_pages)
        pages = [(1, text)] if text else extract_pdf_pages(pdf_path, max_pages=max_pages)
        combined = "\n".join(pg_txt for _, pg_txt in pages)
        specs = extract_specs_from_text(combined)
        cited = extract_specs_with_citations(pages)
        rel = pdf_path.relative_to(ROOT).as_posix() if pdf_path.is_relative_to(ROOT) else pdf_path.as_posix()
        pdf_slug = slugify_identifier(pdf_path.stem)

        manual_entry: JsonDict = {"pdf": rel}
        for key, vals in specs.items():
            manual_entry[key] = [v for v in vals]
        extracted_manuals.append(manual_entry)

        for _, board in boards:
            if not can_mutate_at_tier(evidence_tier(board), "service_manual_only"):
                continue
            pcb = str(board.get("pcb_code", "")).upper()
            if not pcb or pcb not in specs["pcb_codes"]:
                continue
            proposed: dict[str, JsonValue] = {}
            field_citations: dict[str, list[int]] = {}
            if board.get("soc_ref") is None:
                for soc_hit in specs["socs"]:
                    soc_id = soc_by_part.get(soc_hit.upper())
                    if soc_id:
                        proposed["soc_ref"] = soc_id
                        field_citations["soc_ref"] = cited["socs"].get(soc_hit, [1])
                        break
            if board.get("flash_ref") is None:
                for fl_hit in specs["flash_chips"]:
                    matched_flash_id = flash_by_prefix.get(fl_hit.upper()[:9])
                    if matched_flash_id:
                        proposed["flash_ref"] = matched_flash_id
                        field_citations["flash_ref"] = cited["flash_chips"].get(fl_hit, [1])
                        break
            if not board.get("p7501_header_present") and specs["p7501_markers"]:
                proposed["p7501_header_present"] = True
                first_marker = specs["p7501_markers"][0]
                field_citations["p7501_header_present"] = cited["p7501_markers"].get(first_marker, [1])

            if proposed:
                board_id = str(board.get("id") or pcb.lower())
                proposal = _build_quarantine_proposal(
                    target_collection="boards",
                    entity_id=board_id,
                    source_pdf=rel,
                    proposed_fields=proposed,
                    page_citations=_citations_as_json(field_citations),
                )
                board_proposals.append(proposal)
                if apply:
                    out_file = resolved_staging / f"board__{slugify_identifier(board_id)}__{pdf_slug}.proposal.json"
                    save_json_dict(out_file, proposal)
                    emitted_staging_files.append(out_file.as_posix())

        for _, eep in eeproms:
            if not can_mutate_at_tier(evidence_tier(eep), "service_manual_only"):
                continue
            if eep.get("service_replacement_part") is not None:
                continue
            part = str(eep.get("part_number", "")).upper()
            if part and part in specs["eeproms"]:
                repl = next((c for c in specs["eeproms"] if c.startswith("RFKFM")), None)
                if repl:
                    eep_id = str(eep.get("id") or part.lower())
                    eep_proposal = _build_quarantine_proposal(
                        target_collection="eeprom_chips",
                        entity_id=eep_id,
                        source_pdf=rel,
                        proposed_fields={"service_replacement_part": repl},
                        page_citations=_citations_as_json(
                            {"service_replacement_part": cited["eeproms"].get(repl, [1])}
                        ),
                    )
                    eeprom_proposals.append(eep_proposal)
                    if apply:
                        out_file = resolved_staging / f"eeprom__{slugify_identifier(eep_id)}__{pdf_slug}.proposal.json"
                        save_json_dict(out_file, eep_proposal)
                        emitted_staging_files.append(out_file.as_posix())

    return {
        "manuals_scanned": len(pdfs),
        "apply": apply,
        "quarantine_staging_dir": resolved_staging.as_posix(),
        "direct_profile_writes": 0,
        "emitted_staging_files": [p for p in emitted_staging_files],
        "board_proposals": board_proposals,
        "eeprom_proposals": eeprom_proposals,
        "manuals": extracted_manuals,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `manual_spec_extractor.py`."""
    parser = build_common_discovery_parser("Extract T1 hardware specs from service manual PDFs into quarantine staging/")
    parser.add_argument("--manuals-dir", type=Path, default=DEFAULT_MANUALS_DIR, help="Directory with PDF manuals")
    parser.add_argument("--staging-dir", type=Path, default=None, help="Quarantine staging directory (defaults to profiles/staging/manual_proposals)")
    parser.add_argument("--max-pages", type=int, default=80, help="Max pages read per PDF")
    args = parser.parse_args(argv)

    rep = harvest_manuals_into_profiles(
        manuals_dir=args.manuals_dir,
        profiles_dir=args.profiles_dir,
        staging_dir=args.staging_dir,
        apply=bool(args.apply),
        max_pages=int(args.max_pages),
    )
    emit_report(rep, args.json)
    if args.json is None or str(args.json) != "-":
        proposals = rep.get("board_proposals")
        count = len(proposals) if isinstance(proposals, list) else 0
        print(
            f"[manual_spec_extractor] pdfs={rep['manuals_scanned']} quarantined_board_proposals={count} direct_profile_writes=0 apply={rep['apply']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
