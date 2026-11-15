# Dossiê Técnico: Consolidação `DIGADIAG.FRM` vs `PANA__SD.FRM` / `PANA_DVD.FRM`, Bloqueio SD `-91006` e Viabilidade de TBC Autônomo (DMR-EH55 / MN103S)

> **Escopo & Lastro Forense:** Especificação baseada em desmontagem MN103S verificada de `/BOOT.BIN` (NOR `0x44000000`), `PROG061` (`0x50400000`), `EH55_MAIN_3410PQ.bin` (`0x57000000`), `MAIN3060NP` (`0x57000000`), análise empírica de `sd_card_eh55/`, `PANA_DVD.FRM`, `hdd_partition_firmware_0x02000000.bin` e esquemas oficiais `VEP79138` (`na_p49_blk1.png`, `na_p50_blk2.png`, `vol2_p125_interconnect.png`).

---

## 1. Anatomia dos Contêineres `.FRM` e Como o Gravador os Consome

Uma das maiores fontes de erro ao tentar desbrickar o Panasonic DMR-EH55 é confundir **`DIGADIAG.FRM`** com **`PANA__SD.FRM`** e **`PANA_DVD.FRM`**. Embora compartilhem a extensão `.FRM`, eles são lidos por estágios completamente diferentes da cadeia de boot e possuem formatos criptográficos distintos:

| Arquivo | Estágio Leitor | Formato de Cabeçalho / TOC | Camadas Criptográficas | Módulos / Carga Real Empírica | Uso no Unbrick & Manutenção |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`DIGADIAG.FRM`** | **Estágio 1: `/BOOT.BIN`** (`SDDIAG` na NOR `0x44000000`) | Único bloco **`EXTRHEAD`** (`131.072 B` / `0x20000` exatos, tag `"FVU "`, `dst=0x50400000`, `adler_step=4`, build `"PROG061"`) | **Plaintext** (sem Feistel no TOC nem no payload `EXTRHEAD`) | Payload único MN103S (`262.144 B` descomprimidos -> RAM `0x50400000`), ex. `PROG061` patched para BGO (`bypass` padrão) ou `NOR_DMP` | **Porta de entrada Pre-MAIN**: executa antes de `NVRAM U99` e antes do HDD/ODD, ideal para **`bypass` / `uart_monitor` (`PANAMON1`)** ou dump NOR (`auto_format` bloqueado fail-closed sem override) |
| **`PANA__SD.FRM`** | **Estágio 2: `PROG061`** (após carregado em `0x50400000`) | Tabela **TOC em texto claro** com entradas de **`64 bytes` (`0x40`)**, até 128 slots (`8.192 B` TOC), protegida por `list_sum_u16` | TOC em **texto claro**; payloads internos mantêm `EXTRHEAD` Feistel parcial (`DRV`: `0x2800`, `MAIN`: `0x19000`) | Normalmente 3 slots: `DRV S211`, `PROG061 `, `MAIN3410PQ` (ou `MAIN3060NP`), totalizando ~`7,53 MB` (`7.533.792 B`) | Atualização completa gravada na Flash NOR via SD, permitindo persistir **`MAIN3410PQ` com TBC Passthrough (`E-E`) + HDD dinâmico (`>200 GB`)** |
| **`PANA_DVD.FRM`** | **Atualização Óptica** (`CD-R` / `DVD-R` via `MAIN`/`DRV`) | Tabela **TOC de `48 bytes`** (`"Firm list"`) **cifrada em Feistel** | **Dupla camada**: Feistel completo externo em cada módulo + `EXTRHEAD` parcial interno (`DRV`/`MAIN`) | Empírico (`7.531.744 B`): `[0] DRV S211` (`0x001800`, `720.896 B` -> `1.441.792 B @ 0x90202000`), `[1] PROG061 ` (`0x0B1800`, `262.144 B` raw após outer decrypt), `[2] MAIN3060NP` (`0x0F1800`, `6.542.560 B` -> `13.427.852 B @ 0x57000000`) | Fonte oficial para extração e reconstrução de `DIGADIAG.FRM` e `PANA__SD.FRM` |

### Fluxo Completo de Consumo no Boot via Cartão SD (`BOOT.BIN` -> `PROG061` -> `MAIN`)

