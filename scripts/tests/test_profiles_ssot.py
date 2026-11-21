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


def test_all_12_json_schemas_and_v2_entities_parse_cleanly() -> None:
    """Every Draft 2020-12 schema and every v2 entity JSON parses cleanly with valid tiers."""
    for name in EXPECTED_SCHEMAS:
        schema_path = SCHEMA_DIR / name
        assert schema_path.is_file(), f"Missing schema file: {schema_path}"
        parsed = json.loads(schema_path.read_text(encoding="utf-8"))
        assert "$schema" in parsed

    for sub in ENTITY_DIRS:
        files = sorted((PROFILES_DIR / sub).glob("*.json"))
        assert files, f"Expected entity files under profiles/{sub}"
        for path in files:
            data = json.loads(path.read_text(encoding="utf-8"))
            if sub == "models":
                for axis_data in data["claims"].values():
                    assert axis_data["tier"] in VALID_TIERS
            else:
                assert data["evidence"]["tier"] in VALID_TIERS


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
        *sorted(PROFILES_DIR.rglob("*.json")),
    ]
    for path in targets:
        text = path.read_text(encoding="utf-8")
        hits = STALE_SA_PATTERN.findall(text)
        assert not hits, f"Stale sector label {hits!r} present in {path.relative_to(ROOT)}"
