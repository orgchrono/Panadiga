> 🌐 **Idioma / Language:** [English (US)](../en-US/FORENSIC_AUDIT_REPORT_CRYPTANALYSIS_AND_MEIHDFS.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# PARECER CONCLUSIVO DE AUDITORIA FORENSE E CRIPTOANÁLISE DE FIRMWARE EMBARCADO
**Alvo:** Ecossistema Óptico (`PANA_DVD.FRM`) e Armazenamento MEIHDFS (`hdd_100.bin`) — Panasonic DIGA DMR-EH55  
**Base de Verdade Auditada (SSOT):** [`crates/panadiga-core/src/constants.rs`](../../crates/panadiga-core/src/constants.rs), [`crates/panadiga-core/src/crypto/toc.rs`](../../crates/panadiga-core/src/crypto/toc.rs), [`crates/panadiga-core/src/crypto/codebook.rs`](../../crates/panadiga-core/src/crypto/codebook.rs) e [`crates/panadiga-core/src/hdd/meihdfs.rs`](../../crates/panadiga-core/src/hdd/meihdfs.rs).

---

## 0. Preliminar de Auditoria: Retificação de Evidências (Fact-Check Binário)

| Item Auditado | Realidade Física no Binário (SSOT Auditado) | Referência no Código |
| :--- | :--- | :--- |
| **1. Tamanho do `PANA_DVD.FRM`** | **`7.531.744 bytes`** ($7{,}18\text{ MB}$ / $941.468$ blocos de 8 bytes). | `dump_toc` |
| **2. Assinatura em `0x00000000` (`.FRM`)** | **100% cifrado desde o byte `0x00`.** Os 16 bytes iniciais são `49 2F 05 60 CE DE 50 8C E3 C0 8A FD 49 B6 4A 9A`. | `PANA_FRM_GLOBAL_HEADER` |
| **3. Geometria da TOC (`.FRM`)** | Offset **`0x0010..0x009F`** ($144\text{ bytes}$) com **3 entradas periódicas de 48 bytes** (`0x30`): AM34 OS, Servo MN102 e Front Panel MN103S. Carga útil inicia em `0x00A0`. | `PANA_FRM_TOC_ENTRY_SIZE` |
| **4. Entropia de Cabeçalhos (`.FRM`)** | Quedas locais de entropia ($H \approx 2{,}8\text{–}3{,}0\text{ bits/byte}$) ocorrem nas zonas de **padding ECB** onde o mesmo bloco cifrado de 8 bytes se repete milhares de vezes. | `ECB_VIRGIN_FLASH_8B` |
| **5. Offset da Assinatura `"SH"` (LBA 0)** | `0x00..0x03` = `00 00 00 00`, `0x04..0x05` = `01 00` (Rev 1.0), **`0x06..0x07` = `53 48` (`"SH"`)** e **`0x08..0x0F` = GUID** (`62 8C 6E F5 1B 2C 36 66`). | `PANASONIC_HDD_MBR_FULL_SIGNATURE` |
| **6. Estado das Partições 1 e 2 no HDD** | Ambas as partições (`0x02000000` e `0x04000000`) iniciam com o cabeçalho `51 87 13 8D 5B 0A FB 22` e **estão cifradas em ECB de 64 bits** no HDD com a mesma chave do `.FRM`. Além disso, os descritores em `0x00003000` são gravados invertidos em complemento de um (`XOR 0xFF`). | `PANASONIC_HDD_PARTITION_MAGIC` |

---

## 1. Validação da Criptoanálise do `.FRM` e Salvaguardas de Padding

1. **Quebra Estrutural vs. Recuperação de Chave:** O vazamento ECB combinado ao KPA quebra a ofuscação estrutural do contêiner óptico, mas não recupera a chave simétrica sobre $2^{64}$ blocos. A decriptação integral deve extrair a rotina em texto-plano do IPL da Flash NOR (`0x80000000..0x807EFFFF`) a partir de um dump físico do chip `IC5001` ou via interface serial `P7501` (`115200 8N1` / modo JIG/loader).
2. **Proibição de Despadding PKCS#7:** O firmware Matsushita utiliza padding por blocos constantes (`0xFF * 8` e `0x00 * 8`), sem PKCS#7. O recorte obedece a limites estritos de comprimento de carga útil.

---

## 2. Invariante da Cadeia de Boot em Dois Estágios (`mkfs.meihdfs` e Prevenção do `U99`)

- A síntese procedural gera com perfeição o LBA 0 (`"SH"`), descritores invertidos LBA 24..31 (`XOR 0xFF`), journal e espelhos.
- **Invariante de Estágio 2:** O IPL da Flash NOR exige que a Partição 1 (`0x02000000`) contenha o kernel AM34 descriptografável com hash válido. Se não provisionada via `--inject-frm`, o boot inicial exigirá a inserção da mídia óptica de serviço `PANA_DVD.FRM` no gravador para evitar o erro fatal `U99`.

---

## 3. Invariante de Recuperação Dupla Atômica

- Em desastres causados por PC (sobrescrita de MBR/GPT nos LBAs 0..33), os descritores em LBA 24..31 também são danificados.
- A recuperação exige restauração atômica dupla:
  - Superbloco MBR: `0x01C00000` (LBA 57344) $\longrightarrow$ `0x00000000` (LBA 0)
  - Tabela de Descritores: `0x01C03000` (LBA 57368) $\longrightarrow$ `0x00003000` (LBA 24..31)
- Preserva 100% dos títulos gravados na Partição 3 sem qualquer perda de dados.

---

## 4. Veredito Final de Viabilidade de Bancada & Protocolo "What's Next"

1. **Camada Física P7501 & Controle Discreto:** Os esquemáticos oficiais do DMR-EH55 comprovam que o conector `P7501` implementa **UART assíncrona `115200 8N1`** com controle discreto em `SW_UART` (Pin 1) e `XRESET` em Open-Drain (Pin 2), mantendo `NSW5V` (Pin 5) estritamente como entrada `Hi-Z` — sem interface síncrona proprietária DWire32A/JTAG exposta nesse conector.
2. **Cura do `U99` & Captura Ouro de 8 MiB `IC5001` (Concluída em Bancada):** O fluxo `JIG:2500` $\rightarrow$ `PANAMON1>` + `PANADMP1` (`0x9043950E` *uncached* $\rightarrow$ `J 904394FA`) capturou [`eh55_nor_8mb_ic5001_dump.bin`](../../firmware_and_images/README.md) (`8.388.608 B`, `SHA-256: c1faee798f60545853eefdba0d15f23678b471466dc2bc84f522bc972f15a02d`), `eh55_loader_ver061_256k.bin` (`262.144 B`, `9f5c448d...`) e `eh55_nvram_64k_raw.bin` (`65.536 B`, `803d1465...`), mantendo intactos os `256 KiB` iniciais de IPL (`SA0..SA10` / política `SA0..SA3` `E05`). O reparo cirúrgico no FTL BGO real do `ver0.61` (`SA26..SA70` em `0x84130000..0x843FFFFF`, separado do Upper EEMUL `SA71..SA141`: formatação de `SA70` + `1.629` escritas puras `1->0` em 13 spares `SA42..SA66` + `6` bytes em `SA41+0x14`) eliminou `BGO driver init failed[1].` (`U99`) e restabeleceu `BOOTTRACE 15000 NOJIG` até `***** normal mode *****` $\rightarrow$ `*** RTSC Initialize ***`.
3. **Preservação Criptográfica & Chaves:** Os artefatos originais (`hdd_100.bin`, `PANA_DVD.FRM`, `eh55_nor_8mb_ic5001_dump.bin`) permanecem imutáveis com somas SHA-256 verificadas, além da tabela BGO `0x440337A4` e rotina de decriptação extraídas do IPL em texto-plano.
4. **Prontidão de Bancada:** Todos os 9 Quality Gates foram validados e o sistema está aprovado para operação física com a sonda RP2040/RP2350.
