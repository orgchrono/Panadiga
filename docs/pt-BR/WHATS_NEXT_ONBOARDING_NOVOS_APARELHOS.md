> 🌐 **Idioma / Language:** [English (US)](../en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Catálogo de Perfis DIGA](CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# What's Next: Pipeline Completo de Onboarding de Novos Aparelhos DIGA (Bancada → SSOT JSON → Pull Request)

> **Objetivo:** Este guia padroniza o fluxo completo (*end-to-end*) para integrar qualquer novo modelo **Panasonic DIGA** ao ecossistema **Panadiga** usando arquitetura **100% declarativa (SSOT JSON)** — eliminando código *hardcoded* por nome de modelo, garantindo paridade estrita entre CLI/Rust/Tauri/Python/Ghidra e protegendo o silício na bancada.

---

## Visão Geral dos 7 Estágios (`T0` → `T4`)

| Estágio | Objetivo Principal | Ferramentas / Scripts Canônicos | Critério de Saída |
| :--- | :--- | :--- | :--- |
| **1. Bancada & Segurança `P7501`** | Verificar tensões LVTTL 3.3V, `GP3`/`NSW5V` em `Hi-Z`, `XRESET` Open-Drain | Sonda Pico + `panadiga pico status` / `bench` | Log em `docs/debug_logs/` sem overdrive |
| **2. Captura NOR (`2..16 MiB`)** | Obter dump integral IC5001 somente-leitura via UART `P7501` ou gravador TSOP-48 | `panadiga unbrick nor-stub`, `decode-nor-dump`, `panadiga profiles discover-nor`, `scripts/re/nor_intake_analyzer.py` | Dump SHA-256 imutável + mapa `nor_layout` rascunho |
| **3. Desempacotamento `.FRM` & `panamon` / `rebuild-from-dump`** | Identificar versão do `DIGADIAG` (`ver0.61`, `ver0.63`, ...), extrair `DRV`/`PROG`/`MAIN` e validar kits `PANA__SD.FRM` | `scripts/re/frm_train_harvester.py`, `scripts/re/pana_frm.py`, `panadiga unbrick extract-frm`, `panadiga unbrick rebuild-from-dump` | Hashes `EXTRHEAD`, builds `MAIN` e loader perfilados no `firmware_trains/` |
| **4. Hidratação via Manuais PDF (`T1`)** | Extrair códigos PCB (`VEP`/`REP`), SoC (`MN2DS...`), Flash (`S29...`/`MX29...`) e EEPROMs | `scripts/re/manual_spec_extractor.py --apply` | `boards/`, `socs/`, `flash_chips/`, `eeprom_chips/` preenchidos em `T1` |
| **5. Auditoria Ghidra MN103S & Pseudocódigo** | Resolver 7 Armadilhas MN103S, 0 `UNDEF`, identificar VAs de BGO/FTL, TBC, `DAVIS` e capacidade HDD | `tools/ghidra/ghidra_zero_undef_pipeline.py`, `tools/ghidra/ghidra_export_pseudocode_bundle.py`, `panadiga ghidra export-all` | Pacote C/texto plano em `docs/pseudocode/` + `ghidra_ssot_maps.json` |
| **6. Modelagem Declarativa SSOT JSON** | Registrar geometrias BGO/NOR/HDD/TBC por ID de geometria/trem (`BGO_*`, `blobs/*.bin.gz`), jamais por nome de produto em Rust/TS | `profiles/*.json`, `panadiga ssot export-all`, `panadiga profiles promote-evidence`, `panadiga profiles completeness` | `panadiga profiles validate` com `0` violações + nível `T2/T3/T4` |
| **7. Testes, Qualidade & Pull Request** | Garantir paridade CLI + E2E Playwright + 0% clones `jscpd` antes de abrir PR | `python scripts/validate.py` (`9/9` gates) | Todos os 9 Quality Gates verdes + checklist em [`CONTRIBUTING.md`](../../CONTRIBUTING.md) |

---

## Passo a Passo Detalhado na Bancada

### Estágio 1 — Checklist Elétrico Obrigatório (`P7501` / `IC5001` / `CH1`)

Antes de conectar qualquer fio ou alimentador:
1. Confirme no manual de serviço da placa (ex.: `VEP79138`, `VEP79106A`) se o conector **`P7501` (`TO JIGU`)** existe e se opera em **3.3V LVTTL**.
2. Na sonda Pico (`RP2040` / `RP2350`), verifique que **`GP3` (`NSW5V` / Pino 5)** permanece **SEMPRE como `Hi-Z Input` sem pulls**, **`GP5` (`XRESET` / Pino 2)** opera em **Open-Drain** e a sonda **NUNCA é alimentada pelo `P7501`** (use USB ou `VSYS`).
3. Em problemas de boot HDD (`UPD NG`, ausência de `MAIN` ou `0xE8000000` no barramento `DAVIS CH1`), meça `P1503 Pino 4 (DR5V = +5.1V)`, `P1503 Pino 1 (DR12V = +12.3V)` e confirme polaridade de cabo FFC (`Type A` vs `Type B`) antes de assumir erro de firmware.

### Estágio 2 — Captura Física da NOR Flash e Triagem Automática

```bash
# 1. Gere o stub somente leitura para UART (ou use PANA__SD.FRM para entrar em PANAMON1>)
panadiga unbrick nor-stub --start 0x80000000 --len 0x800000 --out tmp/nor_stub.bin --wrap-digadiag

# 2. Após capturar PANADMP1..PANADONE em docs/debug_logs/, decodifique o binário imutável:
panadiga unbrick decode-nor-dump -i docs/debug_logs/captura_nova.log -o tmp/dmr_novo_8mb.bin

# 3. Extraia mapa preliminar (Loader banner, EXTRHEAD, setores BGO 0xF101/0xF2xx e EEMUL 0xF1/1284):
panadiga profiles discover-nor --input tmp/dmr_novo_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles --json tmp/nor_report.json
# Ou em Python offline com heurísticas detalhadas:
python -m scripts.re.nor_intake_analyzer --input tmp/dmr_novo_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles
```

> **Invariante Forense:** Nunca edite dumps originais (*in-place*). Registre o SHA-256 original no manifesto do cofre (`panadiga session init` + `ingest`) e salve logs reais em `docs/debug_logs/` atualizando `docs/debug_logs/README.md`.

### Estágio 3 — Versão de `DIGADIAG`, Construção de `pana_*` via `panamon` e `rebuild-from-dump`

Cada geração DIGA exige uma família exata de `Loader` e empacotamento `FRM`:
- **`Loader ver0.61` (`06/03/06`, `SDDIAG` clássico — ex.: `DMR-EH55`)**: consome `DIGADIAG.FRM` em **raw `256 KiB`** (`PROG061` descriptografado sem cabeçalho `EXTRHEAD`) e `PANA__SD.FRM` com entrada de TOC de **`48 bytes` (`sd48`)**. O módulo `PROG061` carrega em **`0x50400000`** via salto `50400000` (monitor `PANAMON1>`).
- **`Loader ver0.63+` (`bootbin` / modelos posteriores)**: consome `DIGADIAG.FRM` encapsulado em **`128 KiB EXTRHEAD`** com `PANA__SD.FRM` de **`64 bytes` (`sd64`)**.

Para construir binários compatíveis a partir de pacotes oficiais ou da própria NOR + HDD:

```bash
# Extrair e descriptografar DRV / PROG / MAIN de qualquer PANA_DVD.FRM oficial ou atualizar catálogo FRM:
python -m scripts.re.frm_train_harvester --unpack-missing --apply

# Reconstruir o kit completo DIGADIAG.FRM + PANA__SD.FRM diretamente de dump NOR + Partição 1 MAIN:
panadiga unbrick rebuild-from-dump \
  --model DMR-EH55 \
  --nor-dump tmp/dmr_novo_8mb.bin \
  --hdd-main tmp/partition_1_am34_os.bin \
  --target-gb 250 \
  --out-dir tmp/sd_kit_rebuild \
  --json tmp/rebuild_report.json
```

Se houver divergência de modelo (`UPD NG` por tabela `<deviceinfo>` ou `DEV.BIN` no `panamon` / `MAIN`), inspecione e ajuste de forma declarativa via `panadiga unbrick region-compat` ou forneça explicitamente `--hdd-main` e `--frm` compatíveis. Para experimentação *clean-room* em SDRAM (`panamon2` / TBC standalone sem `MAIN` no disco), veja o protótipo em [`firmware/panadiga-blackbox-fw/README.md`](../../firmware/panadiga-blackbox-fw/README.md).

### Estágio 4 — Hidratação Declarativa a Partir dos Manuais PDF (`T1`)

Adicione o PDF de serviço em `manuals_and_schematics/` (atualizando `manuals_and_schematics/README.md`) e execute:

```bash
python -m scripts.re.manual_spec_extractor --apply
```

O script popula `soc_ref`, `flash_ref`, `service_replacement_part` e `p7501_header_present` respeitando a trava hierárquica `T0 → T1` (nunca sobrescreve entidades já promovidas em `T2/T3/T4`).

### Estágio 5 — Análise Ghidra MN103S, Texto Plano & Mapas `ghidra_ssot_maps.json`

1. Consulte [`ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md`](ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) para não cair nas armadilhas do compilador Matsushita (PC-relative `cur_insn`, `setlb/lene`, saltos indiretos e divisão 64-bit).
2. Rode [`tools/ghidra/ghidra_zero_undef_pipeline.py`](../../tools/ghidra/ghidra_zero_undef_pipeline.py) seguido de [`tools/ghidra/ghidra_export_pseudocode_bundle.py`](../../tools/ghidra/ghidra_export_pseudocode_bundle.py) para gerar os arquivos texto plano segmentados por domínio (`boot_core`, `bgo_nfrom`, `sd_fat`, `main_hdd_tbc`, `davis_ide`) e o rascunho de endereços para [`tools/ghidra/ghidra_ssot_maps.json`](../../tools/ghidra/ghidra_ssot_maps.json).
3. Sincronize os geradores automáticos em Rust:

```bash
panadiga ghidra export-all --dir tools/ghidra
```

### Estágio 6 — Regra de Ouro SSOT Declarativa e Gate `T0..T4`

**PROIBIÇÃO ABSOLUTA DE NOMES DE PRODUTO EM LÓGICA EXECUTÁVEL:**
- Nunca coloque `match model { "DMR-EH55" => ... }`, constantes nomeadas por aparelho (`EH55_TRAIN_ID`) ou *offsets* mágicos soltos em `crates/panadiga-core`, `apps/desktop/src` ou `firmware/pico-p7501`.
- Toda característica de geometria BGO, tamanho de NVRAM, mapa `nor_layout`, blob de driver (`profiles/blobs/*.bin.gz`), tabela `firmware_train` e estratégia de patch pertence exclusivamente a `profiles/**/*.json` sob um **ID técnico declarativo de geometria/trem** (ex.: `BGO_45SEC_8MB_0X84130000`, `BGO_35SEC_4MB_0X84200000`).
- Ao alterar qualquer JSON de `profiles/`, regenere projeções e promova a evidência com:

```bash
panadiga profiles export-schema --out-dir profiles/schema
panadiga ssot export-all
panadiga profiles promote-evidence --entity-type model --id DMR-NOVO --tier T3 --sha256 <SHA256> --fixture <PATH>
panadiga profiles completeness
panadiga profiles validate
```

### Estágio 7 — Portão de Qualidade (`9/9`) e Pull Request

Execute a validação completa (`python scripts/validate.py`) garantindo `0` clones no `jscpd`, `100/100` no `uxlint`, tipagem estrita `mypy --strict`, testes Rust/Vitest/Playwright/C11 verdes e paridade bilíngue (`test_docs_parity.py`) antes de submeter seu PR conforme o checklist da **Seção 8 de [`CONTRIBUTING.md`](../../CONTRIBUTING.md)**.
