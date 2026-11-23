# Catálogo Completo MCP, Resources, Prompts & Arquitetura RAG

## 1. As 25 Tools Nativas MCP (`crates/panadiga-mcp/src/tools/`)

### Núcleo, Hardware, Forense & Conhecimento (`defs_core.rs` — 13 tools)
| Tool MCP | Propósito | Parâmetros Principais |
| :--- | :--- | :--- |
| `panadiga_memory_map` | Mapa físico oficial AM34/MN103S | `filter_name` |
| `panadiga_inspect_hdd` | Inspeção MEIHFS (MBR `'SH'`, espelho 28 MB, partições) | `image_path` |
| `panadiga_safety_check` | Valida tensão (<=3.3V LVTTL) e faixa de barramento | `voltage`, `address`, `confirm_upper_nor`, `flash_size_bytes` |
| `panadiga_clean_nvram` | Analisa, limpa (`clean`) ou patcheia CoW (`patch`) 64 KiB NVRAM (IPL bloqueado) | `input_file*`, `action`, `output_file`, `nvram_offset`, `patch_hex`, `model_id`, `lab_override_*` |
| `panadiga_search_manuals` | Busca nos 32 PDFs (nome + texto extraído via `lopdf`) | `query*`, `max_results` |
| `panadiga_rag_query` | Consulta o motor Zero-Error RAG de 6 camadas | `query*`, `include_corpus_stats` |
| `panadiga_inspect_frm` | Inspeciona `PANA_DVD.FRM` (TOC, módulos, entropia 8-byte) | `frm_path` |
| `panadiga_read_hex` | Leitura read-only hex+ASCII (16..4096 B) | `file_path*`, `offset`, `length` |
| `panadiga_pico_catalog` | Pinagem `P7501`, limiares elétricos, sequência erase, comandos Pico | `category` |
| `panadiga_ghidra_symbols` | Regiões AM34 SSOT e diretivas de símbolos Ghidra | `filter` |
| `panadiga_search_code` | Busca em código indexado Rust/C11/PIO/TS/Ghidra | `query*`, `scope=code|ghidra|docs|all`, `limit` |
| `panadiga_search_visuals` | Busca nas 238 colheitas visuais (`docs/assets/visual_manifest.json`) | `query`, `edition`, `category`, `page`, `limit` |
| `panadiga_project_status` | Topologia ao vivo (crates, PDFs, modelos, RAG stats) | `include_rag_stats` |

### RE, Unbrick, Capacidade HDD & Kit SD (`defs_re.rs` — 5 tools)
| Tool MCP | Propósito | Parâmetros Principais |
| :--- | :--- | :--- |
| `panadiga_hdd_capacity` | Inspeciona/patcheia CoW o teto `MaxProductLBA` no `MAIN` | `input_path*`, `action`, `mode`, `target_gb`, `output_path` |
| `panadiga_nor_dumper_stub` | Monta stub MN103S somente leitura `PANADMP1..PANADONE` | `start_addr`, `total_len`, `block_size`, `wrap_digadiag`, `load_addr`, `output_path` |
| `panadiga_bgo_unbrick` | Inspeciona/patcheia 3 sítios BGO do `PROG061` | `input_path*`, `action`, `mode`, `allow_unsafe_bgo_format`, `wrap_as_digadiag`, `output_path` |
| `panadiga_main_passthrough` | Inspeciona/patcheia TBC no `MAIN` ou gera plano RSP `$M` | `action`, `input_path`, `mode`, `output_path`, `callback_target_va` |
| `panadiga_sd_unbrick_kit` | Workbench unificado SD/FRM + emulador `Matshita R6-MV4` | `action` (`status|extract_frm|prepare_sd|verify_sd|matshita_emu`), `loader_profile`, `emu_action`, etc. |

### RE Avançado, BGO 45 Setores, DAVIS & CapabilityGate (`defs_re_ext.rs` — 7 tools)
| Tool MCP | Propósito | Parâmetros Principais |
| :--- | :--- | :--- |
| `panadiga_region_compat` | Inspeção/patch CoW `<deviceinfo>`, `DEV.BIN`/`ifodev2`, `.FRM` e TV | `action`, `input_path`, `output_path`, `model`, `oem`, `preset`, `tv_standard`, etc. |
| `panadiga_hdd_flash_inspect` | Planner estritamente dry-run de gravação HDD, lockouts & `mirror_main_lba` | `target_gb`, `allow_extended_4tb`, `disk_bytes`, `mock_disks` |
| `panadiga_tbc_appliance_plan` | Plano clean-room Diskless/TBC (`stage15_digadiag_tbc` / experimental) | `mode`, `tv_standard`, `experimental_ram_chainload` |
| `panadiga_nor_sector_audit` | Auditoria offline de todos os 142 setores Spansion (`SA0..SA141`) | `nor_path*` |
| `panadiga_eh55_bgo` | Audita BGO 45 setores (`SA26..=SA70`), gera plano CoW ou aplica live | `action*`, `nor_path`, `out_plan_path`, `port`, `backup_dir`, `confirm_upper_nor` |
| `panadiga_davis_probe` | Probe seguro `DAVIS CH0/CH1` e `0xEC IDENTIFY` (bloqueia `0xC000_100C`) | `action*` (`build_stubs|parse_ec|run_live`), `drive`, `port`, `input_path`, `ec_hex` |
| `panadiga_profiles_authorize` | Avaliador fail-closed `CapabilityGate T0..T4` | `model_id*`, `capability*`, `dump_size_bytes`, `jedec_*`, `wp_pin_deasserted`, `lab_override_*` |

## 2. Resources (`panadiga://`) & Prompts (`prompts.rs`)
- **Resources (teto `32 KiB` por leitura):**
  - `panadiga://invariants` · `panadiga://memory-map` · `panadiga://pinout-p7501-uart`
  - `panadiga://visual-catalog` · `panadiga://models-catalog` · `panadiga://pico-commands`
  - `panadiga://profiles/<MODEL_ID>` (89 modelos) · `panadiga://docs/<rel_path.md>` (até 250 docs, protegido contra traversal)
- **Prompts MCP:** `u99_nvram_triage`, `p7501_uart_diagnostics`, `cddiag_p7501_capture` *(use `pico capture-nor`, não o typo legado do texto do prompt)*, `frm_crypto_re`.

## 3. Arquitetura das 6 Camadas RAG (`crates/panadiga-rag/src/`)
1. `guardrails.rs` (`QueryGuard`): saneamento, expansão e classificação da consulta.
2. `indexer/` (`HierarchicalIndexer`) + `retriever.rs` (`HybridRetriever`): indexação BM25 + vetor em `markdown.rs`, `pdf.rs` (`lopdf`), `code.rs`, `visual.rs` com `ChunkSourceFilter` (`All|Documentation|PdfManuals|SourceCode|Ghidra`).
3. `critic.rs` (`RetrievalCritic`): filtragem de relevância e remoção de ruído.
4. `generator.rs` (`GroundedGenerator`): montagem ancorada exclusivamente em trechos recuperados.
5. `critic.rs` (`FactCritic`): verificação factual de citações antes de emitir resposta.
6. `safety/{mod.rs, patterns.rs}` (`HardwareSafetyValidator`): bloqueio determinístico contra instruções inseguras (tensão >3.3V, `NSW5V` push-pull, escrita em `SA0..SA10`, acesso a `0xC000_100C`).
