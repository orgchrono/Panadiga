> 🌐 **Language / Idioma:** **Bilingual (PT-BR / EN-US)**
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Pseudocode Index](./INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Dossiê Integral EH55 (`26.555` linhas)](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) | [`??` Audit](./GHIDRA_UNDEF_AUDIT_REPORT.md) | [`fleet_verify.json`](../audits/zero_undef/fleet_verify.json)

# Evidência Ghidra Headless — Frota Completa Panasonic DIGA (`100% Zero-??` MN103S + UniPhier Linux + SPARC V8 + Acervo STB) / Full-Fleet Headless Ghidra Pseudocode & Verification Dossier

> [!NOTE]
> **PT-BR:** Este dossiê consolida a decriptação em texto plano de **40 fontes originais** (`78` binários únicos por SHA-256 em `C:\ghidra\staging\decrypted_fleet\`), seguida da execução em escala completa do pipeline **Ghidra Headless de 4 passes** (`ghidra_zero_undef_pipeline.py` + SLEIGH patcheado `am33_panadiga_zero_undef.patch` + auditoria Java `PanadigaMn103sTrapAudit.java --scope=exec` + `ghidra_export_pseudocode_bundle.py`) sobre projetos isolados `Panadiga_MN103S_ZeroUndef` e `Panadiga_MN103S_Fleet_*` (preservando `C:\ghidra\projects\Panadiga_MN103S` e os binários originais estritamente intactos). Todos os **`53 / 53` programas MN103S analisados** atingiram **`0` bytes indefinidos (`??`)**, **`0` chamadas `mn_todo()` em funções** e **`0` bookmarks de erro**.
>
> **EN-US:** This dossier documents the complete plaintext decryption of **40 original firmware sources** (`78` SHA-256 unique binaries staged in `C:\ghidra\staging\decrypted_fleet\`) and full **Ghidra Headless 4-pass** analysis over isolated projects (`Panadiga_MN103S_ZeroUndef` + `Panadiga_MN103S_Fleet_*`), keeping `C:\ghidra\projects\Panadiga_MN103S` and raw firmware files untouched. All **`53 / 53` analyzed MN103S programs** achieved **`0` undefined (`??`) bytes**, **`0` `mn_todo()` calls inside functions**, and **`0` Error bookmarks**, alongside full SPARC V8 analysis for `DMR-EZ485V` and forensic plaintext extraction for `STB` ST20/OS20/MStar modules and UniPhier NAND filesystems.

---

## 1. Sumário Executivo & Totais Globais Verificados (`fleet_verify.json`)

| Métrica / Metric | Valor Verificado | Fonte / Observação |
| :--- | ---: | :--- |
| **Fontes originais decriptadas** | **40** | Todos `.FRM` (Feistel/SD), dumps NOR (`EH55`, `ES35V`, `EZ485V`), `DVDRAM.7z` (`EH58`) |
| **Binários únicos por SHA-256** | **78** | Deduplicação integral em `C:\ghidra\staging\decrypted_fleet\unique_plaintext\` |
| **Programas executáveis analisados no Ghidra** | **55** | `6` núcleo (`core`) + `25` shard `small` + `24` `MAIN` (`main_a`/`main_b`/`main_c`) |
| **Programas MN103S em `Zero-Undef` (`0` / `0` / `0`)** | **53 / 53 (100%)** | `undefined_code_bytes == 0`, `mn_todo_in_functions == 0`, `bad_instruction_bookmarks == 0` |
| **Instruções totais decodificadas** | **75.031.001** | Soma exata reportada em [`fleet_verify.json`](../audits/zero_undef/fleet_verify.json) |
| **Funções descobertas no Ghidra** | **1.083.676** | Todas indexadas em texto plano no repositório (`docs/pseudocode/raw/fleet_*.txt`) |
| **Funções decompiladas em pseudocódigo C** | **1.083.618 (99,995%)** | Dumps integrais em C localizados em `C:\ghidra\staging\decrypted_fleet\pseudocode_full\*.c` |
| **Capturas brutas rastreadas no repositório (`./raw/fleet_*.txt`)** | **9 arquivos (`24,91 MB`)** | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) · [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) · [`fleet_drv_families.txt`](./raw/fleet_drv_families.txt) · [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) · [`fleet_main_function_maps_part1.txt`](./raw/fleet_main_function_maps_part1.txt) / [`part2`](./raw/fleet_main_function_maps_part2.txt) / [`part3`](./raw/fleet_main_function_maps_part3.txt) · [`fleet_sparc_ez485v.txt`](./raw/fleet_sparc_ez485v.txt) · [`fleet_stb_and_non_mn103_inventory.txt`](./raw/fleet_stb_and_non_mn103_inventory.txt) |

---

## 2. Inventário Completo dos 55 Programas Analisados no Ghidra Headless

Notação da coluna **Gate (`U`/`T`/`E`)**: `undefined_code_bytes` / `mn_todo_in_functions` / `bad_instruction_bookmarks`.

| # | Programa Ghidra | Modelo | Estágio | ISA (`Language ID`) | Base VA | Tamanho (B) | Instruções | Funções | C Decompilado | Gate (`U`/`T`/`E`) | SHA-256 (prefixo) | Bundle Raw |
|---:|---|---|---|---|---|---:|---:|---:|---:|---|---|---|
| 1 | `BOOT.BIN` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x44000000` | 262,144 | 76,337 | 1,018 | 1,018 | `0` / `0` / `0` | `7c13c72a54c77387...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 2 | `eh55_loader_ver061_256k.bin` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x44000000` | 262,144 | 70,058 | 487 | 487 | `0` / `0` / `0` | `9f5c448d8879106b...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 3 | `eh55_frm_PROG061_BOOT.bin` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 75,550 | 888 | 888 | `0` / `0` / `0` | `c7da9121174fcf4c...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 4 | `DRV_decomp.bin` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x40000000` | 1,441,792 | 338,305 | 2,870 | 2,870 | `0` / `0` / `0` | `694a4ab8aa29fcc4...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 5 | `EH55_DRV_S211.bin` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x40000000` | 1,441,792 | 353,745 | 1,454 | 1,446 | `0` / `0` / `0` | `6f2b61ede80ea9e3...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 6 | `EH55_MAIN_3410PQ.bin` | DMR-EH55 | `CORE` | `mn10300:LE:32:default` | `0x57000000` | 13,003,052 | 2,712,328 | 38,317 | 38,317 | `0` / `0` / `0` | `c3275d39549b68f4...` | [`EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md`](./EVIDENCIA_PSEUDOCODIGO_GHIDRA_MN103S.md) |
| 7 | `DMR-BS850_MAIN3600_kernel_45070b4f.bin` | DMR-BS850 | `MAIN_LINUX_KERNEL` | `mn10300:LE:32:default` | `0x90100000` | 4,194,304 | 1,089,933 | 11,171 | 11,171 | `0` / `0` / `0` | `45070b4f5a79fe8c...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 8 | `DMR-BS850_PROG0.00_c202674a.bin` | DMR-BS850 | `PROG` | `mn10300:LE:32:default` | `0x50800000` | 163,840 | 36,177 | 427 | 427 | `0` / `0` / `0` | `c202674a65d1c2d9...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 9 | `DMR-BS880_MAIN3410_kernel_02111625.bin` | DMR-BS880 | `MAIN_LINUX_KERNEL` | `mn10300:LE:32:default` | `0x90100000` | 4,194,304 | 1,109,081 | 11,317 | 11,316 | `0` / `0` / `0` | `021116252114aa9f...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 10 | `DMR-BS880_PROG0.00_a2630208.bin` | DMR-BS880 | `PROG` | `mn10300:LE:32:default` | `0x50800000` | 163,840 | 41,221 | 480 | 480 | `0` / `0` / `0` | `a26302081cda36d5...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 11 | `DMR-BS885_MAIN3400_kernel_ec8169b9.bin` | DMR-BS885 | `MAIN_LINUX_KERNEL` | `mn10300:LE:32:default` | `0x90100000` | 4,325,376 | 1,120,139 | 11,651 | 11,649 | `0` / `0` / `0` | `ec8169b952b3180c...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 12 | `DMR-BS885_PROG0.00_6a81ad16.bin` | DMR-BS885 | `PROG` | `mn10300:LE:32:default` | `0x50800000` | 163,840 | 43,121 | 476 | 476 | `0` / `0` / `0` | `6a81ad1624d67d01...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 13 | `DMR-EH49_DRV_L104_d7555eb0.bin` | DMR-EH49 | `DRV` | `mn10300:LE:32:default` | `0x40000000` | 1,441,792 | 357,923 | 1,400 | 1,394 | `0` / `0` / `0` | `d7555eb0d182f645...` | [`fleet_drv_families.txt`](./raw/fleet_drv_families.txt) |
| 14 | `DMR-EH49_DRV_S514_583c3df1.bin` | DMR-EH49 | `DRV` | `mn10300:LE:32:default` | `0x40000000` | 1,441,792 | 358,627 | 1,410 | 1,405 | `0` / `0` / `0` | `583c3df11cbfd893...` | [`fleet_drv_families.txt`](./raw/fleet_drv_families.txt) |
| 15 | `DMR-EH49_PROG071_3bf48f80.bin` | DMR-EH49 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 74,031 | 553 | 553 | `0` / `0` / `0` | `3bf48f806898643a...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 16 | `DMR-EH50_PROG019_9327803f.bin` | DMR-EH50 | `PROG` | `mn10300:LE:32:default` | `0x50030400` | 196,608 | 51,956 | 353 | 353 | `0` / `0` / `0` | `9327803f9ee4c1dd...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 17 | `DMR-EH55_DIGADIAG.FRM_305cc2ee.bin` | DMR-EH55 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 76,064 | 531 | 531 | `0` / `0` / `0` | `305cc2eebc09bbaa...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 18 | `DMR-EH55_PROG061_6351dc26.bin` | DMR-EH55 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 75,554 | 531 | 531 | `0` / `0` / `0` | `6351dc26d479f45a...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 19 | `DMR-EH55_PROG061_668f1df3.bin` | DMR-EH55 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 75,654 | 533 | 533 | `0` / `0` / `0` | `668f1df3291ffe11...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 20 | `DMR-ES35V_ES35V_IPL_ver018.bin_c488403d.bin` | DMR-ES35V | `IPL` | `mn10300:LE:32:default` | `0x44000000` | 262,144 | 69,404 | 490 | 490 | `0` / `0` / `0` | `c488403d44628da8...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 21 | `DMR-ES35V_embedded_DRV_2_f6655390.bin` | DMR-ES35V | `DRV` | `mn10300:LE:32:default` | `0x40000000` | 1,441,792 | 354,485 | 1,465 | 1,454 | `0` / `0` / `0` | `f6655390b3769bf2...` | [`fleet_drv_families.txt`](./raw/fleet_drv_families.txt) |
| 22 | `DMR-EX773_PROG201_c8ad2b70.bin` | DMR-EX773 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 74,268 | 553 | 553 | `0` / `0` / `0` | `c8ad2b70737a74d0...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 23 | `DMR-EX78_PROG061_ac7571e3.bin` | DMR-EX78 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 72,021 | 548 | 548 | `0` / `0` / `0` | `ac7571e316e4cbb0...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 24 | `DMR-EX80S_PROG062_01e8afdb.bin` | DMR-EX80S | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 75,716 | 532 | 532 | `0` / `0` / `0` | `01e8afdbe4db8c46...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 25 | `DMR-EX84C_PROG205_ae0d199b.bin` | DMR-EX84C | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 74,302 | 554 | 554 | `0` / `0` / `0` | `ae0d199b514df470...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 26 | `DMR-EX86_PROG202_7ab95990.bin` | DMR-EX86 | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 70,764 | 525 | 525 | `0` / `0` / `0` | `7ab959907d7c3d10...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 27 | `DMR-EZ485V_embedded_lzma_0x007d88_36f71648.bin` | DMR-EZ485V | `VXWORKS_BAL` | `sparc:BE:32:default` | `0x40000000` | 629,112 | 102,896 | 2,418 | 2,418 | `0` / `0` / `0` | `36f7164899242e48...` | [`fleet_sparc_ez485v.txt`](./raw/fleet_sparc_ez485v.txt) |
| 28 | `DMR-EZ485V_panasonic_dmr_ez485v_flash_dump_4mb.bin_6ebb31f2.bin` | DMR-EZ485V | `NOR_FULL` | `sparc:BE:32:default` | `0x00000000` | 4,194,304 | 3,419 | 29 | 29 | `0` / `0` / `1` | `6ebb31f21197e911...` | [`fleet_sparc_ez485v.txt`](./raw/fleet_sparc_ez485v.txt) |
| 29 | `DMR-EZ49V_PROG201_43efd302.bin` | DMR-EZ49V | `PROG` | `mn10300:LE:32:default` | `0x50400000` | 262,144 | 70,726 | 525 | 525 | `0` / `0` / `0` | `43efd30266675a2b...` | [`fleet_ipl_and_progs.txt`](./raw/fleet_ipl_and_progs.txt) |
| 30 | `DMR-XW300_MAIN3700_kernel_f3f923dc.bin` | DMR-XW300 | `MAIN_LINUX_KERNEL` | `mn10300:LE:32:default` | `0x90100000` | 4,194,304 | 1,063,204 | 10,717 | 10,715 | `0` / `0` / `0` | `f3f923dc16848824...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 31 | `DMR-XW300_PROG0.00_48e71fb5.bin` | DMR-XW300 | `PROG` | `mn10300:LE:32:default` | `0x50800000` | 262,144 | 32,972 | 423 | 423 | `0` / `0` / `0` | `48e71fb54b8a521c...` | [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt) |
| 32 | `DMR-EH49_MAIN3100YA_594b6c5a.bin` | DMR-EH49 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,653,820 | 2,964,979 | 43,294 | 43,290 | `0` / `0` / `0` | `594b6c5aa5872bb4...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 33 | `DMR-EH50_MAIN3360NA_662d5e71.bin` | DMR-EH50 | `MAIN` | `mn10300:LE:32:default` | `0x57000400` | 10,678,476 | 2,091,074 | 31,212 | 31,208 | `0` / `0` / `0` | `662d5e71787ec3e6...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 34 | `DMR-EH55_MAIN3410PQ_f5502f29.bin` | DMR-EH55 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 13,003,052 | 2,711,239 | 41,028 | 41,028 | `0` / `0` / `0` | `f5502f2912d49ca3...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 35 | `DMR-EX72S_MAIN3010YK_42f79ba9.bin` | DMR-EX72S | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,051,500 | 2,835,669 | 42,223 | 42,223 | `0` / `0` / `0` | `42f79ba975f6bc82...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 36 | `DMR-EX78_MAIN3410MS_b8d87e6b.bin` | DMR-EX78 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 12,418,636 | 2,696,979 | 39,878 | 39,877 | `0` / `0` / `0` | `b8d87e6bec9ccef2...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 37 | `DMR-EX80S_MAIN3020PZ_ab5609b5.bin` | DMR-EX80S | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 12,138,476 | 2,519,967 | 38,606 | 38,606 | `0` / `0` / `0` | `ab5609b5f59438c3...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 38 | `DMR-EX84C_MAIN3030YN_baf40d78.bin` | DMR-EX84C | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 14,299,116 | 3,079,645 | 45,036 | 45,035 | `0` / `0` / `0` | `baf40d787e82a043...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 39 | `DMR-EX93_MAIN3240YM_19340bee.bin` | DMR-EX93 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 14,906,028 | 3,249,345 | 47,820 | 47,820 | `0` / `0` / `0` | `19340bee7f59f702...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 40 | `DMR-EH49_MAIN3100YS_a7182537.bin` | DMR-EH49 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,886,636 | 3,016,419 | 44,088 | 44,084 | `0` / `0` / `0` | `a7182537e8859b83...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 41 | `DMR-EH55_MAIN3060NP_075f552e.bin` | DMR-EH55 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 13,427,852 | 2,577,016 | 38,188 | 38,188 | `0` / `0` / `0` | `075f552ed583ba18...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 42 | `DMR-EH58_MAIN.BIN_a7009bcf.bin` | DMR-EH58 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 13,131,164 | 2,840,982 | 41,488 | 41,488 | `0` / `0` / `0` | `a7009bcfd97ea386...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 43 | `DMR-EX769_MAIN3400YX_c4f0895b.bin` | DMR-EX769 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 11,382,156 | 2,370,706 | 35,924 | 35,924 | `0` / `0` / `0` | `c4f0895b6f9d9920...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 44 | `DMR-EX79_MAIN3400YH_8e9fe910.bin` | DMR-EX79 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,130,204 | 2,857,834 | 42,338 | 42,338 | `0` / `0` / `0` | `8e9fe910b6761784...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 45 | `DMR-EX81S_MAIN3410EE_37828336.bin` | DMR-EX81S | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 12,409,484 | 2,694,604 | 39,889 | 39,889 | `0` / `0` / `0` | `378283368b8360ef...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 46 | `DMR-EX88_MAIN3500MM_745eda64.bin` | DMR-EX88 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 12,567,596 | 2,734,876 | 40,367 | 40,366 | `0` / `0` / `0` | `745eda64f2159684...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 47 | `DMR-EZ49V_MAIN3100Y1_4e61e3fa.bin` | DMR-EZ49V | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 10,224,316 | 2,222,710 | 37,404 | 37,403 | `0` / `0` / `0` | `4e61e3fa28c08eb6...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 48 | `DMR-EH49_MAIN3200YL_9d8a4bca.bin` | DMR-EH49 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,773,740 | 2,996,928 | 43,675 | 43,675 | `0` / `0` / `0` | `9d8a4bca9209bbfe...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 49 | `DMR-EH55_MAIN3060NP_d9cb1d93.bin` | DMR-EH55 | `MAIN` | `mn10300:LE:32:default` | `0x57000000` | 13,427,852 | 2,577,016 | 38,188 | 38,188 | `0` / `0` / `0` | `d9cb1d9376143bce...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 50 | `DMR-ES35V_embedded_MAIN_4_841a5419.bin` | DMR-ES35V | `MAIN` | `mn10300:LE:32:default` | `0x52DF6000` | 6,739,752 | 1,847,808 | 20,416 | 20,413 | `0` / `0` / `0` | `841a5419506e995a...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 51 | `DMR-EX773_MAIN3200XC_205e56ec.bin` | DMR-EX773 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 12,973,564 | 2,796,936 | 41,522 | 41,518 | `0` / `0` / `0` | `205e56ec2c7ca101...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 52 | `DMR-EX79_MAIN3400YV_5e3260a2.bin` | DMR-EX79 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,120,428 | 2,817,084 | 58,788 | 58,788 | `0` / `0` / `0` | `5e3260a27c134bb4...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 53 | `DMR-EX83_MAIN3220YE_36fe8aa9.bin` | DMR-EX83 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 14,790,972 | 3,227,944 | 47,568 | 47,568 | `0` / `0` / `0` | `36fe8aa9c0cb7fc6...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 54 | `DMR-EX89_MAIN3400YW_67c62707.bin` | DMR-EX89 | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 13,227,804 | 2,879,947 | 42,683 | 42,683 | `0` / `0` / `0` | `67c627072fd221cc...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |
| 55 | `DMR-EZ49V_MAIN3200Y1_36eb83ae.bin` | DMR-EZ49V | `MAIN` | `mn10300:LE:32:default` | `0x50622000` | 10,224,572 | 2,223,313 | 37,407 | 37,407 | `0` / `0` / `0` | `36eb83ae61104564...` | [`fleet_main_subsystems.txt`](./raw/fleet_main_subsystems.txt) |

> *(Todos os mapas integrais de endereços das `1.083.676` funções dos estágios `MAIN` também estão distribuídos e pesquisáveis em [`fleet_main_function_maps_part1.txt`](./raw/fleet_main_function_maps_part1.txt), [`fleet_main_function_maps_part2.txt`](./raw/fleet_main_function_maps_part2.txt) e [`fleet_main_function_maps_part3.txt`](./raw/fleet_main_function_maps_part3.txt); dumps C individuais de 100% de cada binário residem em `C:\ghidra\staging\decrypted_fleet\pseudocode_full\<programa>.c`.)*

---

## 3. Correspondência de Âncoras Cross-Model (`PROG`, `DRV`, `MAIN` `_dev_SERIAL` & UniPhier)

### 3.1 Tabela CLI / Comando `PASS` (`MATS`/`HITA`) em Drives Ópticos (`DRV`)

| Programa `DRV` | Base VA | CLI `<8sIII>` VA (`cmds`) | Função `PASS` (`0x4d415453`/`0x48495441`) | Flag de Desbloqueio |
|---|---:|---|---:|---|
| `core/DRV_decomp.bin` (`S415` / `EH58`) | `0x40000000` | `0x400DF5C0` (`114`) | `drv_cmd_pass_unlock_decomp @ 0x4013420A` | `DAT_00000cd8 \|= 1` |
| `core/EH55_DRV_S211.bin` | `0x40000000` | `0x400D92A4` (`111`) | `FUN_40130bc0 @ 0x40130BC0` *(site `0x40130efa`)* | `BSET 0x1,(0xd78)` |
| `DMR-ES35V_embedded_DRV_2` | `0x40000000` | `0x400DA174` (`113`) | `FUN_4012e448 @ 0x4012E448` | Flag interna R/W SRAM |
| `DMR-EH49_DRV_S514` | `0x40000000` | `0x400E04A0` (`114`) | `FUN_40139ba0 @ 0x40139BA0` *(+ `FUN_40139eda @ 0x40139EDA`)* | `bRam00000cd8 \|= 1` |
| `DMR-EH49_DRV_L104` | `0x40000000` | `0x400E0758` (`114`) | `FUN_40139ca4 @ 0x40139CA4` | Flag interna R/W SRAM |

### 3.2 `PANAMON1` Slack & `BGO` nos Estágios `PROG`

| Estágio `PROG` | Base VA | Entrypoint Real | Maior Slack `0xFF` (`PANAMON1`) | Geometria BGO / S-Box |
|---|---:|---:|---:|---|
| `DMR-EH50_PROG019` | `0x50030400` *(corrigido de `0x50030000`)* | `0x50030400` | `0x500583A8` | S-Box `1` ocorrência verificada |
| `DMR-EH55_DIGADIAG.FRM` | `0x50400000` | `0x50400000` | `0x50439B78` | BGO `0x50436E30` (`0x84130000`) |
| `DMR-EX80S_PROG062` | `0x50400000` | `0x50400000` | `0x50439534` | S-Box `1` ocorrência |
| `DMR-EH49_PROG071` (`EXTRHEADMINI`) | `0x50400000` | `0x50400011` *(header 17B `00 f2 f3 fe`)* | `0x5043AF24` | `[nfrom]` File map `0x5040168C` |
| `DMR-EX773_PROG201` (`EXTRHEADMINI`) | `0x50400000` | `0x50400011` | `0x5043B3F4` | S-Box `1` ocorrência |
| `DMR-EX84C_PROG205` (`EXTRHEADMINI`) | `0x50400000` | `0x50400011` | `0x5043B464` | S-Box `1` ocorrência |
| `DMR-EX86_PROG202` / `EZ49V_PROG201` | `0x50400000` | `0x50400011` | `0x50439494` | S-Box `1` ocorrência |
| `DMR-BS850/BS880/BS885/XW300 PROG0.00` | `0x50800000` | `0x50800000 -> 0x50800010` | `—` *(UniPhier SWUP NAND)* | `FUN_50804ad4` (`[SWUP] make key`) |

### 3.3 Deslocamento de `_dev_SERIAL` nos Binários `MAIN` (`0x57000000` vs `0x50622000`)

| Série / Modelo `MAIN` | Base VA | Anchor `_dev_SERIAL` Verificado |
|---|---:|---:|
| `DMR-EH55_MAIN3410PQ` (`f5502f29`) | `0x57000000` | `FUN_574dac1c @ 0x574DAC1C` |
| `DMR-EH58_MAIN.BIN` | `0x57000000` | `FUN_574f080d @ 0x574F080D` |
| `DMR-EX78_MAIN3410MS` | `0x57000000` | `FUN_5750165b @ 0x5750165B` |
| `DMR-EX81S_MAIN3410EE` | `0x57000000` | `FUN_57502a80 @ 0x57502A80` |
| `DMR-EX88_MAIN3500MM` | `0x57000000` | `FUN_57520f76 @ 0x57520F76` |
| `DMR-EX769_MAIN3400YX` | `0x50622000` | `FUN_50a8f9af @ 0x50A8F9AF` |
| `DMR-EZ49V_MAIN3100Y1` / `MAIN3200Y1` | `0x50622000` | `FUN_509f3812 @ 0x509F3812` |
| `DMR-EX773_MAIN3200XC` | `0x50622000` | `FUN_50b403a1 @ 0x50B403A1` |
| `DMR-EH49_MAIN3100YA` / `EX84C_3030YN` / `EH49_3200YL` | `0x50622000` | `0x50B5272D` / `0x50B5527E` / `0x50B646F7` |
| `DMR-EX79_MAIN3400YV` / `YH` / `EX72S_3010YK` | `0x50622000` | `0x50B6C111` / `0x50B6E0A7` / `0x50B73FEC` |
| `DMR-EH49_3100YS` / `EX89_3400YW` / `EX83_3220YE` / `EX93_3240YM` | `0x50622000` | `0x50B80961` / `0x50B875E0` / `0x50BC6CE8` / `0x50BCBCFC` |

---

## 4. Provas Literais em Pseudocódigo C (Excertos Reais da Frota)

### 4.1 `DMR-EH50_PROG019` (`0x50030400`) — Leitura de ID `NFROM` via MMIO `0xD400F000`

```c
/* === FUNCTION FUN_5003561d @ 0x5003561D (size=103 B) === */
undefined4 FUN_5003561d(void) {
  undefined4 uVar1;
  undefined1 *puVar2;
  undefined1 auStack_4 [4];

  puVar2 = auStack_4;
  func_0x5003671a();
  Ramd400f004 = 0;
  *(undefined **)(puVar2 + -4) = &UNK_c8000000;
  *(uint *)(puVar2 + -4) = *(uint *)(puVar2 + -4) | 0x100;
  *(uint *)(puVar2 + -4) = *(uint *)(puVar2 + -4) | 0x90;
  Ramd400f000 = *(undefined4 *)(puVar2 + -4);
  uVar1 = Ramd400f014;
  **(undefined2 **)(puVar2 + 8) = (short)uVar1;
  func_0x5003672e();
  func_0x50037226(s_NFROM_id___0x_x_500528e6,**(undefined2 **)(puVar2 + 8));
  return 1;
}
```

### 4.2 `DMR-EH49_PROG071` (`EXTRHEADMINI` `0x50400011`) — Mapa de Arquivos `[nfrom]`

```c
/* === FUNCTION FUN_5040168c @ 0x5040168C (size=304 B) === */
undefined4 FUN_5040168c(void) {
  int iVar1;
  char *pcVar3;
  uint uVar4 = 0;

  iVar1 = func_0x50401591(2,0,0);
  if (iVar1 == 0) {
    func_0x5042c0d2(0x51917024,0xff,0x1adc);
    ...
    do {
      iVar1 = func_0x50401629(uVar4 + 0x802 & 0xffff);
      if (iVar1 != 2) {
        pcVar3 = s__nfrom__File_map_init_error__d___50435d0f;
        goto LAB_504017b0;
      }
      uVar4 = uVar4 + 1;
    } while ((uVar4 & 0xffff) < 4);
```

### 4.3 `DMR-EH49_DRV_S514` (`0x40000000`) — Comando Serial `PASS` (`MATS` / `HITA`)

```c
/* === FUNCTION FUN_40139eda @ 0x40139EDA (size=57 B) === */
undefined4 FUN_40139eda(void) {
  int iVar1 = func_0x40027d08();
  int iVar2 = func_0x40027d08();
  if ((iVar1 == 0x4d415453) && (iVar2 == 0x48495441)) {
    bRam00000cd8 = bRam00000cd8 | 1;
  }
  return 0;
}
```

### 4.4 UniPhier (`DMR-BS850_PROG0.00` `0x50800000` & Kernel Linux 2.6.11.12 `0x90100000`)

```c
/* === FUNCTION FUN_50804ad4 @ 0x50804AD4 (DMR-BS850_PROG0.00) === */
undefined4 FUN_50804ad4(int *param_1) {
  int iVar1 = param_1[1];
  int iVar2 = *param_1;
  if (iVar2 == 0) {
    func_0x508166d0(s__SWUP__arg_err_F_08x__50819e92,FUN_50804ad4);
  } else {
    if (iRam52020170 == 0) {
      FUN_50816694(s__SWUP__make_key_50819e81);
      FUN_50819611(0x52025d64,0x52025d6b,0x52046d84);
      iRam52020170 = 1;
    }
    for (; 0 < iVar1; iVar1 = iVar1 + -8) {
      FUN_508198f8(0x52046d84,iVar2);
      iVar2 = iVar2 + 8;
    }
  }
  return 1;
}
```

### 4.5 `DMR-EZ485V` (`sparc:BE:32:default` @ `0x40000000`) — Console UART ZEBRA (`0xFFBE0100`)

```c
/* === FUNCTION FUN_400003ec @ 0x400003EC (size=64 B, SPARC V8 VxWorks BAL) === */
void FUN_400003ec(char *param_1) {
  char cVar1;
  while (true) {
    cVar1 = *param_1;
    param_1 = param_1 + 1;
    if (cVar1 == '\0') break;
    do {
      DAT_ffbe0100 = cVar1;
    } while ((DAT_ffbe0114 & 0x20) == 0);
  }
  return;
}
```

---

## 5. Auditoria Java (`PanadigaMn103sTrapAudit.java --scope=exec`)

Executada em modo `-noanalysis -readOnly` via Ghidra Headless `12.1.4_PATCHED` sobre 7 binários representativos da nova frota (cobrindo `IPL`, `PROG`, `UniPhier PROG0.00`, `DRV` e `MAIN`):

| Programa Auditado | Exit Code | Achados `am33.sinc` Trap #8 (`fb 9c`) | Observação |
|---|---:|---|---|
| `DMR-ES35V_ES35V_IPL_ver018` | `0` | Múltiplos sítios `fb 9c` (ex.: `0x4402947e`+) | Cobertos na leitura segura da Armadilha #8 |
| `DMR-EH50_PROG019` | `0` | **6** sítios (`0x5003ec71`..`0x50042ed8`) | Base `0x50030400` resolvida sem falhas |
| `DMR-EH49_PROG071` | `0` | Múltiplos sítios (`0x5042bb7c`..`0x5042bf8f`) | FTL `[nfrom]` decompilado (`553` funções) |
| `DMR-BS850_PROG0.00` | `0` | Múltiplos sítios (`0x508158c5`..`0x50819441`) | Loader UniPhier SWUP `427/427` C |
| `DMR-EH49_DRV_S514` | `0` | **0** sítios | Firmware FE optical drive `0` ocorrências |
| `DMR-EH50_MAIN3360NA` | `0` | Executado com sucesso | `31.212` funções, `0` undef |
| `DMR-EH49_MAIN3100YA` | `0` | Executado com sucesso | `43.294` funções, `0` undef |

---

## 6. Inventário Forense Transparente: Payloads `STB` (ST20 / MStar), Drives Ópticos `J147`/`J315` e Itens Excluídos

Todos os **15 módulos `STB`** extraídos dos `.FRM` europeus/DX foram integralmente decriptados (Feistel + descompressão `zlib` nos 11 casos compactados e remoção de cabeçalho ASCII `CRC:... size:... Model:...` nos 4 casos `STB_RAW`), porém operam em processadores auxiliares DVB sem módulo SLEIGH no Ghidra padrão (**STMicroelectronics ST20/OS20** ou **MStar**). O mesmo vale para os microcontroladores ópticos `DRV_J147`/`DRV1J315` e para `DEV.BIN` (`512 B` metadados `EH58`):

- **Inventário completo em texto plano:** [`./raw/fleet_stb_and_non_mn103_inventory.txt`](./raw/fleet_stb_and_non_mn103_inventory.txt) e registros estruturados em [`fleet_verify.json`](../audits/zero_undef/fleet_verify.json).
- **11 `STB` STMicroelectronics ST20/OS20 (`FRM_FEISTEL+STB_ZLIB`, `@0x80000400`):** `EX72S_112`, `EX769_104`, `EX773_109_EC`, `EX773_109_EP`, `EX78_103`, `EX79_104`, `EX79_105`, `EX81S_112`, `EX83_109_EC`, `EX86_107`, `EZ49V_105`.
- **1 `STB` ST20 cru (`FRM_FEISTEL+STB_RAW`):** `DMR-EX80S_STB_115` (`M5SAT`, `1.438.792 B`).
- **3 `STB` MStar (`FRM_FEISTEL+STB_RAW`, prefixo `MS EX...`):** `EX795_STB_108` (`M3H264`), `EX84C_STB_107` (`M1H2EG`), `EX93_STB_139` (`M2H2EG`).
- **4 imagens NAND Linux UniPhier completas (`MAIN_LINUX_NAND`):** `BS850` (`64.749.568 B`), `BS880` (`64.749.568 B`), `BS885` (`64.749.568 B`), `XW300` (`51.380.224 B`) — seus prefixos de kernel executável foram isolados em `@0x90100000` (linhas 7, 9, 11 e 30 da tabela), e os demais offsets pertencem a partições `CramFS` (`0x28CD3D45`), `-rom1fs-` e `SquashFS` listadas em [`fleet_uniphier_progs_and_linux.txt`](./raw/fleet_uniphier_progs_and_linux.txt).
- **3 arquivos não-Feistel / não-executáveis preservados para transparência forense:**
  - `dmr-ex_ez_series/ex97eb/PANAEUSB.FRM` (`107.417.648 B`, entropia `8.000`, formato USB encriptado exigindo `keys.ukf` externo);
  - `dmr-ex_ez_series/ex97seg_ceg/PANAEUSB.FRM` (`112.118.212 B`, entropia `8.000`, formato USB encriptado);
  - `dmr-ex98v/panasonic_dmr_ex98v_firmware_cd.zip` (contém apenas `11` digitalizações fotográficas `.jp2`, sem binário executável).
