> 🌐 **Idioma / Language:** [English (US)](../en-US/DIGADIAG_PANA_SD_AND_STANDALONE_TBC.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# Dossiê Técnico: Consolidação `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM`, Bloqueio SD `-91006` e Viabilidade de TBC Autônomo (DMR-EH55 / MN103S)

> **Escopo & Lastro Forense:** Especificação baseada em desmontagem MN103S verificada do bootloader real do **DMR-EH55 (`Loader ver0.61 [06/03/06]`**, espelhado em `prog061_dec.bin` `sd_load.o` @ `0x5040eca9` / `0x5040ed7c` / `0x5040ea7d` e NOR de março de 2006 `flash.bin` @ `0x440085d9` / `0x440095ab` / `0x4400bc70`), contrastado com `/BOOT.BIN` (**`Loader ver0.63 [07/10/24]`**, DMR-EZ485V, NOR `0x44000000`), além de `PROG061` (`0x50400000`), `EH55_MAIN_3410PQ.bin` (`0x57000000`), `MAIN3060NP` (`0x57000000`), `PANA_DVD.FRM`, `hdd_partition_firmware_0x02000000.bin` e esquemas oficiais `VEP79138` (`na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`).

---

## 1. Anatomia dos Contêineres `.FRM` e Como o Gravador os Consome

Uma das maiores fontes de erro ao tentar desbrickar o Panasonic DMR-EH55 é confundir **`DIGADIAG.FRM`** com **`PANA__SD.FRM`** e **`PANA_DVD.FRM`** — e, adicionalmente, confundir o formato esperado pelo **bootloader real do DMR-EH55 (`Loader ver0.61 [06/03/06]`)** com o de geração posterior presente em `/BOOT.BIN` (**`Loader ver0.63 [07/10/24]`**, DMR-EZ485V):

| Arquivo | Estágio Leitor | Formato no DMR-EH55 Real (`Loader ver0.61 [06/03/06]`) | Formato em `/BOOT.BIN` (`Loader ver0.63 [07/10/24]`, EZ485V) | Módulos / Carga Real Empírica | Uso no Unbrick & Manutenção |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`DIGADIAG.FRM`** | **Estágio 1: `sd_diag_load`** (`0x4400bc70` / `0x5040eca9` no `ver0.61`; `FUN_4400a97c` no `ver0.63`) | **Binário bruto descomprimido de exatos `262.144 B` (`0x40000`)**, iniciando em `dc 14 00 00 00` (`JMP 0x50400014`), **sem wrapper `EXTRHEAD`**; lido direto para **`0x90400000`** (alias *uncached* de `0x50400000`) com salto direto para `0x50400000` após `*** start diag ***` | Único bloco **`EXTRHEAD`** (`131.072 B` / `0x20000` exatos, tag `"FVU "`, `dst=0x50400000`, `adler_step=4`), lido para **`0x93fc0000`** e descomprimido via `uncompress()` | Payload único MN103S (`262.144 B` em `0x50400000`), ex. `PROG061` patched para BGO (`bypass` / `PANAMON1`) ou `NOR_DMP` | **Porta de entrada Pre-MAIN**: executa antes de `NVRAM U99` e antes do HDD/ODD; se fornecer `131.072 B` (`EXTRHEAD`) ao `ver0.61`, `boot_fat_read` trunca para `0x20000 != 0x40000` (`0x5040ed67`) e aborta com **`SD diag load failed[0].`** |
| **`PANA__SD.FRM`** | **Estágio 2: `PROG061`** (`sd_read_firm_list` @ `0x5040ed7c` / `sd_find_firm_entry` @ `0x5040ea7d` no `ver0.61`) | Tabela **TOC em texto claro com entradas de `48 bytes` (`0x30`)**, até 128 slots (**`0x1800 B` = `6.144 B`**, modo **`sd48`**), validada por `list_sum_u16` | Tabela **TOC em texto claro com entradas de `64 bytes` (`0x40`)**, até 128 slots (**`0x2000 B` = `8.192 B`**, modo **`sd64`**) | Normalmente 3 slots: `DRV S211`, `PROG061 `, `MAIN3060NP` (ou `MAIN3410PQ`), totalizando **`7.531.744 B`** no `sd48` stock (`7.533.792 B` no `sd64`) | Atualização completa gravada na Flash NOR via SD, permitindo persistir **`MAIN3410PQ` com TBC Passthrough (`E-E`) + HDD dinâmico (`>200 GB`)** |
| **`PANA_DVD.FRM`** | **Atualização Óptica** (`CD-R` / `DVD-R` via `MAIN`/`DRV`) | Tabela **TOC de `48 bytes`** (`"Firm list"`) **cifrada em Feistel**; dupla camada (Feistel externo + `EXTRHEAD` parcial interno em `DRV`/`MAIN`) | Mesmo formato óptico Feistel `48 B` | Empírico (`7.531.744 B`): `[0] DRV S211` (`0x001800`, `720.896 B` -> `1.441.792 B @ 0x90202000`), `[1] PROG061 ` (`0x0B1800`, `262.144 B` raw após outer decrypt), `[2] MAIN3060NP` (`0x0F1800`, `6.542.560 B` -> `13.427.852 B @ 0x57000000`) | Fonte oficial para extração e reconstrução de `DIGADIAG.FRM` e `PANA__SD.FRM` |

### Fluxo Completo de Consumo no Boot via Cartão SD (`Loader ver0.61` vs `/BOOT.BIN` `ver0.63`)

```text
[Cartão SD: FAT16 SDSC <= 2 GiB (<= 0xFFF5 clusters)]
   │
   ├──► Estágio 1 (DMR-EH55 Real): Loader ver0.61 [06/03/06]
   │       (espelho prog061_dec.bin sd_load.o @ 0x5040eca9 / NOR 03/2006 @ 0x440085d9, 0x440095ab, 0x4400bc70)
   │       1. Valida SD 1-bit -> troca para 4-bit (ACMD6) -> 2ª checagem ACMD13
   │       2. Monta FAT16 -> abre DIGADIAG.FRM -> exige EXATOS 262.144 B (0x40000) brutos descomprimidos
   │          (se tiver 131.072 B EXTRHEAD, 0x5040ed67 falha cmp 0x40000, d0 -> "SD diag load failed[0].")
   │       3. Carrega direto para 0x90400000 (alias uncached de 0x50400000), fecha FAT, limpa caches,
   │          imprime "*** start diag ***" e executa JMP 0x50400000 (@ 0x44008601)
   │          ├─► Modo bypass (padrão seguro): pula checagens BGO mantendo CS1 IPL/RAM intocados
   │          ├─► Modo uart_monitor (PANAMON1): ativa monitor interativo pré-MAIN (P7501 115200 8N1)
   │          ├─► Modo auto_format (BLOQUEADO fail-closed; exige --allow-unsafe-bgo-format: atua em CS1 0x84200000..0x843FFFFF no PROG061/ver0.63; no ver0.61 real do EH55 o BGO fica em 0x84130000..0x843FFFFF SA26..SA70)
   │          └─► Stub NOR_DMP: extrai a NOR Flash (0x84000000, 8 MiB) via UART em modo somente-leitura
   │
   ├──► Estágio 2 (DMR-EH55 Real): PROG061 em 0x50400000 -> sd_read_firm_list (0x5040ed7c) / sd_find_firm_entry (0x5040ea7d)
   │       └─► Lê PANA__SD.FRM em modo sd48 (TOC plaintext de 48B x 128 = 0x1800 B; stock 7.531.744 B) e grava módulos na NOR
   │
   └──► Contraste (/BOOT.BIN de referência no Ghidra): Loader ver0.63 [07/10/24] (DMR-EZ485V)
           └─► FUN_4400a97c lê 0x20000 B (131.072 B) EXTRHEAD para 0x93fc0000 + uncompress() -> 0x50400000;
               PANA__SD.FRM usa TOC plaintext de 64B (0x2000 B, modo sd64)
```

> **Nota Crítica sobre Builds `MAIN` (`3410PQ` vs `3060NP`) e Modos `sd48` vs `sd64`:**
> A partição de firmware extraída do HD (`firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin`, `6.173.264 B`) descomprime via `panadiga frm unpack --mode main` para **exatos `13.003.052 B` = `EH55_MAIN_3410PQ.bin`** (`VA 0x57000000`), onde todos os 3 sítios TBC Passthrough (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) e o bloqueio LBA48 (`0x574DD1FC`) coincidem byte-a-byte. Já no contêiner óptico `PANA_DVD.FRM` (`7.531.744 B`) e no `PANA__SD.FRM` stock (`7.531.744 B` no modo `sd48` `0x1800 B` do `ver0.61`, ou `7.533.792 B` no modo `sd64` `0x2000 B` do `ver0.63`), o slot `[2]` contém **`MAIN3060NP`** (`13.427.852 B` descomprimido), onde o desvio `odd_branch` desloca para `0x4FE986`. O comando `panadiga unbrick prepare-sd` exige validação explícita de build (`--expected-main-build` / `--allow-target-mismatch`) ao gravar `MAIN3410PQ` em `PANA__SD.FRM` para impedir mismatch silencioso. Para alvos `Loader ver0.61`, use `panadiga unbrick bgo --raw-prog061` (para gerar `DIGADIAG.FRM` bruto de `262.144 B` / `0x40000`) e `panadiga frm pack --mode sd48` (para gerar `PANA__SD.FRM` com TOC `0x1800 B` de `48 B`), reservando `EXTRHEAD` `131.072 B` (`0x20000`) + `sd64` (`0x2000 B`) para `Loader ver0.63`.

---

## 2. O Erro SD `-91006` e Por Que o Pico Via `P7501` Não Pode Forçar Modo 1-Bit em `/BOOT.BIN`

Durante a análise de campo do erro `-91006` no diagnóstico SD, levantou-se a hipótese de forçar o bootloader a permanecer em modo SD **1-bit** usando o Raspberry Pi Pico conectado em `P7501`. A engenharia reversa completa de `/BOOT.BIN` fechou a questão com prova negativa definitiva:

1. **Sequência Exata em `/BOOT.BIN`:**
   * **`0x44027826`:** Primeira chamada a `GetSD_Status` executa com sucesso no barramento em **1-bit** (`0x89001850 = 0x80b8`).
   * **`0x44027F79`:** `/BOOT.BIN` envia `ACMD6(2)` ao cartão para negociar barramento **4-bit** e limpa o bit 15 (`0x89001850 &= 0x7fff`).
   * **`0x44027F93`:** Segunda chamada a `GetSD_Status` falha com **`-91006`** sempre que há falha elétrica nas linhas **`DAT1..DAT3`** (`P6801` -> `VEP73136A` -> `P6802` FFC -> `P53501` -> `MN2DS0009`, hipótese dominante após CMD/CLK/DAT0 já passarem em 1-bit) ou quando o cartão/controlador corrompe `SD_STATUS[2]` (`!= 0`, ou **SDHC > 2 GiB** que falha já em `CMD8`/OCR). Note que o erro óptico **`0x83A7`** não pode ser contornado na IPL NOR somente-leitura e só afeta o fallback de disco `CDDIAG` caso o SD falhe primeiro.
2. **Por que o Pico NÃO pode interceptar via `P7501` durante `/BOOT.BIN`:**
   * As rotinas seriais de recepção em `/BOOT.BIN` (`uart0_getc` em `0x4400479F` -> `console_getline` em `0x4400488A`) **não possuem nenhum chamador ativo** (código morto no bootloader de produção).
   * Além disso, o conector `P7501` **não expõe o clock do depurador On-Chip Debug (`OCD`)**, fazendo com que leituras de controle retornem `DCSR = 0xFFFFFFFF`. Portanto, é fisicamente impossível pausar a CPU ou alterar `0x89001850` no Estágio 1 via Pico.
3. **Mitigação Prática e Segurança Térmica em Bancada:**
   * **Regra Térmica / Curto Antes de Energizar:** NUNCA energize o conjunto com flat FFC aquecido (especialmente flat de 18 vias do pickup óptico ODD) nem conecte/remova FFC energizado; meça a frio a resistência para `GND` de **`D3R3V`/`D+3.3V`/`DR+5V`/`DR+12V`** e continuidade via a via em `P6801`/`P6802`/`VEP73136A`.
   * Utilize exclusivamente cartão **SDSC `<= 2 GiB`** formatado em **FAT16** (`<= 0xFFF5` clusters; FAT32 falha com `-10025`) com `DIGADIAG.FRM` bruto de **`262.144 B` (`0x40000`)** para `Loader ver0.61` (usar `131.072 B` `EXTRHEAD` do `ver0.63` após passar o portão 4-bit produz **`SD diag load failed[0].`**). Execute `panadiga unbrick verify-sd <dir>` antes do teste.
   * Com o hardware eletricamente validado, use **`SDSNIFF`** (sniffer passivo Hi-Z PIO0 em `GP10..GP15` no Pico, isolando CRC16 por linha `DAT0..DAT3` + `ACMD13`) e `scripts/bench/boot_harness.py` (`ab-boot`) para isolar defeito elétrico em `DAT1..DAT3` vs rejeição do cartão.

---

## 3. O Digital PCB (`MN2DS0009` / `VEP79138` `P59001`) Pode Operar TBC Autônomo Fora do Aparelho?

**Veredito Técnico: É possível eliminar completamente o Drive Óptico (ODD) e o HD mecânico para criar uma caixa TBC / Captura de Estado Sólido compacta, MAS o Digital PCB (`P59001`) ainda depende obrigatoriamente da Main PCB (`P7402`).**

### O Que Pode Ser Eliminado via Firmware (`MAIN3410PQ`)
Aplicando o perfil **`AV_PASSTHROUGH`** nos 3 sítios verificados de `MAIN3410PQ`:
1. **`fpc_status_ctrl` (`VA 0x5771ABFB`, offset `0x71ABFB`, 6B):** Substitui o handshake de painel frontal (`FPC`) por `mov 1, d0; ret; nop; nop` (`80 01 f0 fc cb cb`).
2. **`odd_state_branch` (`VA 0x574F7B8C`, offset `0x4F7B8C`, 4B):** Salta incondicionalmente (`jmp16 +0x00D0; nop` -> `cc d0 00 cb`) para `0x574F7C5C`.
3. **`odd_init_status` (`VA 0x574F7C5C`, offset `0x4F7C5C`, 7B):** Retorna sucesso simulado de inicialização ODD (`clr d0; nop x5` -> `80 00 cb cb cb cb cb`).
* Complementarmente, substituindo o HD PATA mecânico por adaptador **`StarTech IDE2SAT2` (`MASTER`) + CF/SSD** e removendo o teto de `200 GB` em `src/hdd_cmd.c` (`VA 0x574DD1FC`), toda a mecânica móvel desaparece — mantendo inclusive o loop **`PANARSP` ao vivo via `P7501` (`_dev_SERIAL @ 0x515B2E00`, handshake 3-byte `0e 04 02` + `0x15` NAK)** para ligar/desligar o passthrough em tempo real pela bancada Pico.

### Por Que a Main PCB (`P7402`) Ainda é Obrigatória (Diagramas `na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`)
Os diagramas em blocos da arquitetura `VEP79138` mostram que o Digital PCB (`MN2DS0009`) **não contém front-end analógico nem gerenciamento autônomo de energia**:
* **Vídeo Analógico (`IC3001` na Main PCB):** Clamps de sincronismo, filtro passa-baixa **`6M+12M LPF`** e drivers de saída **`75 Ω`** residem fora da placa digital; sem a Main PCB os ADCs/DACs do `MN2DS0009` ficam sem acoplamento/terminação padrão.
* **Áudio (`IC4001` na Main PCB):** Matriz analógica de áudio/ADC/DAC é gerida fora do bloco digital.
* **Controlador Timer/Syscon (`IC7501` na Main PCB):** Orquestra o protocolo `TBUS` de reset (`XRESET`), despertar e confirmação de fontes.
* **8+ Trilhas de Alimentação (`P7402 -> P59001`):** Múltiplas linhas de alimentação e referências trafegam pelo conector placa-a-placa `P7402 <-> P59001`.

**Conclusão Construtiva:** Remover **chassis metálico pesado + Drive DVD + HD 3.5"** mantendo apenas o **par Digital PCB + Main PCB + PSU + SSD/CF via `IDE2SAT2` + `MAIN3410PQ (AV_PASSTHROUGH)`** já liberta o circuito TBC da mecânica de 20 anos atrás de forma segura e reproduzível.

---

## 4. Guia Rápido das Novas Ferramentas Unbrick (CLI, Desktop UI & MCP)

### Linha de Comando (`panadiga unbrick`)
```bash
# 1. Diagnóstico completo do workspace (FRM, PROG061 BGO, MAIN3410PQ TBC, plano RSP, fatos hardware)
panadiga unbrick status

# 2. Extrair & decifrar módulos internos de PANA_DVD.FRM (Feistel + EXTRHEAD)
panadiga unbrick extract-frm --frm firmware_and_images/PANA_DVD.FRM --outdir unbrick_extracted --unpack

# 3. Construir Kit Completo para Cartão SD (padrão --loader-profile ver061: 256 KiB raw + PANA__SD.FRM sd48 em firmware_and_images/sd_unbrick_kit; ou --loader-profile ver063 para 128K EXTRHEAD + sd64)
# Nota: --bgo-mode auto_format (0x50401F3C) é bloqueado fail-closed e exige --allow-unsafe-bgo-format
panadiga unbrick prepare-sd --loader-profile ver061 --bgo-mode uart_monitor --passthrough-mode av_passthrough

# Alternativa manual para gerar apenas DIGADIAG.FRM bruto (ver0.61):
panadiga unbrick bgo --input firmware_and_images/extracted_partitions/prog061_dec.bin --mode uart_monitor --raw-prog061 --output firmware_and_images/sd_unbrick_kit/DIGADIAG.FRM

# 4. Auditar integridade do cartão SD (contêineres, Adler-32, list_sum_u16 & alertas -91006 / consistência ver061 vs ver063)
panadiga unbrick verify-sd firmware_and_images/sd_unbrick_kit

# 5. Inspecionar/patchar MAIN3410PQ para TBC Passthrough E-E ou gerar frames GDB RSP (_dev_SERIAL)
panadiga unbrick passthrough --input firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin
panadiga unbrick passthrough --rsp-plan --callback-va 0x50401F3C

# 6. Gerar stub MN103S somente-leitura para dump UART da NOR Flash (128K EXTRHEAD p/ ver0.63; ver0.61 exige imagem raw 262.144 B @ 0x90400000)
panadiga unbrick nor-stub --wrap-digadiag --out unbrick_out/DIGADIAG_NOR_DUMPER.FRM
```

### Interface Desktop (`Centro Unbrick`) & Servidor MCP
* **Desktop Tauri:** Abra a aba **`Centro Unbrick`** na barra lateral para inspecionar vereditos de arquitetura, extrair `.FRM`, selecionar `--loader-profile` (`ver061` padrão EH55 ou `ver063`) ao gerar/verificar o Kit SD em `firmware_and_images/sd_unbrick_kit/`, patchar `MAIN3410PQ` para TBC Passthrough, recalcular frames RSP do Pico e gerar o stub `NOR_DMP` (`ver0.63` `EXTRHEAD` ou stub raw SDRAM).
* **Servidor MCP (`panadiga mcp`):** Agentes de IA contam com a ferramenta **#18 `panadiga_sd_unbrick_kit`** (`loader_profile`, ações `status`, `extract_frm`, `prepare_sd`, `verify_sd`), além de `panadiga_bgo_unbrick` (`--raw-prog061` / `wrap_as_digadiag: false`) e `panadiga_main_passthrough`.

---

## 5. Compatibilidade Regional (`PAL-M`/`LBS`), Bloqueios Diskless de TBC, Front-End `ES10/ES15/ES35V` e Painel/IR

- **Matriz Regional `PAL-M`/`LBS` & `ifodev2` (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]`):** O bloco `ifodev2` (`2,048 B`, `SHA-256: f4f22dd0...`, destino `0x0055`) possui metadados apenas na cauda `0x400..0x7FF`. Os subsistemas **`AIOF` (`0x8800_0000`, `cps_pal_m 0x8021`)**, **`AVDD` (`0x8980_0000`)** e **`IC3001`** (`C1AB00002379` vs `1979`) governam o padrão de vídeo e compatibilidade entre regiões (`re::region_compat::tv_standard`), enquanto `hdd/capacity_patch` suporta do perfil `5 GB` (`0x0024C388` `SV0511D`) até `2 TB` e `re::tbc_appliance` cataloga os estágios de TBC sem disco.
- **Rotas Viáveis de TBC sem Disco (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]` vs `[EM INVESTIGAÇÃO / BLOQUEIO FÍSICO CH1]`):** Em `MAIN3410PQ`, `VDM_Init` e `SDM_Init` (além de `FUN_574f5c6a`/`FUN_574f469d`) impõem acoplamento ao HDD (`HDD Err -> P-OFF!`), exigindo o pipeline `Stage15DigadiagTbc` / `PANA__SD.FRM` (`PROG 0x0B1800 -> 0x90400000`) ou `Nor-FROM 0x84040000` para inicialização autônoma do TBC sem `MAIN` carregado via `CH1`.
- **Veredicto de Incompatibilidade `ES10` / `ES15` / `ES35V` vs `EH55` (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]`):** Imagens `DMR-ES10`/`ES15`/`ES35V` não são intercambiáveis com o `DMR-EH55` e causam *brick* por divergência de SoC/memória/energia: `MN2DS0009` vs `MN2DS0012-H`/`MN2DS00151DB`, EEPROM `RFKFM6016KT` vs `24LC64`, trilhas `X_SW_12V`/`DR12V`/`DR5V`, e execução XIP no `ES35V` (`0x130000..0x40FFFF`).
- **Painel Frontal `FRONT(L)` e Receptor `IR7501` (`[EVIDÊNCIA ESTÁTICA GHIDRA / ESTÁGIO 0]`):** A subplaca `FRONT(L)` expõe apenas a tecla `[POWER]`; as demais chaves residem em `KEYIN1..3` na placa principal e o receptor `IR7501` (`PNA4618M09VT`, `X_SW_5R2V`) alimenta o modo Standby `SERV` (`[CH UP]+[OPEN/CLOSE]+[REC]` por 5 s em `Power OFF`) antes da transição para `FPC_ReciveTASK`/`R_SERVICEMODE` no `MAIN` ([`HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md`](./HISTORICO_UNBRICK_EH55_BGO_CH1_DAVIS_E_REGIAO.md)).

---

## 6. Triangulação Estática Estágio 0 (`AIOF` / `IC3001` / `IC7501 M0`) & Runbook Porta #1

### 6.1 Tabela Verificada `AIOF`, Sequência de Clock e `IC3001` (`EH55_MAIN_3410PQ.bin`)

1. **Banco Completo de 82 Registradores `AIOF` (`0x8800_0000..0x8800_00B4`, descritores `{u32 name_ptr, u32 reg_off}` em `0x578935FC..0x57893888`):**
   - Controle de Modo & Clocks: `VMODESEL` (`+0x00`: `0x8020` `cps_ntsc`, `0x8021` `cps_pal_m`, `0x8027` `cps_pal`), `V` (`+0x66`: Preset `[93]` `clock_start = 0x8001`, Preset `[94]` `ddr_stop = 0x8039`), `TEST1` (`+0x6C`: Preset `[0]` `0xB040`; `AIOF_SoftReset` `@ 0x570D64E4` limpa o bit 15 `& 0x7FFF`).
   - TBC & Frame Sync: `TBCSS0..2` (`+0x1C/+0x1E/+0x20`), `TBC` (`+0x24`), `FS0..1` (`+0x2C/+0x2E`: Preset `[85]` `standard = 0x40F0`, Preset `[86]` `not_standard = 0xC0F0`), `YC3` (`+0x64`: Preset `[90]` `mvagc_line_tbc_on` bit 5 `0x0020`), `CONT` (`+0xA4`: `[80]` `ntsc_setup_lighter = 0x4000`, `[81]` `ntsc_setup_darker = 0x8000`).
   - Telemetria Somente-Leitura & Ênfase (`SSOT` corrigida em `crates/panadiga-core/src/constants/tbc_mmio.rs`): `RDACC` (`+0x80`), `RDMV` (`+0x82`), `RDDET` (`+0x84`), `RDVBID_WSS0..1` (`+0x86/+0x88`), `RDDET1` (`+0x92`), `RDFCNT` (`+0x94`) e `EMPHA` (`+0xB4`).
   - **98 Presets Estáticos:** Ponteiros `{u32 reg_off, u16 and_mask, u16 or_val, u32 extra}` indexados de `0x578957D8..0x5789595C` com strings paralelas de nome em `0x57C1F57C..0x57C1F704` (`common`, `cps_ntsc`, `cps_pal_m`, `cps_pal`, `cps_secam`, `cpn_*`, `rgb_pal`, `rec656_*`, `mvagc_line_tbc_on/off`, `clock_start`, `ddr_stop`).
2. **Endereçamento I²C de `IC3001` (`C1AB00002379`) em `0x570D627E` (`src/iic33.c`):**
   - Chamador `AVIN` carrega endereço 8-bit de escrita **`0x94`** (**7-bit `0x4A`**, leitura `0x95`) antes de chamar `0x573FF1B4` (`IIC1` `0xD840_1000` com arbitragem Timer).

### 6.2 Keepalive `M0` `IC7501 TBUS` & Runbook de Bancada (`Porta #1` + Pico + Multímetro)

