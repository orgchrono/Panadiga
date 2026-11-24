> 🌐 **Idioma / Language:** **Português (Brasil)** · [English Version (`en-US`)](../en-US/HARDWARE_GUIDE_STARTECH_IDE2SAT2_AND_TYPE_B_FFC_CABLE.md) · [Índice de Documentação](../INDEX.md)

# Guia de Hardware: Ponte SATA `StarTech IDE2SAT2`, Interface `P58001` e Cabo FFC `Type B` (Fechamento da Case)

> `[PROVADO EM HARDWARE REAL]` · `[EVD: EV-20261122-019 + EV-20261123-022 + P58001 + StarTech IDE2SAT2]`

Este guia documenta os requisitos elétricos, mecânicos e de firmware para substituir o disco rígido PATA original por um HDD/SSD SATA 2,5" no gravador **Panasonic DMR-EH55LB-S** (`PAL-M`, placa principal `VEP79138`, chip `M6930LB`, Flash NOR Spansion `S29JL064H` de `8 MiB`, `Loader ver0.61 [06/03/06]`) usando a ponte **`StarTech IDE2SAT2`** e o adaptador FFC **`N5EZZ0000003`**, incluindo a troca do flat original `Type A` por um **`Type B`** que permite virar a plaquinha azul em `180°` e fechar a tampa do aparelho sem forçar conectores.

---

## 1. Matriz de Compatibilidade de Pontes SATA ↔ PATA

| Ponte / Controlador | Chip Principal | Jumper Obrigatório | Comportamento no DMR-EH55 (`VEP79138`) | Veredito |
| :--- | :--- | :--- | :--- | :--- |
| **`StarTech IDE2SAT2`** | **`Marvell 88SA8052`** | **`MASTER`** | Reconhecida em hardware real com `SAMSUNG HM251JI` (`250 GB` SATA) em Ultra DMA/PIO após janela de reset compatível (`1000 ms`). | **APROVADA** |
| **Pontes genéricas `JM20330`** | `JMicron JM20330` | — | Falha na inicialização / detecção ATA do ecossistema DIGA (`Loader` / `MAIN` / temporização IDE proprietária). | **INCOMPATÍVEL / EVITAR** |

> [!IMPORTANT]
> Configure obrigatoriamente o jumper da **`StarTech IDE2SAT2`** na posição **`MASTER`** antes de encaixar no barramento IDE de 40 vias da plaquinha **`N5EZZ0000003`**.

---

## 2. Interface Física na Placa `VEP79138`: Conectores `P58001` e `P1503`

No esquemático oficial do DMR-EH55 (`manuals_and_schematics/dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`, catalogado em [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md)), a interface do HDD é dividida entre um conector plano FFC de sinalização ATA e um conector dedicado de potência DC:

| Subsistema | Conector | Pinagem / Trilhos Observados | Regras de Segurança & Medição de Bancada |
| :--- | :--- | :--- | :--- |
| **Barramento ATA (FFC 40 vias)** | **`P58001`** (`VEP79138`) ↔ **`N5EZZ0000003`** | Mapeamento espelhado **`Pin k = ATA(41 - k)`** | Sinais LVTTL `3.3 V`. Leituras DC de bancada medidas em repouso/operação: **`IORDY = 3.34 V`**, linhas de controle em **`2.45–2.47 V`**. |
| **Diagnóstico de Orientação** | **`P58001`** (`0°` vs `180°`) | Máscara diagnóstica **`0x00002004`** | Inserção invertida/espelhada de `P58001` (`Pin k` trocado por `41 - k`) impede detecção do disco e acusa assinatura de erro `0x00002004`. |
| **Alimentação DC do Drive** | **`P1503`** (`VEP79138`) | Trilhos **`DR5V`** (`+5 V`) e **`DR12V`** (`+12 V`) | Confirme polaridade e tensão de **`DR5V` / `DR12V`** antes de ligar; discos SATA 2,5" + `IDE2SAT2` alimentam-se apenas pelo ramal de `+5 V` via chicote floppy/Molex da ponte. |

Consulte também o diagrama de bancada geral em [`BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md`](BLUEPRINT_ESQUEMA_ELETRICO_BANCADA.md) e a estrutura lógica do disco em [`ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md`](ARQUITETURA_STORAGE_MEIHDFS_PARTICOES.md).

