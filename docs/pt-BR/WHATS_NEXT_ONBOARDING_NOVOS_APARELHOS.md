> 🌐 **Idioma / Language:** [English (US)](../en-US/WHATS_NEXT_NEW_DEVICE_ONBOARDING.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Catálogo de Perfis DIGA](CATALOGO_E_AUDITORIA_PERFIS_HARDWARE_LINHA_DIGA.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# What's Next: Pipeline Completo de Onboarding de Novos Aparelhos DIGA (Bancada → SSOT JSON → Pull Request)

> **Objetivo:** Este guia padroniza o fluxo completo (*end-to-end*) para integrar qualquer novo modelo **Panasonic DIGA** ao ecossistema **Panadiga** usando arquitetura **100% declarativa (SSOT JSON)** — identificando cada placa pela tupla física **`(model, region, board_rev, flash_jedec_id)`**, eliminando código *hardcoded* por nome de modelo e impondo segurança elétrica (incluindo isolamento de SMPS) antes de qualquer escrita em Flash ou disco.

---

## Visão Geral dos 7 Estágios (`T0` → `T4`)

| Estágio | Objetivo Principal | Ferramentas / Scripts Canônicos | Critério de Saída |
| :--- | :--- | :--- | :--- |
| **1. Segurança Elétrica SMPS & `P7501`** | Isolamento galvânico AC/USB, descarga de capacitores, verificação `3.3V`/`1.8V`/`5V`, `GP3`/`NSW5V` em `Hi-Z`, `XRESET` Open-Drain | Multímetro + Transformador Isolador / Isolador USB `ADuM3160` + `panadiga pico status` | Log em `docs/debug_logs/` com zero overdrive e referência de terra segura |
| **2. Captura NOR (`2..16 MiB`) & Triagem** | Dump IC5001 somente-leitura, detecção automática de byte-swap 16-bit, espelhamento por pino flutuante e gerações `EEMUL` | `panadiga unbrick nor-stub`, `decode-nor-dump`, `panadiga profiles discover-nor`, `scripts/re/nor_intake_analyzer.py` | Dump SHA-256 íntegro (sem byte-swap/mirror) + mapa `nor_layout` rascunho |
| **3. `.FRM`, `panamon` & `rebuild-from-dump`** | Identificar `Loader` / `trust_model`, extrair `DRV`/`PROG`/`MAIN`/Blu-ray `fmaX`, bloquear mixagem não verificada e validar kits `PANA__SD.FRM` | `scripts/re/fleet_decryptor.py`, `scripts/re/frm_train_harvester.py --frm`, `panadiga frm unpack-all`, `panadiga unbrick rebuild-from-dump` | Hashes `EXTRHEAD`/`Adler32`, builds `MAIN`, `trust_model` e versão IPL validados |
| **4. Quarentena via Manuais PDF (`T1`)** | Extrair códigos PCB (`VEP`/`REP`), SoC, Flash e EEPROM com citação de página em fila humana (**nunca direto** em `profiles/*.json`) | `scripts/re/manual_spec_extractor.py --apply` → `profiles/staging/manual_proposals/` | Propostas auditadas por humano com página citada antes do merge `T1` |
| **5. Orquestração Ghidra & Cobertura Alcançável** | Resolver Armadilhas MN103S / SPARC / ARM / AArch64 em seções alcançáveis, mantendo *literal pools*/tabelas tipados como dados | `scripts/re/ghidra_fleet_orchestrator.py`, `tools/ghidra/ghidra_zero_undef_pipeline.py`, `panadiga ghidra export-all` | Pacotes C/texto plano em `docs/pseudocode/` + `fleet_verify.json` |
| **6. SSOT Declarativo, Allowlist & Gate `< T3`** | Modelar tupla `(model, region, board_rev, flash_jedec_id)` e allowlist positiva BGO; bloquear perfis `< T3` em runtime | `profiles/*.json`, `panadiga ssot export-all`, `panadiga profiles promote-evidence`, `panadiga profiles completeness` | `panadiga profiles validate` com `0` violações + prova matemática de allowlist BGO |
| **7. Emulação `Mn103Vm`, Testes & Pull Request** | Validar fluxo ISA no `Mn103Vm` (sem substituir prova física de IRQ/WDT/cache), E2E Playwright, contratos IPC e 0% clones `jscpd` | `python scripts/validate.py` (`9/9` gates) | Todos os 9 Quality Gates verdes + checklist em [`CONTRIBUTING.md`](../../CONTRIBUTING.md) |

---

## Passo a Passo Detalhado na Bancada

### Estágio 1 — Checklist Elétrico Obrigatório (SMPS / `P7501` / `IC5001` / `CH1`)

> [!CAUTION]
> **Risco de Choque Elétrico & Ground Loop em Fontes Chaveadas (SMPS):** Gravadores DVD/HDD Panasonic possuem chassi e trilhas de secundário frequentemente acoplados por **capacitores Y (~60–110 VAC em circuito aberto contra o terra AC)**. Ligar o PC/notebook via USB-UART direto sem isolamento galvânico fecha um loop de terra capaz de queimar a porta USB, o SoC `AM34` ou causar choque no operador.

Antes de conectar qualquer fio, sonda ou cabo USB:
1. **Isolamento Galvânico & Descarga Segura:**
   - Alimente a placa sob bancada por um **transformador isolador AC/AC** *ou* interponha um **isolador USB galvânico** (`ADuM3160` / `ADuM4160`) entre o computador e o Raspberry Pi Pico.
   - Com o cabo AC desconectado, confirme com multímetro DC que os capacitores do primário (**~170–340 VDC**) e das linhas secundárias estão totalmente descarregados antes de soldar fios ou encaixar garras/soquetes.
2. **Identidade Elétrica & Níveis Lógicos:**
   - Confirme no esquemático do manual (`VEP79138`, `VEP79106A`, etc.) e meça com multímetro a tensão nominal dos barramentos lógicos (**`3.3V`**, **`1.8V`** ou **`5V`**) no conector **`P7501` (`TO JIGU`)** e nos pinos da Flash antes de ligar o Pico. Nunca assuma `3.3V` sem medir um modelo novo.
3. **Invariantes da Sonda Pico (`RP2040` / `RP2350`):**
   - Verifique que **`GP3` (`NSW5V` / Pino 5)** permanece **SEMPRE como `Hi-Z Input` sem pulls**, **`GP5` (`XRESET` / Pino 2)** opera em **Open-Drain** e a sonda **NUNCA é alimentada pelo `P7501`** (use USB isolado ou `VSYS`).
4. **Inspeção de Storage (`CH1`):**
   - Em falhas de boot HDD (`UPD NG`, ausência de `MAIN` ou `0xE8000000` em `DAVIS CH1`), meça `P1503 Pino 4 (DR5V = +5.1V)`, `P1503 Pino 1 (DR12V = +12.3V)` e confirme polaridade FFC (`Type A` vs `Type B`) antes de atribuir o erro ao firmware.

### Estágio 2 — Captura Física da NOR Flash e Triagem de Integridade

```bash
# 1. Gere o stub somente leitura para UART (ou use PANA__SD.FRM para entrar em PANAMON1>)
panadiga unbrick nor-stub --start 0x80000000 --len 0x800000 --out tmp/nor_stub.bin --wrap-digadiag

# 2. Após capturar PANADMP1..PANADONE em docs/debug_logs/, decodifique o binário imutável:
panadiga unbrick decode-nor-dump -i docs/debug_logs/captura_nova.log -o tmp/dmr_novo_8mb.bin

# 3. Extraia mapa preliminar e valide integridade (rejeita byte-swap 16-bit, espelhamento por pino flutuante e all-0xFF):
panadiga profiles discover-nor --input tmp/dmr_novo_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles --json tmp/nor_report.json
# Análise Python offline com detecção de múltiplos EEMUL e headers EXTRHEAD inválidos/stale:
python -m scripts.re.nor_intake_analyzer --offline-only --input tmp/dmr_novo_8mb.bin --model-id DMR-NOVO --out-dir tmp/draft_profiles
```

> **Invariante Forense & Identidade Física:** Nunca edite dumps originais (*in-place*). Confirme o Flash JEDEC ID real via autoselect CFI antes de gerar payloads de escrita; nunca identifique hardware apenas pelo nome comercial (`--model`): registre sempre a tupla completa **`(model, region, board_rev, flash_jedec_id)`**.

### Estágio 3 — Versão de `DIGADIAG`, `trust_model` & `rebuild-from-dump`

Cada trem de firmware declara `digadiag_spec.trust_model` (`checksum32_only`, `encrypted_toc`, `rsa_signed`, `unknown`) além do formato de boot:
- **`Loader ver0.61` (`06/03/06`, `SDDIAG` — `checksum32_only`)**: consome `DIGADIAG.FRM` em **raw `256 KiB`** + `PANA__SD.FRM` com TOC de **`48 bytes` (`sd48`)** em mídia SDSC `<= 2 GiB` FAT16. Perfis verificados (`>= T3`) passam pelo gate padrão; perfis experimentais (`< T3`, como `ver063` em `T2`) exigem `--allow-unverified-profile`.
- **`Loader ver0.63+` (`bootbin` — `encrypted_toc`)**: consome `DIGADIAG.FRM` **`128 KiB EXTRHEAD`** + `PANA__SD.FRM` de **`64 bytes` (`sd64`)**.

```bash
# Reconstruir DIGADIAG.FRM + PANA__SD.FRM diretamente de dump NOR íntegro + Partição 1 MAIN:
panadiga unbrick rebuild-from-dump \
  --model DMR-EH55 \
  --nor-dump tmp/dmr_novo_8mb.bin \
  --hdd-main tmp/partition_1_am34_os.bin \
  --target-gb 250 \
  --out-dir tmp/sd_kit_rebuild \
  --json tmp/rebuild_report.json
```

O comando verifica integridade da NOR (bloqueando dumps com byte-swap ou espelhamento), consistência de versão IPL vs `firmware_train`, integridade checksum de `DRV` e identidade `MAIN` antes de empacotar (use `--allow-unverified-mix` somente em laboratório para combinações cross-build). Para protótipos experimentais em SDRAM (`DigaOS`), veja [`firmware/digaos/README.md`](../../firmware/digaos/README.md) (aviso: protótipo de interoperabilidade, **NOT LEGALLY CLEAN-ROOM**).

### Estágio 4 — Quarentena de Especificações via Manuais PDF (`T1`)

Adicione o PDF de serviço em `manuals_and_schematics/` e execute:

```bash
python -m scripts.re.manual_spec_extractor --apply
```

Por segurança (**Regra anti-envenenamento do SSOT**), o extrator **NUNCA modifica diretamente** `profiles/boards/*.json` nem `profiles/eeprom_chips/*.json`. Ele gera propostas em quarentena sob **`profiles/staging/manual_proposals/*.proposal.json`** com `"requires_human_review": true` e **citações exatas de página (`page_citations`)** para que um engenheiro valide região, revisão de PCB e peça substituta antes da promoção manual a `T1`.

### Estágio 5 — Análise Ghidra MN103S sobre Seções Executáveis Alcançáveis

1. Consulte [`ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md`](ARMADILHAS_MN103S_E_RESOLUCAO_GHIDRA.md) para resolver as Armadilhas MN103S (`cur_insn`, `setlb/lene`, saltos indiretos e divisão 64-bit).
2. Meça cobertura **exclusivamente sobre seções executáveis delimitadas e funções alcançáveis** a partir de vetores e ponteiros verificados — **nunca persiga `0 ??` (*Zero-Undef*) cegamente**, pois *literal pools*, tabelas de salto (*jump tables*), strings, tabelas TBC e padding `0xFF` são dados legítimos; forçar disassembly sobre dados gera *xrefs* falsos para MMIO.
3. Decripte e deduplique contêineres `.FRM`/`.zip`/`.7z`/`.exe` via [`scripts/re/fleet_decryptor.py`](../../scripts/re/fleet_decryptor.py), colete telemetria com `python -m scripts.re.frm_train_harvester --frm <PATH> --unpack-missing`, orquestre o Ghidra via SSOT em [`scripts/re/ghidra_fleet_orchestrator.py`](../../scripts/re/ghidra_fleet_orchestrator.py), exporte bundles via [`tools/ghidra/ghidra_export_pseudocode_bundle.py`](../../tools/ghidra/ghidra_export_pseudocode_bundle.py) e sincronize geradores com `panadiga ghidra export-all --dir tools/ghidra` (guia completo em [`scripts/re/README.md`](../../scripts/re/README.md)).

### Estágio 6 — Regra de Ouro SSOT, Allowlist Matemática BGO & Gate `< T3`

- **Tupla Operacional Obrigatória:** Identifique cada configuração física por `(model, region, board_rev, flash_jedec_id)` e zero *hardcode* de modelo em Rust/TS/C.
- **Invariante Positivo BGO (Allowlist):** Toda escrita/retarget BGO é verificada por inclusão estrita `[start..=end] ⊆ nor_layout.regions[kind == bgo]`, alinhamento aos setores físicos de `flash_ref` e interseção rigorosamente vazia (`∩ = ∅`) com `ipl`, `drv`/`extrhead`, `nor_fs` e `eemul`.
- **Gate Runtime `< T3`:** `prepare_sd_unbrick_kit` e `retarget` recusam perfis com tier `< T3` (`physical_dump_verified`) sem `--allow-unverified-profile`.

```bash
panadiga profiles export-schema --out-dir profiles/schema
panadiga ssot export-all
panadiga profiles promote-evidence --entity-type model --id DMR-NOVO --tier T3 --sha256 <SHA256> --fixture <PATH>
panadiga profiles completeness
panadiga profiles validate
```

### Estágio 7 — Limites do `Mn103Vm`, Portão (`9/9`) e Pull Request

O emulador `Mn103Vm` valida apenas fluxo de controle ISA (`MN103S`) e fronteiras estáticas de memória — **não aprova sozinho** temporização real de barramento/CFI Spansion, mascaramento atômico de IRQ (`CLI`), coerência I/D-cache ou watchdog `IC7501`. Sempre valide em hardware real isolado antes de promover a `T4`, e execute `python scripts/validate.py` (`9/9` Quality Gates verdes, `0` clones `jscpd`, `100/100` `uxlint`, `mypy --strict`, testes Rust/Vitest/Playwright/C11 e paridade bilíngue `test_docs_parity.py`) conforme a **Seção 8 de [`CONTRIBUTING.md`](../../CONTRIBUTING.md)**.

---

## Pesquisa Futura & Próximos Portões Pós-`EV-20261124-004`

Com o bloqueio `S11` encerrado em silício real e o mapa completo `BGO` (`p1_sa=41`, `45/45` saudáveis) + `SDCD_CFG4` (`0x000040B8` `4BIT`) verificados em [`com10_round5_live_unbrick_sd_trace_20261124.log`](../evidence/com10_round5_live_unbrick_sd_trace_20261124.log), as frentes imediatas de evolução são:

1. **NorAutoboot `SA11..SA14` Opt-In (`--nor-autoboot-capsule`, Boot Sem SD via `Nor-FROM`):** `NOR_MAP` e `BGO_INSPECT NORFROM` confirmaram `SA11 0x84040000 = ERASED_FF` (`norfrom_used=0`), explicando por que o `MAIN` ainda depende do SD inserido para carregar o hook pré-FVU. Quando houver autorização explícita do operador, gravar `SA11..SA14` (protegendo `SA0..SA10` com `E05`) via `--sata-hdd-autoboot --nor-autoboot-capsule` + `--confirm-upper-nor` com backup SHA-256 completo.
2. **Bring-Up AO VIVO do `DigaOS` (`M1 -> M3`) em SDRAM:** empacotar `firmware/digaos` (`digaos_ssot.h`, `digaos_shell_poll_once()`, comandos `sd`/`bgo`/`ssot`, `DIGAOS>`) como `DIGADIAG.FRM` (`262.144 B` `@ 0x5040_0000`) e validar boot e keepalive `IC7501` `255s` em bancada sem tocar em `SA0..SA141`.
3. **Parser Completo `Upper EEMUL` (`SA71..SA133` `"1284"`):** decodificar os `7` registros ativos e `11` superseded identificados pelo `NOR_MAP` para auditoria fina de calibração/opções além de `SA134`.
4. **`PANAEUSB.FRM` & Expansão NAND/eMMC (`Schema v2.1`):** estender extração/desofuscação USB e modelagem SSOT de armazenamento não-NOR nos perfis de modelos `T1 -> T4`.