```text
[Cartão SD: FAT16 SDSC <= 2 GiB]
   │
   ├──► Estágio 1: /BOOT.BIN (NOR 0x44000000, rotina SDDIAG)
   │       1. Valida SD 1-bit (@ 0x44027826) -> troca para 4-bit via ACMD6 (@ 0x44027F79) -> 2ª checagem (@ 0x44027F93)
   │       2. Abre DIGADIAG.FRM (precisa ter EXATOS 131.072 bytes)
   │       3. Descomprime EXTRHEAD ("FVU ") diretamente para 0x50400000 (Adler-32) e salta para lá
   │          ├─► Modo bypass (padrão seguro): pula checagens BGO mantendo CS1 IPL/RAM intocados
   │          ├─► Modo uart_monitor (PANAMON1): ativa monitor interativo pré-MAIN (P7501 115200 8N1)
   │          ├─► Modo auto_format (BLOQUEADO fail-closed; exige --allow-unsafe-bgo-format: apaga CS1 0x84200000..0x843FFFFF / ponteiros 0x504F8464..6C)
   │          └─► Stub NOR_DMP: extrai a NOR Flash (0x84000000, 8 MiB) via UART em modo somente-leitura
   │
   └──► Estágio 2: PROG061 (em 0x50400000) -> lê PANA__SD.FRM (slots 64B em texto claro)
           └─► Grava os módulos atualizados (DRV S211, PROG061, MAIN3410PQ) com nossas modificações permanentes
```

> **Nota Crítica sobre Builds `MAIN` (`3410PQ` vs `3060NP`):**
> A partição de firmware extraída do HD (`firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin`, `6.173.264 B`) descomprime via `panadiga frm unpack --mode main` para **exatos `13.003.052 B` = `EH55_MAIN_3410PQ.bin`** (`VA 0x57000000`), onde todos os 3 sítios TBC Passthrough (`0x5771ABFB`, `0x574F7B8C`, `0x574F7C5C`) e o bloqueio LBA48 (`0x574DD1FC`) coincidem byte-a-byte. Já no contêiner óptico `PANA_DVD.FRM` e no `PANA__SD.FRM` de fábrica (`7.533.792 B`), o slot `[2]` contém **`MAIN3060NP`** (`13.427.852 B` descomprimido), onde o desvio `odd_branch` desloca para `0x4FE986`. O comando `panadiga unbrick prepare-sd` exige validação explícita de build (`--expected-main-build` / `--allow-target-mismatch`) ao gravar `MAIN3410PQ` em `PANA__SD.FRM` para impedir mismatch silencioso.

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
   * Utilize exclusivamente cartão **SDSC `<= 2 GiB`** (não SDHC >= 4 GiB) formatado em **FAT16** e execute `panadiga unbrick verify-sd <dir>` antes do teste.
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

# 3. Construir Kit Completo para Cartão SD (DIGADIAG.FRM 128K BGO bypass padrão + PANA__SD.FRM)
# Nota: --bgo-mode auto_format (0x50401F3C) é bloqueado fail-closed e exige --allow-unsafe-bgo-format
panadiga unbrick prepare-sd --outdir sd_card_eh55_kit --bgo-mode bypass --passthrough-mode av_passthrough

# 4. Auditar integridade do cartão SD (contêineres, Adler-32, list_sum_u16 & alertas -91006)
panadiga unbrick verify-sd sd_card_eh55_kit

# 5. Inspecionar/patchar MAIN3410PQ para TBC Passthrough E-E ou gerar frames GDB RSP (_dev_SERIAL)
panadiga unbrick passthrough --input firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin
panadiga unbrick passthrough --rsp-plan --callback-va 0x50401F3C

# 6. Gerar stub MN103S somente-leitura para dump UART da NOR Flash (opcionalmente encapsulado em DIGADIAG 128K)
panadiga unbrick nor-stub --wrap-digadiag --out unbrick_out/DIGADIAG_NOR_DUMPER.FRM
```

### Interface Desktop (`Centro Unbrick`) & Servidor MCP
* **Desktop Tauri:** Abra a aba **`Centro Unbrick`** na barra lateral para inspecionar vereditos de arquitetura, extrair `.FRM`, gerar e verificar o Kit SD, patchar `MAIN3410PQ` para TBC Passthrough, recalcular frames RSP do Pico e gerar o stub `NOR_DMP`.
* **Servidor MCP (`panadiga mcp`):** Agentes de IA contam com a ferramenta **#18 `panadiga_sd_unbrick_kit`** (ações `status`, `extract_frm`, `prepare_sd`, `verify_sd`), além de `panadiga_bgo_unbrick` e `panadiga_main_passthrough`.