---

## 3. Dica Crítica de Montagem: Cabo FFC `Type B` para Virar a `N5EZZ0000003` e Fechar a Case

### O problema mecânico com o cabo de fábrica (`Type A`, `20,5 mm`)
O flat FFC original da Panasonic para conectar `P58001` à plaquinha adaptadora azul **`N5EZZ0000003`** (códigos de serviço **`VEK0K00`** / **`VEK0K15`**) possui especificação:
- **40 vias (`40-pin`)**, passo **`0,5 mm`**, norma **`UL AWM 20624`**
- Comprimento muito curto: **`20,5 mm`**
- Configuração **`Type A`** (contatos metálicos expostos **do mesmo lado** nas duas extremidades)

Quando a **`StarTech IDE2SAT2`** é encaixada diretamente no conector IDE macho de 40 pinos da plaquinha azul **`N5EZZ0000003`** sobre esse FFC de apenas `20,5 mm`, o conjunto fica alto/travado na orientação original e **impede o fechamento da tampa metálica superior da case do DMR-EH55**.

### A solução prática validada: FFC `Type B` (`100–150 mm`) + Giro de `180°` da Plaquinha Azul

| Parâmetro do Cabo | Cabo Original de Fábrica | Cabo Recomendado para Fechar a Case |
| :--- | :--- | :--- |
| **Pinos & Passo** | `40-pin`, `0,5 mm` (`UL AWM 20624`) | `40-pin`, **`0,5 mm`** (`UL AWM 20624` ou equivalente `60 V` / `80 °C+`) |
| **Tipo de Contatos** | **`Type A`** (contatos no mesmo lado) | **`Type B`** (contatos em **lados opostos** nas duas pontas) |
| **Comprimento** | `20,5 mm` (`VEK0K00` / `VEK0K15`) | **`100–150 mm`** |
| **Efeito Mecânico na `N5EZZ0000003`** | Mantém plaquinha presa próxima a `P58001` | Permite **virar a plaquinha azul `N5EZZ0000003` em `180°`**, deitando o conjunto `IDE2SAT2 + drive 2,5"` com folga dentro do chassi para **fechar completamente a tampa do DMR-EH55** |

> [!CAUTION]
> **CHECAGEM OBRIGATÓRIA `1:1` COM MULTÍMETRO ANTES DE ENERGIZAR:**
> Jamais confie apenas na descrição comercial *"Type A"* ou *"Type B"* do vendedor ou na aparência da trava ZIF. Como o cabo **`Type B`** tem contatos em lados opostos e você estará **virando a plaquinha azul `N5EZZ0000003`**, um erro de inserção (ou cabo invertido no lote) espelha todos os `40` pinos (`Pin k` ↔ `41 - k`), causando falha **`0x00002004`** ou risco de curto entre trilhos ATA e terra.
>
> **Procedimento obrigatório com o aparelho fora da tomada:**
> 1. Encaixe o novo FFC **`Type B`** (`100–150 mm`) em `P58001` (`VEP79138`) e na `N5EZZ0000003` já posicionada invertida.
> 2. Com o multímetro em modo continuidade (**beep**), confira diretamente pino a pino que cada **`Pin k`** em `P58001` corresponde exatamente a **`ATA(41 - k)`** no header IDE de 40 pinos da plaquinha `N5EZZ0000003` (mapeamento **`1:1`** idêntico ao obtido com o conjunto original `VEK0K00 Type A`, apenas com a plaquinha rotacionada `180°` graças à inversão de face dos contatos `Type B`).
> 3. Confirme também isolamento entre trilhos adjacentes e terra (`GND`) antes de conectar o chicote de força `P1503` e ligar a alimentação AC.

---

## 4. Temporização de Reset da `Marvell 88SA8052` & Correção de Firmware

Mesmo com a fiação elétrica `1:1` validada, a ponte **`Marvell 88SA8052`** apresenta uma característica temporal crítica documentada em bancada:

1. **Tempos medidos pós-reset ATA no `88SA8052`:**
   - Afirmação inicial de **`BSY`**: **`~32 ms`**
   - Liberação para **`DRDY`**: **`~555 ms`**
