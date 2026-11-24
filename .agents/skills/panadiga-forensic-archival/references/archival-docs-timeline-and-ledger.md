# Padrões de Arquivismo, Vault, Evidence Ledger & Template Portátil

## 1. Contrato do Vault Forense (`crates/panadiga-core/src/vault/`)
- Diretório: `<base_root>/sessions/<session_id>/` contendo `session_manifest.json` e `SHA256SUMS.{raw,frm,recompiled}.txt`.
- **Regras por Bucket:**
  - `raw/`: apenas `.bin`, `.img`, `.nvram`, `.dump` (bloqueia `.frm`); falha fechado se já existir o mesmo nome com SHA-256 divergente.
  - `frm/`: apenas `.frm`; imutável por SHA-256.
  - `recompiled/`: bloqueado por padrão se `PreWriteGateReport.passed == false`; grava `recompiled/emu_gate_report.json` e atualiza `manifest.last_emu_gate`.
- **Máquina de 6 Etapas (`StrictWorkflowStep`):** `SelectModel` -> `BuildCleanroomSd` -> `SetupPicoProbe` -> `CaptureAndBackup` -> `OrganizeAndRecompile` -> `EmulateAndFlash`.
- **Sanitização de Path:** `validate_safe_segment` bloqueia path traversal (`..`, barras/prefixos absolutos).

## 2. Schema do Evidence Ledger (`docs/evidence/ledger.json`)
```json
{
  "schema_version": 1,
  "levels": ["N1_STATIC", "N2_EMULATED", "N3_LIVE_PROBE", "N4_HARDWARE", "N5_REPRODUCED"],
  "entries": [
    {
      "id": "EV-YYYYMMDD-NNN",
      "level": "N4_HARDWARE",
      "claim_pt": "Afirmação verificada em português...",
      "claim_en": "Verified statement in English...",
      "sites": ["VA / endereço / registrador"],
      "evidence_files": ["docs/debug_logs/..."],
      "sha256_assertions": [
        { "path": "docs/debug_logs/...", "sha256": "<64-HEX-UPPER>", "required": true }
      ],
      "docs": ["docs/pt-BR/..."]
    }
  ]
}
```
- **Estado atual:** `36` entradas (`EV-20261119-001`..`EV-20261124-003`: `22` `N4_HARDWARE`, `8` `N2_EMULATED`, `6` `N1_STATIC`, `0` `N5_REPRODUCED`).

## 3. Manutenção de Timeline & Paridade Documental
- **Linha do Tempo Cronológica:** Registre marcos datados (refutações, dumps ouro, reparos BGO, probes `CH1`) em `HISTORICO_UNBRICK_*`, `COMPENDIO_GLOBAL_*` §5 e snapshots datados em `docs/audits/` (sem editar snapshots históricos).
- **Trilhas Diátaxis (`docs/STATUS.md`):** Organize dossiês pelas 5 trilhas (A Bancada/Pico, B U99/Decrypto, C RE/Ghidra/GDB, D Storage/ODD/IR, E TBC/Arquitetura/Onboarding/QA).
- **Lembrete de Sincronia:** `docs/INDEX.md` (tabelas bidirecionais EN/PT), `docs/translation_pairs.json` e `test_docs_parity.py` garantem paridade exata; nunca use badges `[PROVADO EM HARDWARE REAL]` em documentos `en-US` ou sem lastro `N4_HARDWARE` no ledger.

## 4. Template Portátil de Arquivismo para Novos Projetos
Ao iniciar novo repositório de RE/forense de hardware:
1. Crie desde o Dia 0: `vault/` (3 buckets imutáveis), `evidence/ledger.json` (`N1..N5`), `debug_logs/README.md` (com SHA-256 obrigatório de cada captura), `manuals/` (`SHA256SUMS.txt`) e `docs/INDEX.md`.
2. Separe binários brutos pesados em Releases externas versionadas por hash (`FIRMWARE_SHA256SUMS.txt`).
3. Escreva um teste automatizado em CI (como `test_docs_parity.py`) verificando SHA-256 do ledger, links relativos e `git check-ignore` a cada commit.