* **Motor `M0` Verificado em `Mn103Vm` ([`keepalive.rs`](../../crates/panadiga-core/src/re/tbc_appliance/keepalive.rs)):**
  - `build_ic7501_wdt_255s_frame()` monta `[0x8A, 0x02, 0x00, 0xFF, 0, ..., 0, 0x30]` (`sum - 0x5B = 0x30`);
  - `emit_m0_tbus_wdt_keepalive_stub()` gera código MN103S *position-independent* (chamável via `PANAMON1>` `J 51800000` via `CALLS (A2)` $\rightarrow$ `RETS 0xF0 FC`), gravando em `0xDC01_0010..0xDC01_001F`, pulsando bit 4 em `0xDC01_0000` e retornando `D0 = 0x30`.
* **Runbook de Bancada `M0` (`NoJigPanaSdProgSlot`, apenas Pico + SD SDSC FAT16 + Multímetro):**
  1. **Pré-energização:** Confirmar `GND` Pin 4 (`0.0 Ω`), `GP3` (`NSW5V`) em `Hi-Z Input` sem pull-ups, `GP5` (`XRESET`) em Open-Drain e alimentação do Pico exclusivamente via USB (`P7501` Pin 1 `< 10 mA`).
  2. **Boot Porta #1 (sem HDD / sem JIG):** Inserir cartão SDSC (`<= 2 GiB` FAT16) com [`PANA__SD.FRM`](../../firmware_and_images/sd_unbrick_kit/PANA__SD.FRM) (`cdcc2cdf...`), manter HDD/ODD desconectados e iniciar a captura em `115200 8N1` (`COM10`). Após `HDD_MAX_LBA get failed[0xE8000000]`, o `Loader ver0.61` entra automaticamente em `PANAMON1>`.
  3. **Injeção `M0` (`0x5180_0000`):** Gravar os bytes gerados por `emit_m0_tbus_wdt_keepalive_stub()` via comando `W` em `0x5180_0000..`, executar `J 51800000` periodicamente (intervalo `< 200 s`) e confirmar no multímetro que `DR_P_ON_H` / `DI_P_ON_H` (`NSW5V ~3.29V`) e os trilhos da `VEP79138` permanecem estáveis por `> 30 min` sem corte `U99`. Consulte também [`STATUS.md`](../STATUS.md).
