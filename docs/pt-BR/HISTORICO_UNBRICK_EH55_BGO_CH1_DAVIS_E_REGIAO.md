> 🌐 **Idioma / Language:** [English (US)](../en-US/EH55_UNBRICK_BREAKTHROUGH_BGO_CH1_DAVIS_AND_REGION.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Dossiê Histórico e Forense EH55: Recuperação BGO (`SA39`/`SA54`), `PANAMON1>` Direto, Polaridade `DAVIS`, Pinagem `P58001` / `IDE2SAT2`, Rotas Diskless & Região `PAL-M`

> **Escopo & Regra Epistêmica Obrigatória:** Este dossiê consolida a linha do tempo cronológica e as provas físicas/estáticas obtidas na bancada `COM10` do **Panasonic DMR-EH55 (`VEP79138`)**, no host Windows (`\\.\PhysicalDrive1`) e na desmontagem MN103S. Para impedir qualquer extrapolação indevida (*overclaiming*), **toda afirmação técnica carrega explicitamente um dos 4 selos normativos de maturidade**:
> - **`[PROVADO EM HARDWARE REAL]`** — Evidência medida diretamente na placa `VEP79138`, captura UART `COM10`, multímetro DC ou read-back SHA-256 em `PhysicalDrive1`.
> - **`[PROVADO EM EMULADOR Mn103Vm]`** — Código MN103S / FSM Spansion executado de forma determinística no emulador Rust `panadiga-core::re::mn10300_emu`.
> - **`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]`** — Descompilação, endereços, estruturas e planos *clean-room* derivados de binários e esquemáticos, ainda não executados integralmente em silício.
> - **`[EM INVESTIGAÇÃO / BLOQUEIO FÍSICO CH1]`** — Etapa fisicamente bloqueada pelo defeito elétrico ativo no caminho `P58001`–`VEK0K00`–`N5EZZ0000003`–`StarTech IDE2SAT2`; **o firmware `MAIN` ainda NÃO bootou a partir do HDD em hardware real**.

---

## 1. Matriz Global de Maturidade e Estado Real em Bancada

| Subsistema / Descoberta | Selo de Maturidade | Evidência Verificada |
| :--- | :--- | :--- |
| **Cura BGO `U99` no `Loader ver0.61` (Ciclo 1 + Ciclo 2 `SA54/SA41/SA29/SA39`)** | `[PROVADO EM HARDWARE REAL]` | [`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log) · [`repair_bgo_sa39_sa54_com10_20261119.log`](../debug_logs/repair_bgo_sa39_sa54_com10_20261119.log) |
| **Entrada direta em `PANAMON1>` sem JIG via `PANA__SD.FRM` (`cdcc2cdf...`, `0x214E681`)** | `[PROVADO EM HARDWARE REAL]` | [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) · [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) |
| **Polaridade assimétrica `DAVIS` (`CH0` ativo-BAIXO vs `CH1` ativo-ALTO) + `CH0` `0x50`** | `[PROVADO EM HARDWARE REAL]` | [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) (`J 504156AD` $\rightarrow$ `0x504F81D0=2`, `0x504F81D4=1`) |
| **Pinagem reversa `P58001` (`Pin k = ATA (41 - k)`) & prova `IDE2SAT2` `180°` (`0x00002004`) vs `0°` (`0x00000000`)** | `[PROVADO EM HARDWARE REAL]` | `vol2_schematics.pdf` p. 113 + [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) + [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) |
| **Gravação CoW e read-back SHA-256 em `PhysicalDrive1` (`250 GB` + espelho `LBA 488266096`)** | `[PROVADO EM HARDWARE REAL]` | [`hdd_unbrick_flash_250gb_20261119.log`](../debug_logs/hdd_unbrick_flash_250gb_20261119.log) (`4fde753c...` + `d932692c...`) |
| **Extração `ifodev2` (`2.048 B`, `"M6930LB"`, `f4f22dd0...`) da NOR reparada** | `[PROVADO EM HARDWARE REAL]` | Extraído de `eh55_nor_8mb_ic5001_repaired.bin` (cadeia `[0x0001, 0x0002, 0x0055, 0x0004]`) |
| **Kit `sd_unbrick_kit`, FSM Spansion, stubs `davis_probe`, `eh55_bgo::repair_plan`, `tv_standard`, `capacity_patch` (`5 GB..2 TB`)** | `[PROVADO EM EMULADOR Mn103Vm]` | Suíte Rust em `crates/panadiga-core/` (`mn10300_emu` com `0xFC 0xE0..0xE3` e stubs `0xEC`) |
| **Incompatibilidade `ES15/ES10/ES35V`, rotas `0x84040000`/`PROG` `0x0B1800`/`VDM_Init`, topografia `FRONT(L)`/`KEYIN1..3`/`IR7501` e planos `AIOF`/`AVDD` (`PAL-M`)** | `[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]` | `vol1`/`vol2`/NA schematics + desmontagem `Loader ver0.61` (`Ldr061_v2`) + `MAIN3410PQ` |
| **Leitura ATA real no `CH1` (`0x00000000`, `is_ata=0`), boot real do `MAIN` pelo HDD `250 GB`, bypasses `MAIN` e `PAL-M`/`TBC` após `MAIN`** | `[EM INVESTIGAÇÃO / BLOQUEIO FÍSICO CH1]` | Multímetro `CH1` (`IORDY`/`DIOR-`/`DIOW- = 0V` conectado) bloqueia `Loader` antes de carregar a Partição 1 |

---

## 2. Linha do Tempo Forense: Do Erro `U99` à Cura BGO em 2 Ciclos (`SA39`/`SA54`)

### 2.1 Ciclo 1 — Cura Inicial da FTL BGO de 45 Setores (`0x440337A4`) `[PROVADO EM HARDWARE REAL]`

A partir do dump ouro [`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md) (`8.388.608 B`, `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`), a desmontagem de `Loader ver0.61 [06/03/06]` provou que o DMR-EH55 usa uma tabela própria em **`0x440337A4`** com **`45` (`0x2D`) setores de `64 KiB` em `0x84130000..0x843FFFFF` (`SA26..SA70`)** — diferente da janela de `35` setores em `0x84200000..0x843FFFFF` do `PROG061` (`0x50436E30`) / `Loader ver0.63`.

No Ciclo 1 (`apply_bgo_repair_com10.log`), sob `PANAMON1>` (`U CONFIRM_UPPER_NOR` $\rightarrow$ `U LOCK`, mantendo **`SA0..SA3` em leitura estrita `E05`**):
1. Formatou-se `SA70` (`0x843F0000`) com `256` bytes `0x00` em `0x843F0100`;
2. Aplicaram-se **`1.629` escritas puras `1 -> 0`** em `13` setores spare (`SA42..SA66`), restaurando os `13` blocos `0xF2xx` perdidos (`0xF209`, `0xF21D..0xF228`), o bloco 2 de `ifodev2` (`0x0055` @ `0x84250800`) e as páginas FAT `0x0F0A` / `0x0F19` (`0x8427A200` / `0x8427AE00`);
3. Aplicaram-se **`6` escritas puras `1 -> 0` em `SA41 + 0x14`** (`0x84220014..0x84220023`), restaurando o link de migração `0x0004` $\rightarrow$ `SA29` e gerando a imagem offline equivalente [`eh55_nor_8mb_ic5001_repaired.bin`](../../firmware_and_images/README.md) (`SHA-256: adb21d955ed96121249e7abeec58e1ce3951f511a595636d70ebb07a35ca4c9c`, `19/19` arquivos `[nfrom]`, `996/996` tags L2P).

### 2.2 Recaída `[nfrom] file_no[17]` via FVU Stock & Ciclo 2 (`SA54`/`SA41`/`SA29`/`SA39`) `[PROVADO EM HARDWARE REAL]`

Quando o canal `CH1` falhou no `Boot 2` subsequente (`[RTSD] HDD spin-up error (80000404)` / `IDE HOST ERROR (E8000000)` $\rightarrow$ `HDD_MAX_LBA get failed[0xE8000000].` $\rightarrow$ `mainsoft data is invalid, so set update flag.`), o bootloader montou um cartão SD legado contendo **`Fvu ver0.61 [05/11/01]`** (`checksum 0x2182DC9`, `PROG061` stock de 35 setores sem bypass). Ao inicializar na janela legada `0x84200000..0x843FFFFF`, o comando **`[nfrom] file_no[17],skip(0),non_skip(1),erase(0),new_write(0).` apagou `SA54` (`0x842F0000`)** e rompeu novamente a árvore BGO.

O **Ciclo 2 de Restauração Cirúrgica** (`repair_bgo_sa39_sa54_com10.py`) reconstruiu a BGO via `PANAMON1>` sobre `BRIDGE 115200 PASSIVE` utilizando:
* **Ping seguro `?\r`** (evitando `\r` nu que desalinhava o parser de linha do monitor) e **retentativa única com passo de `300 µs` por byte** (`~183 B/s`, ~15× mais rápido que o envio byte-a-byte anterior);
* **Guarda `U CONFIRM_UPPER_NOR` + `U LOCK` imediato** (respeitando integralmente o bloqueio `E05` em `SA0..SA3` e realizando *read-back* de 100% dos alvos):
  1. **`SA54` (`0x842F0000`):** restauração `1 -> 0` de cabeçalho (`+0x000..+0x0FF`), tags (`+0x200..+0x3FF`, bloco lógico `0xF20D`) e **21 slots ativos** (`1, 25..30, 71..77, 118..124`) validados byte-a-byte contra `eh55_nor_8mb_ic5001_repaired.bin` (`adb21d95...`).
  2. **`SA41` (`0x84220000`, slot 9):** cópia do payload de `live_sa39_slot8_0054.bin` (`512 B` @ `0x84221600`) + gravação da tag `0x54 0x00` (`LSN 0x0054`) em `0x84220224`.
  3. **`SA29` (`0x84160000`, tabela L2P):** promoção da página `0x51` (`tid=0x0005`, `t2=0x013F`, `entry[4]=0x0A10`) para a página `0x64` slot `s=6` (`0x841664DC`, `t2=0x0140`, `entry[4]=0x0A12`), mutação monotônica `1 -> 0` `0xC0C0C0FD -> 0x808080FD` em `0x84166400..6402`, e invalidação da página `0x51` em `0x84165103` (`0xD3 -> 0xD2`, formando `0x808080D2`).
  4. **`SA39` (`0x84200000`):** comando `E 84200000` (`OK:ERASE`), polling até confirmar `0xFF`, e formatação *spare* gravando `256` bytes `0x00` em `0x84200100..0x842001FF`.

**Prova de Boot Normal Pós-Ciclo 2 ([`verify_bgo_restored_boot_20261119.log`](../debug_logs/verify_bgo_restored_boot_20261119.log) & [`repair_bgo_sa39_sa54_com10_20261119.log`](../debug_logs/repair_bgo_sa39_sa54_com10_20261119.log)):**
Em `BOOTTRACE 15000 NOJIG`, o `Loader ver0.61 [06/03/06]` executou `DDR CHECK: OK` $\rightarrow$ `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***` **sem emitir `BGO driver init failed[1].` no `Loader`** (que só reapareceu quando o fallback por falha de `CH1` carregou o cartão com FVU stock `0x2182DC9`, confirmando a importância do kit canônico abaixo).

---

## 3. Entrada Direta em `PANAMON1>` sem JIG (`PANA__SD.FRM` `cdcc2cdf...`) `[PROVADO EM HARDWARE REAL]`

Em [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log) (`VDD: 3.26V`) e [`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log) (`VDD: 3.25V`), ao substituir o cartão pelo kit canônico [`firmware_and_images/sd_unbrick_kit/PANA__SD.FRM`](../../firmware_and_images/sd_unbrick_kit/SHA256SUMS.txt):
* **`PANA__SD.FRM` canônico (`sd48`, `7.181.224 B`, `SHA-256: cdcc2cdf4617c4f08d8af3f617db369cfd360d8db2601b33c84bae0e6f995d90`):** empacota **6 slots de TOC de 48 B** apontando para os 3 mesmos payloads nas duas famílias (`M6930LB` slots `0..2` e `M6930` slots `3..5`, `[0x18]=0x4D`), com `PROG061` em modo `UART_MONITOR_PANAMON1` (**checksum real `0x214E681`**).
* Quando o `Loader ver0.61` encontra `HDD_MAX_LBA get failed[0xE8000000]` no modo `normal` (`NOJIG` / `NORMAL`), ele monta o SD (`0x3c4000[LBA]`), lê `0xB1800` $\rightarrow$ `0x90400000` (`cheking for fvu program check-sum[0x214E681].`) e entra **diretamente em `PANAMON1>[B/R/W/D/F/J/E/U/Q]` sem precisar do pino `SW_UART` (JIG)**, habilitando sondagem completa de RAM e registradores `DAVIS` ao vivo!
* **Perigo comprovado do `Fvu` stock (`0x2182DC9`):** manter um cartão com `Fvu ver0.61 [05/11/01]` (`0x2182DC9`) em `normal mode` dispara `BGO driver init failed[1].` e o comando destrutivo `[nfrom] file_no[17]` em `SA54`.

---

## 4. Sondagem `DAVIS` IDE, Pinagem Reversa `P58001` (`41 - k`) & Prova `IDE2SAT2` (`0°` vs `180°`)

### 4.1 Polaridade Assimétrica de Reset do Controlador `DAVIS` `[PROVADO EM HARDWARE REAL]`

Executando `J 504156AD` (`rtsd_hw_init`) em [`probe_ch1_full_diag_20261119.log`](../debug_logs/probe_ch1_full_diag_20261119.log):
* `DAVIS_TOP` (`0x89000000`) = `0x5963464C`.
* **`0x504F81D0 = 0x00000002` (`CH0` ODD ativo-BAIXO):** liberado por `0x89000420 = 0x02`/`0x03`. O controle positivo **`1C CH0`** passou ao vivo (`STATUS=0x50`, `ALTSTAT=0x50`, `DEVSEL=0xA0`, `SECCNT=0x03`, `LBAMID=0xFF`, `LBAHI=0xFF`, `DATA=0xFFFF`, `SENTINEL=0xCAFE0001`).
* **`0x504F81D4 = 0x00000001` (`CH1` HDD ativo-ALTO):** liberado por `0x89000C20 = 0x00`, coerente com inversor externo na linha `RESET-` de `CH1`.
* **Guarda de registradores letais:** `DavisProbeEngine` bloqueia determinísticamente qualquer leitura do endereço letal **`0xC000_100C`** (`AM34_FORBIDDEN_LETHAL_REG`, trava WDT/bus observada nos ensaios iniciais; além do hang em `0xDC0F0000` visto em `probe_davis_ide_ch1.log`).

### 4.2 Pinagem Reversa de `P58001`, Ponte `VEK0K00` / `N5EZZ0000003` & Prova `0°` vs `180°`

* **`vol2_schematics.pdf` p. 113 (`P58001` na Digital PCB) `[PROVADO EM HARDWARE REAL]`:** comprova geometricamente que o conector FFC de 40 pinos **`P58001`** possui numeração invertida segundo a lei exata:
  $$\text{Pino}_{P58001}(k) \equiv \text{ATA}(41 - k)$$
  Exemplos diretos do esquemático: `Pin 1=GND`, `Pin 3=CS1-`, `Pin 4=CS0-`, `Pin 13=CSEL`, `Pin 14=IORDY`, `Pin 16=DIOR-`, `Pin 18=DIOW-`, `Pin 21=NC` (chave ATA 20), `Pin 27=DD13`, `Pin 28=DD2`, `Pin 39=GND`, `Pin 40=RESET-`. O flat `VEK0K00` liga `P58001` à placa adaptadora **`N5EZZ0000003`** (`item 13`, 40 pinos contínuos verificados; apenas o conector macho da `StarTech IDE2SAT2` tem pino 20 chaveado/ausente).
* **Prova empírica `IDE2SAT2` `180°` (`0x00002004`) vs `0°` (`0x00000000`) `[PROVADO EM HARDWARE REAL]`:**
  * Ao rotacionar a `StarTech IDE2SAT2` para **`180°`** (`probe_ch1_full_diag_20261119.log`, `2A MASTER`/`SLAVE`), **todos os 9 registradores ATA de `CH1`** retornaram **`0x00002004`** = bits **`DD13` (`0x2000`, ATA 14)** lendo `ATA 27 (IORDY = 3.3V)` + **`DD2` (`0x0004`, ATA 13)** lendo `ATA 28 (CSEL = 3.3V)`! Isso **prova matematicamente** a equivalência geométrica Tipo B / `180°` invertido e comprova que **`0°` é a orientação correta**.
  * Na orientação **`0°`** ([`probe_ch1_live_v2_0deg_20261119.log`](../debug_logs/probe_ch1_live_v2_0deg_20261119.log)), `CH1` parou de travar a CPU em `R 89000D58,4`, porém retornou **`0x00000000`** em `2A..2C` e em `FUN_5041CD4D(1)` (`rc=0x00000000`, `struct_504F8234`: `is_init=1, is_atapi=0, is_ata=0`).
* **Isolamento Elétrico Atual (`[EM INVESTIGAÇÃO / BLOQUEIO FÍSICO CH1]`):** medições em DC no caminho `P58001`–`VEK0K00`–`N5EZZ0000003`–`IDE2SAT2` revelaram `RESET- = 2.43V`, `CS0- = 3.02V`, `CS1- = 3.23V`, mas **`IORDY`/`DIOR-`/`DIOW- = 0V` com o adaptador conectado** (subindo `IORDY = 3.29V` e linhas digitais `0..0.2V` com FFC desconectado). **Consequência:** a falha física de continuidade/curto nessa interconexão impede que `CH1` responda a `IDENTIFY (0xEC)`, logo **`MAIN` ainda NÃO bootou pelo HDD no EH55 físico**.

---

## 5. Preparação de Disco `250 GB` em `PhysicalDrive1` (`hdd_unbrick_flash_250gb_20261119.log`)

* **`[PROVADO EM HARDWARE REAL]` (Camada Host Windows):** Em [`hdd_unbrick_flash_250gb_20261119.log`](../debug_logs/hdd_unbrick_flash_250gb_20261119.log), `\\.\PhysicalDrive1` (`250.059.350.016 B`, `BusType=7`, `488.397.168 LBAs`) recebeu:
  1. Backup prévio CoW de `65.536 B` (`354d38bd...`), registrando MBR Windows x86 `55aa` (`3db0e904...`) em `LBA 0` enquanto `LBA 24..31`, `LBA 16384`, `LBA 57344` e `LBA 131072` já coincidiam com `hdd_100.bin`;
  2. `IOCTL_DISK_DELETE_DRIVE_LAYOUT` + gravação verificada por read-back SHA-256 de `panasonic_dmr_eh55_hdd_250gb_patched.img` (`104.857.600 B`, `SHA-256: 4fde753c1f4aefb8847621e3bef16dcb9493dc8ef4c52a230f3bb8dfcd34c1c2`, `LBA 0[0:16]=0000000001005348628c6ef51b2c3666`, `LBA 0[16:512]` 100% zeros);
  3. Gravação e read-back SHA-256 do espelho alinhado de Partição 1 (`6.173.696 B`, `SHA-256: d932692c2daca4af9fd86c030341c213774a6477d7ca3305add9122100f265ed`) no LBA alto **`488266096`** (`((disk_bytes / 512) & !3) - 0x20000`).
* **`[EM INVESTIGAÇÃO / BLOQUEIO FÍSICO CH1]` (Camada Target EH55):** A inicialização real dessa imagem `250 GB` no SoC `AM34` aguarda o desbloqueio físico de `CH1`.

---

## 6. Veredito `ES15`/`ES10`/`ES35V` vs `EH55`, Rotas Diskless/TBC e Painel/IR (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]`)

1. **Incompatibilidade irreconciliável de `DMR-ES15`/`ES10`/`ES35V` na NOR `IC5001` do `EH55` (brick garantido):**
   * **`ES10` (`4 MiB` `VEP79104A`, `MN2DS0012-H` `AM33`, sem EEPROM em `IC7404`)**, **`ES15` (`8 MiB` `A-3 863ZES MP-1`, `MN2DS00151DB`, `24LC64` 8 KiB)** e **`ES35V` (`8 MiB`, `Loader ver0.18 [06/03/13]`, `24C64` 8 KiB)** posicionam `MAIN` XIP em **`0x130000..0x40FFFF`** (exatamente a área BGO `SA26..SA70` do EH55) e `EXTRHEAD MAIN 3080GR` LZMA em `0x430000..0x6D42F0` (onde reside o Upper EEMUL `"1284"` do EH55).
   * Além disso, o conector B2B de 88 pinos `P7402 <-> P59001` do EH55 (`DR12V` pinos 14/16, `DR5V` pinos 22/24, `X_SW_12V`/`D1R2V` pinos 64..74 alimentando `IC59501`+`Q59501`), a EEPROM I²C (`RFKFM6016KT` / `C3EBFC000053` 2 KiB em `IC7404`/`IC7401`) e o handshake `TBUS` do Syscon `IC7501` (`C2CBJG000713` EC / `MN67788RA` NA) são incompatíveis.
2. **Rotas Viáveis Diskless / TBC no `DMR-EH55` sem tocar em `SA0..SA3`:**
   * **Porta #1 — `PANA__SD.FRM` slot `PROG` (`0x0B1800` $\rightarrow$ `0x90400000`):** validado apenas por soma 32-bit (`func_0x4400c172`, sem RSA), já alcançado automaticamente quando `HDD_MAX_LBA` falha (`FUN_4400877f` $\rightarrow$ `FUN_44008bc5` $\rightarrow$ `FUN_44009703`).
   * **Porta #2 — Slot `Nor-FROM` em `0x84040000` (`SA11..SA14`):** `FUN_4400877f` consulta BGO para `"fvu.exe"` (`0x504800AF`), copia `0x40000` bytes de `0x84040000` $\rightarrow$ `0x90400000` e salta para `0x50400000` sem SD (depende da política de preservação da flag `"update"` `"ATIHSTAM"` em `FUN_44007f0e`).
   * **Bloqueios de `MAIN3410PQ` sem HDD:** os 3 patches atuais de `AV_PASSTHROUGH` (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) **não** impedem `VDM_Init()` (`0x57323EC3`, `"###### ILLEGAL HDD ######"` / `"HDD NG"` loop infinito @ `0x574F5D37`), `SDM_Init()` / `SDM_Mount("/dev/hda1")` (`FUN_574f5c6a`), nem `FUN_574f469d` msg `1`/`9` (`"Drive Err -> P-OFF!"` / `"HDD Err -> P-OFF!"` @ `0x574F569E`/`0x574F56F8` $\rightarrow$ `func_0x575097d3()`). Por isso, `re::tbc_appliance` separa o modo seguro **`Stage15DigadiagTbc`** (bare-metal `<= 256 KiB` via `DIGADIAG.FRM`) do modo experimental bloqueado **`ExperimentalSdRamChainload`**.
3. **Topografia `FRONT(L)`, `KEYIN1..3`, `IR7501` & Standby `SERV` vs `MAIN`:**
   * `FRONT(L) P.C.B.` (`JW7001 <-> JW7501` / `P7001 <-> P7505`) possui **apenas `[POWER]` (`S7002`)** ligado a `KEYIN3` (pino `88` EC / `18` NA). Todas as outras 8 teclas (`EXT_LINK`, `CH_DOWN`, `CH_UP`, `OPEN/CLOSE`, `SELECT`, `STOP`, `PLAY`, `REC`) e **`IR7501` (`PNA4618M09VT`)** ficam na **Main PCB** alimentados por `+5V` (`X_SW_5R2V` via `R7648` `47 Ω` + pull-ups `8,2 kΩ` `KEYIN1..3` nos pinos `86..88` EC / `16..18` NA; placa NA liga ainda `P7501` Pino 1 `SW_UART` via `K7502` `0 Ω` ao pino `80` `IR_BUSY` de `IC7501`, enquanto EC deixa pino `80` `NC`).
   * `SERV` (`[CH UP]+[OPEN/CLOSE]+[REC]` 5s) e `TEST` (`[STOP]+[CH UP]+[OPEN/CLOSE]` 5s) só são aceitos por `IC7501` em **Standby real (`Power OFF`, `DI_P_ON = 0V`)**; o tratamento real do menu `SERV`, `R_SERVICEMODE` (`0x57A116CB`) e recepção contínua de teclas/IR via `TBUS` (`FPC_ReciveTASK` @ `0x57060DDC`) residem no `EH55_MAIN`, que não sobe enquanto `CH1` falha (disparando WDT `TBUS` de `255s` $\rightarrow$ `U99`).

---

## 7. Região `PAL-M` / `LBS` / `ifodev2`, `capacity_patch` & Centralização Rust

* **`ifodev2` (`2.048 B`, `0x0055`, `SHA-256: f4f22dd0407d9f0bebf4621e938f2ca02e3b6c5f23b21ae972c5185704a77676`):** `[PROVADO EM HARDWARE REAL]` na leitura da NOR reparada (`AA55FFFF`, `"M6930LB\0"`, `0x0C='M'`, `"PANA_DVD_REC"`); sua cauda não verificada (`0x400..0x7FF`) é marcada estritamente como `metadata_and_dev_oem_only = true` em `re::region_compat::tv_standard` para proibir mutação silenciosa.
* **`AIOF` / `AVDD` / `IC3001` & Patches (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]` / `[PROVADO EM EMULADOR Mn103Vm]`):** `AM34_AIOF_BASE = 0x8800_0000` (`AIOF_VMODESEL`: `0x8020` NTSC, `0x8021` `cps_pal_m`, `0x8027` PAL; `FS0 +0x2C`, `YC3 +0x64`, `CONT +0xA4`), `AM34_AVDD_BASE = 0x8980_0000` (`NPALCTL0 +0x0624`), matriz A/V `IC3001` (`C1AB00002379` EC vs `C1AB00001979` NA), bypass `region_compat` (`3410PQ @ 0x57498738`, `3060NP @ 0x574A01AA`), e `hdd::capacity_patch` cobrindo **`5 GB` (`0x0024C388` `SV0511D`) até `2 TB` (`DYNAMIC_UNCLAMPED_MAX_BLOCKS_2KB = 0x3FFF_FFFF`, `250 GB = 0x07466528`)**.
* **Módulos Centrais em Rust (`panadiga-core`):** `pico::panamon1` (`Panamon1Client`, `Panamon1NorUnlockGuard` RAII com `U LOCK` em `Drop` + trava `E05`), `re::nor_dumper::{orchestrator, spansion_audit}` (`142` setores Spansion), `re::eh55_bgo::{audit, repair_plan, live_apply}`, `re::davis_probe` (`DavisProbeEngine` + `0xEC`), `re::region_compat::tv_standard`, `re::tbc_appliance` e `hdd::writer::eh55_flash` (backup CoW `64 KiB`, bloqueio `PhysicalDrive0`/`BusTypeNvme`, read-back SHA-256).