2. **Incompatibilidade com a espera stock (`~2 ms`):**
   - No firmware **`MAIN`** original (`MAIN3060NP` e versão brasileira `PAL-M` `MAIN3410PQ`), o atraso após o pulso de reset do disco rígido é de apenas **`~2 ms`**, muito abaixo dos **`~555 ms`** exigidos pela ponte SATA/IDE antes de aceitar comandos de identificação e leitura de setores.
3. **Correção determinística implementada (`1000 ms`):**
   - O módulo [`sata_bridge_fix.rs`](../../crates/panadiga-core/src/re/sata_bridge_fix.rs) substitui a espera curta por uma chamada segura de **`1000 ms`** (`CALL rtsd_sleep_10ms(100)`), validada tanto para `MAIN3060NP` quanto para o firmware brasileiro **`MAIN3410PQ`** (`VA 0x5710A9D0 -> CALL 0x574DF538`).
   - Os detalhes completos de engenharia reversa, retarget `4 MiB -> 8 MiB` BGO e os 5 Portões de Validação estão separados no dossiê técnico [`PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md`](PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md).

---

## 5. Evidência Comprovada em Hardware Real (`SAMSUNG HM251JI`)

A combinação física **`VEP79138` (`P58001` / `P1503`) + FFC `40-pin` validado `1:1` + `N5EZZ0000003` + `StarTech IDE2SAT2` (`MASTER`) + `SAMSUNG HM251JI` (`250 GB` SATA)** foi validada em bancada real no DMR-EH55 via UART `P7501` (`COM10`, `115200 8N1` — ver [`GUIA_BANCADA_P7501_SEM_DESSOLDA.md`](GUIA_BANCADA_P7501_SEM_DESSOLDA.md)):

- **Portão 2 (boot SD com ponte SATA ativa):** [`boottrace_sata_bridge_gate2_20261122.log`](../evidence/boottrace_sata_bridge_gate2_20261122.log) (`7.536 B`, SHA-256 `f8c1b5699986334f1a97ef004319eaeef880b3447dc4d80d04c1db82c92d6865`) e teste sem SD [`boottrace_sata_bridge_nosd_only_20261122.log`](../evidence/boottrace_sata_bridge_nosd_only_20261122.log) (`9.021 B`, SHA-256 `3120123e29115694e99b51eb1cf120daeb346683b36fc5ad5091026b2a34ac2e`).
- **Portão 3 (`2026-11-23` — Gravação nos espelhos MEIHFS e boot direto pelo disco SATA):**
  - **Boot 1 (`FVU)` + gravação nos dois espelhos):** [`boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot1_fvu_20261123.log) (`4.906 B`, SHA-256 `455a86083636cf4bcc2c49d33ffd55be1391e6e9c12edb7ebd15791dbdacd2d3`, `check_sum = 0x32836a32`, `Vefify OK` × 2).
  - **Boot 2 (Cold boot direto do `SAMSUNG HM251JI` atrás da `StarTech IDE2SAT2`):** [`boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log`](../evidence/boottrace_sata_bridge_gate3_8mb_boot2_main_20261123.log) (`55.917 B`, SHA-256 `acbee64c1710e015dc32f707bff4acd3ce25cb1224575ab38753a7b3ff95595a`, `39.652` bytes de telemetria `PiE-OS / 3060NP`, detecção limpa de **`SAMSUNG HM251JI`**, `M6930LB`, visor **`TM LI`**).

---

## 6. Referências Cruzadas

- **Pesquisa Técnica dos Portões 0..4 e Investigação (~10h):** [`PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md`](PESQUISA_PORTOES_VALIDACAO_SATA_STAGE2_E_RETARGET_BGO_8MB.md)
- **Manual de Serviço & Esquemáticos Vol. 2:** [`manuals_and_schematics/README.md`](../../manuals_and_schematics/README.md) (`dmr-eh55/service/panasonic_dmr_eh55ec_service_manual_vol2_schematics.pdf`)
- **Status do Projeto & Ledger Forense:** [`STATUS.md`](../STATUS.md) · [`ledger.json`](../evidence/ledger.json)
