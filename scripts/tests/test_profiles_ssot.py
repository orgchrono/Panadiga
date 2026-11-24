"""End-to-end SSOT contract tests for v2 `profiles/` manifests, schemas, CLI validation, and SA hygiene."""

from __future__ import annotations

import json
import re
import shutil
import subprocess
import tempfile
from pathlib import Path
from typing import Final

ROOT: Final[Path] = Path(__file__).resolve().parents[2]
PROFILES_DIR: Final[Path] = ROOT / "profiles"
SCHEMA_DIR: Final[Path] = PROFILES_DIR / "schema"

EXPECTED_SCHEMAS: Final[tuple[str, ...]] = (
    "board.schema.json",
    "capability_authorization.schema.json",
    "eeprom_chip.schema.json",
    "firmware_train.schema.json",
    "flash_chip.schema.json",
    "media_format.schema.json",
    "model_manifest.schema.json",
    "nor_layout.schema.json",
    "profiles_bundle.schema.json",
    "resolved_model_profile.schema.json",
    "soc.schema.json",
    "soc_core.schema.json",
)

ENTITY_DIRS: Final[tuple[str, ...]] = (
    "soc_cores",
    "socs",
    "flash_chips",
    "eeprom_chips",
    "firmware_trains",
    "nor_layouts",
    "media_formats",
    "boards",
    "models",
)

VALID_TIERS: Final[set[str]] = {
    "community_unverified",
    "service_manual_only",
    "official_frm_verified",
    "physical_dump_verified",
    "bench_verified",
}

STALE_SA_PATTERN: Final[re.Pattern[str]] = re.compile(
    r"\b(?:SA0\.\.SA3|SA4\.\.SA133|SA38\.\.SA45|SA70\.\.SA77|SA254\.\.SA269)\b"
)


def _find_panadiga_exe() -> str:
    for candidate in (
        ROOT / "target" / "debug" / "panadiga.exe",
        ROOT / "target" / "debug" / "panadiga",
        ROOT / "target" / "release" / "panadiga.exe",
        ROOT / "target" / "release" / "panadiga",
    ):
        if candidate.is_file():
            return str(candidate)
    return "cargo"


def _run_profiles_validate(overlay_dir: Path | None = None) -> subprocess.CompletedProcess[str]:
    exe = _find_panadiga_exe()
    cmd = (
        ["cargo", "run", "-q", "-p", "panadiga-cli", "--", "profiles", "validate"]
        if exe == "cargo"
        else [exe, "profiles", "validate"]
    )
    if overlay_dir is not None:
        cmd.extend(["--overlay-dir", str(overlay_dir)])
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, check=False)


ENTITY_SCHEMA_MAP: Final[dict[str, str]] = {
    "soc_cores": "soc_core.schema.json",
    "socs": "soc.schema.json",
    "flash_chips": "flash_chip.schema.json",
    "eeprom_chips": "eeprom_chip.schema.json",
    "firmware_trains": "firmware_train.schema.json",
    "nor_layouts": "nor_layout.schema.json",
    "media_formats": "media_format.schema.json",
    "boards": "board.schema.json",
    "models": "model_manifest.schema.json",
}

CORE_JSON_CATALOGS: Final[tuple[str, ...]] = (
    "crates/panadiga-core/src/ssot/hardware_catalog.json",
    "crates/panadiga-core/src/ssot/hardware_catalog_labels.json",
    "crates/panadiga-core/src/ghidra/specs/ghidra_maps_ssot.json",
    "crates/panadiga-ir/src/ir_macros_ssot.json",
    "crates/panadiga-core/src/re/error_dict/firmware_error_dict.json",
)


