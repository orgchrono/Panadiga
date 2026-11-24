from __future__ import annotations

from pathlib import Path

import pytest

from scripts.build_lanes.android_verify import (
    BENCH_KEY_ALIAS,
    BENCH_KEY_PASS,
    DEFAULT_BENCH_KEYSTORE,
    ENV_KEY_PASS,
    ENV_KS_PASS,
    SigningConfig,
    build_apksigner_cmd,
    build_keytool_cmd,
    resolve_signing_config,
)


def test_apksigner_and_keytool_pass_secrets_via_env_only(tmp_path: Path) -> None:
    ks = tmp_path / "release.keystore"
    ks.write_bytes(b"mock-jks")
    cfg = SigningConfig(
        keystore_path=ks,
        key_alias="panadigarelease",
        ks_pass="s3cr3t-ks-pass",
        key_pass="s3cr3t-key-pass",
        require_release_keystore=True,
    )

    apk_cmd = build_apksigner_cmd(
        Path("apksigner"),
        cfg,
        tmp_path / "aligned.apk",
        tmp_path / "signed.apk",
    )
    keytool_cmd = build_keytool_cmd(cfg)

    joined_apk = " ".join(apk_cmd)
    joined_kt = " ".join(keytool_cmd)
    assert "s3cr3t" not in joined_apk
    assert "s3cr3t" not in joined_kt
    assert f"pass:{BENCH_KEY_PASS}" not in joined_apk
    assert f"env:{ENV_KS_PASS}" in apk_cmd
    assert f"env:{ENV_KEY_PASS}" in apk_cmd
    assert f"-storepass:env" in keytool_cmd and ENV_KS_PASS in keytool_cmd
    assert f"-keypass:env" in keytool_cmd and ENV_KEY_PASS in keytool_cmd


def test_resolve_signing_config_rejects_bench_defaults_in_release_mode(tmp_path: Path) -> None:
    ks = tmp_path / "release.keystore"
    ks.write_bytes(b"mock-jks")

    with pytest.raises(RuntimeError, match="DEFAULT_BENCH_KEYSTORE"):
        resolve_signing_config(
            DEFAULT_BENCH_KEYSTORE,
            require_release_keystore=True,
            env={ENV_KS_PASS: "custom-pass-1", ENV_KEY_PASS: "custom-pass-1"},
        )

    with pytest.raises(FileNotFoundError, match="Release keystore not found"):
        resolve_signing_config(
            tmp_path / "missing.keystore",
            require_release_keystore=True,
            env={ENV_KS_PASS: "custom-pass-1", ENV_KEY_PASS: "custom-pass-1"},
        )

    with pytest.raises(RuntimeError, match=ENV_KS_PASS):
        resolve_signing_config(
            ks,
            require_release_keystore=True,
            env={ENV_KS_PASS: BENCH_KEY_PASS, ENV_KEY_PASS: BENCH_KEY_PASS},
        )

    cfg = resolve_signing_config(
        ks,
        key_alias="prod_alias",
        require_release_keystore=True,
        env={ENV_KS_PASS: "prod-ks-secret", ENV_KEY_PASS: "prod-key-secret"},
    )
    assert cfg.key_alias == "prod_alias"
    assert cfg.ks_pass == "prod-ks-secret"
    assert cfg.key_pass == "prod-key-secret"

    dev_cfg = resolve_signing_config(DEFAULT_BENCH_KEYSTORE, require_release_keystore=False, env={})
    assert dev_cfg.key_alias == BENCH_KEY_ALIAS
    assert dev_cfg.ks_pass == BENCH_KEY_PASS
