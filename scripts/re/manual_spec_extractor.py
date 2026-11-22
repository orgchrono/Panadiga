"""Extract PCB, SoC, Flash, EEPROM, and P7501 hardware specs from service manual PDFs (`T1`)."""

from __future__ import annotations

import re
from pathlib import Path
from typing import Final

from scripts.panadiga_cli_bridge import JsonDict, JsonValue
from scripts.panadiga_ops import ROOT
from scripts.re._discovery_common import (
    append_evidence_source,
    build_common_discovery_parser,
    can_mutate_at_tier,
    emit_report,
    evidence_tier,
    iter_profile_entities,
    save_json_dict,
)

DEFAULT_MANUALS_DIR: Final[Path] = ROOT / "manuals_and_schematics"

_PCB_RE: Final[re.Pattern[str]] = re.compile(r"\b(?:VEP|REP)[0-9A-Z]{4,8}\b")
_SOC_RE: Final[re.Pattern[str]] = re.compile(r"\bMN(?:2DS|103|2WS)[0-9A-Z]+\b")
_FLASH_RE: Final[re.Pattern[str]] = re.compile(
    r"\b(?:S29[A-Z0-9]+|M29[A-Z0-9]+|MX29[A-Z0-9]+|AM29[A-Z0-9]+|MBM29[A-Z0-9]+|TC58[A-Z0-9]+)\b"
)
_EEPROM_RE: Final[re.Pattern[str]] = re.compile(
    r"\b(?:C3EB[A-Z0-9]+|RFKFM[0-9A-Z]+|BR24[A-Z0-9]+|24LC[0-9A-Z]+|AT24C[0-9A-Z]+)\b"
)
_P7501_RE: Final[re.Pattern[str]] = re.compile(r"\bP7501\b|\bTO JIGU\b")
_MANUAL_CODE_RE: Final[re.Pattern[str]] = re.compile(r"\bDSD[0-9]{7}[A-Z0-9]+\b")


def extract_specs_from_text(text: str) -> dict[str, list[str]]:
    """Extract sorted unique hardware designators and part numbers from manual text."""
    return {
        "pcb_codes": sorted(set(_PCB_RE.findall(text))),
        "socs": sorted(set(_SOC_RE.findall(text))),
        "flash_chips": sorted(set(_FLASH_RE.findall(text))),
        "eeproms": sorted(set(_EEPROM_RE.findall(text))),
        "manual_codes": sorted(set(_MANUAL_CODE_RE.findall(text))),
        "p7501_markers": sorted(set(_P7501_RE.findall(text))),
    }


def extract_pdf_text(pdf_path: Path, max_pages: int = 120) -> str:
    """Read up to `max_pages` of searchable text from a PDF via PyMuPDF."""
    import pymupdf

    if hasattr(pymupdf, "TOOLS"):
        pymupdf.TOOLS.mupdf_display_errors(False)  # type: ignore[no-untyped-call]
        pymupdf.TOOLS.mupdf_display_warnings(False)  # type: ignore[no-untyped-call]

    chunks: list[str] = []
    try:
        with pymupdf.open(pdf_path) as doc:  # type: ignore[no-untyped-call]
            limit = min(len(doc), max_pages)
            for idx in range(limit):
                try:
                    page = doc.load_page(idx)
                    chunks.append(str(page.get_text("text")))
                except Exception:
                    continue
    except Exception:
        return ""
    return "\n".join(chunks)


def harvest_manuals_into_profiles(
    *,
    manuals_dir: Path = DEFAULT_MANUALS_DIR,
    profiles_dir: Path,
    apply: bool = False,
    max_pages: int = 80,
) -> JsonDict:
    """Scan service manual PDFs (skipping component datasheets) and propose non-destructive `T1` fills."""
    pdfs = sorted(
        p for p in manuals_dir.rglob("*.pdf") if "datasheets" not in p.parts
    ) if manuals_dir.is_dir() else []

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
    board_updates: list[JsonValue] = []
    eeprom_updates: list[JsonValue] = []

    for pdf_path in pdfs:
        text = extract_pdf_text(pdf_path, max_pages=max_pages)
        specs = extract_specs_from_text(text)
        rel = pdf_path.relative_to(ROOT).as_posix() if pdf_path.is_relative_to(ROOT) else pdf_path.as_posix()
        manual_entry: JsonDict = {"pdf": rel}
        for key, vals in specs.items():
            manual_entry[key] = [v for v in vals]
        extracted_manuals.append(manual_entry)

        for b_path, board in boards:
            if not can_mutate_at_tier(evidence_tier(board), "service_manual_only"):
                continue
            pcb = str(board.get("pcb_code", "")).upper()
            if not pcb or pcb not in specs["pcb_codes"]:
                continue
            changed_fields: dict[str, JsonValue] = {}
            if board.get("soc_ref") is None:
                for soc_hit in specs["socs"]:
                    soc_id = soc_by_part.get(soc_hit.upper())
                    if soc_id:
                        board["soc_ref"] = soc_id
                        changed_fields["soc_ref"] = soc_id
                        break
            if board.get("flash_ref") is None:
                for fl_hit in specs["flash_chips"]:
                    matched_flash_id = flash_by_prefix.get(fl_hit.upper()[:9])
                    if matched_flash_id:
                        board["flash_ref"] = matched_flash_id
                        changed_fields["flash_ref"] = matched_flash_id
                        break
            if not board.get("p7501_header_present") and specs["p7501_markers"]:
                board["p7501_header_present"] = True
                changed_fields["p7501_header_present"] = True

            if changed_fields:
                ev = board.get("evidence")
                if isinstance(ev, dict):
                    append_evidence_source(
                        ev,
                        kind="service_manual",
                        reference=rel,
                        target_tier="service_manual_only",
                    )
                board_updates.append({"board_id": board.get("id"), "pdf": rel, "fields": changed_fields})
                if apply:
                    save_json_dict(b_path, board)

        for e_path, eep in eeproms:
            if not can_mutate_at_tier(evidence_tier(eep), "service_manual_only"):
                continue
            if eep.get("service_replacement_part") is not None:
                continue
            part = str(eep.get("part_number", "")).upper()
            if part and part in specs["eeproms"]:
                repl = next((c for c in specs["eeproms"] if c.startswith("RFKFM")), None)
                if repl:
                    eep["service_replacement_part"] = repl
                    eeprom_updates.append({"eeprom_id": eep.get("id"), "pdf": rel, "replacement": repl})
                    if apply:
                        save_json_dict(e_path, eep)

    return {
        "manuals_scanned": len(pdfs),
        "apply": apply,
        "board_updates": board_updates,
        "eeprom_updates": eeprom_updates,
        "manuals": extracted_manuals,
    }


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for `manual_spec_extractor.py`."""
    parser = build_common_discovery_parser("Extract T1 hardware specs from service manual PDFs into profiles/")
    parser.add_argument("--manuals-dir", type=Path, default=DEFAULT_MANUALS_DIR, help="Directory with PDF manuals")
    parser.add_argument("--max-pages", type=int, default=80, help="Max pages read per PDF")
    args = parser.parse_args(argv)

    rep = harvest_manuals_into_profiles(
        manuals_dir=args.manuals_dir,
        profiles_dir=args.profiles_dir,
        apply=bool(args.apply),
        max_pages=int(args.max_pages),
    )
    emit_report(rep, args.json)
    if args.json is None or str(args.json) != "-":
        print(
            f"[manual_spec_extractor] pdfs={rep['manuals_scanned']} board_updates={len(list(rep['board_updates'])) if isinstance(rep['board_updates'], list) else 0} apply={rep['apply']}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