def test_all_12_json_schemas_and_v2_entities_parse_cleanly() -> None:
    """Every Draft 2020-12 schema and every v2 entity JSON parses cleanly with required fields & valid tiers."""
    schemas: dict[str, dict[str, object]] = {}
    for name in EXPECTED_SCHEMAS:
        schema_path = SCHEMA_DIR / name
        assert schema_path.is_file(), f"Missing schema file: {schema_path}"
        parsed = json.loads(schema_path.read_text(encoding="utf-8"))
        assert "$schema" in parsed
        schemas[name] = parsed

    for sub in ENTITY_DIRS:
        files = sorted((PROFILES_DIR / sub).glob("*.json"))
        assert files, f"Expected entity files under profiles/{sub}"
        schema_obj = schemas[ENTITY_SCHEMA_MAP[sub]]
        required_keys = schema_obj.get("required")
        for path in files:
            data = json.loads(path.read_text(encoding="utf-8"))
            if isinstance(required_keys, list):
                for req_key in required_keys:
                    assert req_key in data, f"Missing required key {req_key!r} in {path.relative_to(ROOT)}"
            if sub == "models":
                for axis_data in data["claims"].values():
                    assert axis_data["tier"] in VALID_TIERS
            else:
                assert data["evidence"]["tier"] in VALID_TIERS

    for rel in CORE_JSON_CATALOGS:
        catalog_path = ROOT / rel
        assert catalog_path.is_file(), f"Missing core JSON catalog: {rel}"
        parsed_cat = json.loads(catalog_path.read_text(encoding="utf-8"))
        assert isinstance(parsed_cat, (dict, list)) and bool(parsed_cat), (
            f"Core JSON catalog must be non-empty object or array: {rel}"
        )

    traps_cat = json.loads((PROFILES_DIR / "traps" / "geometry_and_binary_traps.json").read_text(encoding="utf-8"))
    assert isinstance(traps_cat, dict) and traps_cat.get("schema_version") == 1 and traps_cat.get("traps")
    atlas_cat = json.loads((PROFILES_DIR / "forensic_atlas" / "firmware_forensic_atlas.json").read_text(encoding="utf-8"))
    assert isinstance(atlas_cat, dict) and isinstance(atlas_cat.get("atlas_sha256"), str) and atlas_cat.get("summary")


def test_cli_profiles_validate_succeeds_and_rejects_tampered_overlay() -> None:
    """Built-in v2 validation passes; LVTTL=5000 overlay or built-in overwrite fails closed."""
    ok_res = _run_profiles_validate()
    assert ok_res.returncode == 0, f"validate failed: {ok_res.stderr}\n{ok_res.stdout}"

    tmp_dir = Path(tempfile.mkdtemp(prefix="panadiga_py_overlay_"))
    try:
        cores_dir = tmp_dir / "soc_cores"
        cores_dir.mkdir(parents=True)
        am34 = json.loads((PROFILES_DIR / "soc_cores" / "mn10300_am34.json").read_text(encoding="utf-8"))
        overwrite_res_path = cores_dir / "mn10300_am34.json"
        overwrite_res_path.write_text(json.dumps(am34), encoding="utf-8")
        overwrite_res = _run_profiles_validate(tmp_dir)
        assert overwrite_res.returncode != 0

        overwrite_res_path.unlink()
        am34["id"] = "tampered_5v_core"
        am34["logic_voltage_mv"] = 5000
        (cores_dir / "tampered_5v_core.json").write_text(json.dumps(am34), encoding="utf-8")
        volt_res = _run_profiles_validate(tmp_dir)
        assert volt_res.returncode != 0
    finally:
        shutil.rmtree(tmp_dir, ignore_errors=True)


def test_no_stale_sa_labels_in_rules_release_or_profiles() -> None:
    """Forbids stale SA0..SA3 / SA4..SA133 / wrong SA38/SA70/SA254 tokens across SSOT & docs rules."""
    targets: list[Path] = [
        ROOT / "AGENTS.md",
        ROOT / "GEMINI.md",
        ROOT / ".agents" / "rules" / "AGENTS.md",
        ROOT / ".github" / "RELEASE_NOTES.md",
        ROOT / "docs" / "evidence" / "ledger.json",
        *(ROOT / rel for rel in CORE_JSON_CATALOGS),
        *sorted(PROFILES_DIR.rglob("*.json")),
    ]
    for path in targets:
        text = path.read_text(encoding="utf-8")
        hits = STALE_SA_PATTERN.findall(text)
        assert not hits, f"Stale sector label {hits!r} present in {path.relative_to(ROOT)}"
