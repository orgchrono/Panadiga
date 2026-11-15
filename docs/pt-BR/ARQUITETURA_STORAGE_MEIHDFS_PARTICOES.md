> 🌐 **Idioma / Language:** [English (US)](../en-US/STORAGE_ARCHITECTURE_MEIHDFS_PARTITIONS.md) | **Português (BR)**
> 🧭 **Portal:** [Índice da Documentação](../INDEX.md) | [Auditoria Completa](../../AUDITORIA_COMPLETA.md) | [Auditorias](../audits/README.md) | [Pseudocódigo Ghidra](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Logs de Debug](../debug_logs/README.md)

# ARQUITETURA DE STORAGE, FILESYSTEM MEIHDFS E PARTICIONAMENTO
## Engenharia Reversa do Disco Rígido e Firmware do Gravador Panasonic DMR-EH55
### Plataforma Matsushita UniPhier / AM34 — Chassi DIGA (PCB: VEP79138)

---

## 1. Identificação Geral do Sistema e Escopo de Engenharia de Armazenamento

O gravador digital de DVD/HDD **Panasonic DMR-EH55** (pertencente à conceituada família DIGA da Matsushita Electric Industrial Co., Ltd.) utiliza uma arquitetura de armazenamento híbrida não-convencional. Diferente de PCs ou gravadores baseados em sistemas operacionais genéricos (como Linux embarcado), o DMR-EH55 baseia-se no processador proprietário **Matsushita AM34** (núcleo RISC de 32 bits da família **MN103S**) e delega o armazenamento persistente a dois componentes complementares:

1. **Memória Flash NOR On-Board (Spansion S29JL064H — 8 MBytes TSOP-48):**
   Contém o bootloader primário (*IPL - Initial Program Loader*), rotinas mínimas de inicialização de hardware, vetores de interrupção e os blocos de parâmetros de calibração emulados em Flash (`EEMUL` / NVRAM).
2. **Disco Rígido PATA/IDE Interno (80 GB a 500 GB):**
   Não armazena apenas os vídeos do usuário. O disco rígido é um elemento ativo do subsistema de boot: **ele armazena as partições com o firmware operacional principal (AM34 executive), drivers dos CODECs de áudio/vídeo, bibliotecas de interface gráfica (OSD), microcódigos de periféricos e a infraestrutura do sistema de arquivos MEIHDFS**.

Se um disco rígido virgem (sem inicialização Panasonic) for instalado na máquina com a unidade de DVD inoperante, o aparelho trava na inicialização em autoteste com as mensagens de erro de comunicação/hardware **`HDDERR`** ou **`DRV NG`**, recusando-se a carregar o sistema operacional ou a formatar o disco pelos menus comuns.

```
                  ARQUITETURA DE INICIALIZAÇÃO HÍBRIDA DO PANASONIC DMR-EH55
 ┌────────────────────────────────────────────────────────────────────────────────────────┐
 │                              PLACA PRINCIPAL (VEP79138)                                │
 │                                                                                        │
 │   ┌──────────────────────────┐                  ┌──────────────────────────────────┐   │
 │   │  Matsushita AM34/MN103S  │   Boot Inicial   │      Spansion S29JL064H          │   │
 │   │  (UniPhier / AM34 Core)  │◄─────────────────│  (8 MB Flash NOR TSOP-48)        │   │
 │   │   108 MHz / 162 MHz      │   (Fase 1 IPL)   │  - Initial Program Loader (IPL)  │   │
 │   └────────────┬─────────────┘                  │  - Bootloader Mínimo             │   │
 │                │                                │  - NVRAM / EEMUL (Calibrações)   │   │
 │                │ Barramento IDE / ATA           └──────────────────────────────────┘   │
 │                ▼                                                                       │
 │   ┌────────────────────────────────────────────────────────────────────────────────┐   │
 │   │                        DISCO RÍGIDO (PATA / IDE / SSD)                         │   │
 │   │                                                                                │   │
 │   │   [ LBA 0 ]                Panasonic MBR Proprietário ("SH", Disk GUID)        │   │
 │   │   [ LBA 24..31 ]           Tabela de Metadados e Alocação de Baixo Nível       │   │
 │   │   [ LBA 16.384..19.199 ]   Journal Transacional (10 Blocos de 128 KB)          │   │
 │   │   [ LBA 57.344 (28 MB) ]   Tabela Espelho / Backup Idêntico ao LBA 0           │   │
 │   │   [ LBA 65.536 (32 MB) ]   Partição Firmware 1 (0x5E3250 = ~5.89 MB) (Fase 2)  │   │
 │   │   [ LBA 131.072 (64 MB) ]  Partição Firmware 2 (0xB0000 = ~704 KB) (OSD/Micro) │   │
 │   │   [ LBA 204.800+ ]         Volume MEIHDFS (VR_MOVIE.VRO / VR_MANGR.IFO)        │   │
 │   └────────────────────────────────────────────────────────────────────────────────┘   │
 └────────────────────────────────────────────────────────────────────────────────────────┘
```

### Arquivo da Imagem Analisado
* **Nome do Arquivo:** `firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin`
* **Tamanho Exato:** `104.857.600` bytes (exatamente 100,00 MB)
* **Total de Setores LBA:** `204.800` setores de 512 bytes
* **Hash MD5 Global:** `ef4453fc8651903b0a4b0c1873d950d4`
* **Hash SHA-256 Global:** `9d292d5b46f190775f359e30af11d074021fb321f040bf701d0ecf0f2056db00`

---

## 2. Mapa Geral de Memória e Estrutura de Disco (LBAs e Offsets)

A análise exaustiva setor por setor da imagem de 100 MB revelou a exata topografia de armazenamento adotada pelos engenheiros da Matsushita. O layout divide o disco em zonas de proteção, metadados transacionais, redundância em espelho e partições de código executável:

| Faixa de Setores (LBA) | Faixa de Offsets (Hexadecimal) | Tamanho (Bytes) | Tamanho (MB) | Função Primária / Descrição Técnica |
| :--- | :--- | :--- | :--- | :--- |
| **LBA 0** | `0x00000000` – `0x000001FF` | 512 B | 0,0005 MB | **Master Boot Record Panasonic:** Assinatura `"SH"`, versão 1.0 e GUID do disco. |
| **LBA 1 .. 23** | `0x00000200` – `0x00002FFF` | 11.776 B | 0,0112 MB | **Zona de Guarda 1:** Setores nulos (`0x00`) de proteção de pré-metadados. |
| **LBA 24 .. 31** | `0x00003000` – `0x00003FFF` | 4.096 B | 0,0039 MB | **Tabela de Descritores:** 8 setores com registros de 16 bytes de mapeamento físico. |
| **LBA 32 .. 16.383** | `0x00004000` – `0x007FFFFF` | 8.372.224 B | 7,9844 MB | **Zona de Guarda Intermediária:** Espaço preenchido com zeros até 8 MB. |
| **LBA 16.384** | `0x00800000` – `0x008001FF` | 512 B | 0,0005 MB | **Catálogo do Journal:** Header com contagem de blocos (`0x0A` = 10 blocos). |
| **LBA 16.385 .. 16.639** | `0x00800200` – `0x0081FFFF` | 130.560 B | 0,1245 MB | **Guarda de Pré-Alocação:** Alinhamento de limite de bloco. |
| **LBA 16.640 .. 19.199** | `0x00820000` – `0x0095FFFF` | 1.310.720 B | 1,2500 MB | **Área de Journaling Transacional:** 10 blocos de 128 KB (256 setores cada). |
| **LBA 19.200 .. 49.151** | `0x00960000` – `0x017FFFFF` | 15.335.424 B | 14,625 MB | **Zona de Guarda Pós-Journal:** Espaço preenchido com zeros até 24 MB. |
| **LBA 49.152** | `0x01800000` – `0x018001FF` | 512 B | 0,0005 MB | **Marcador de Transição de 24 MB:** Estado de formatação e limites de volume. |
| **LBA 49.153 .. 57.343** | `0x01800200` – `0x01BFFFFF` | 4.193.792 B | 3,9995 MB | **Zona de Guarda de Pré-Espelho:** Espaço preenchido com zeros até 28 MB. |
| **LBA 57.344** | `0x01C00000` – `0x01C001FF` | 512 B | 0,0005 MB | **Tabela Espelho (Mirror MBR):** Cópia idêntica bit-a-bit do LBA 0 (a 28,00 MB). |
| **LBA 57.368 .. 57.375** | `0x01C03000` – `0x01C03FFF` | 4.096 B | 0,0039 MB | **Espelho dos Descritores:** Cópia idêntica bit-a-bit dos setores 24 a 31. |
| **LBA 57.376 .. 65.535** | `0x01C04000` – `0x01FFFFFF` | 4.177.920 B | 3,9844 MB | **Zona de Guarda de Pré-Firmware:** Espaço preenchido com zeros até 32 MB. |
| **LBA 65.536 .. 77.593** | `0x02000000` – `0x025E33FF` | 6.173.696 B | 5,8877 MB | **Partição de Firmware 1:** Código principal AM34, codecs e sistema DIGA (0x5E3250). |
| **LBA 77.594 .. 131.071** | `0x025E3400` – `0x03FFFFFF` | 27.382.784 B | 26,114 MB | **Espaço Reservado Inter-Partições:** Espaço preenchido com zeros até 64 MB. |
| **LBA 131.072 .. 132.479** | `0x04000000` – `0x040AFFFF` | 720.896 B | 0,6875 MB | **Partição de Firmware 2:** Microcódigo, fontes de tela OSD e serviços (0xB0000). |
| **LBA 132.480 .. 204.799** | `0x040B0000` – `0x063FFFFF` | 37.052.416 B | 35,336 MB | **Área de Expansão dos 100 MB:** Transição final até o espaço de dados do usuário. |
| **LBA 204.800 em diante** | `0x06400000` até o fim | Restante HD | Restante | **Área do Filesystem MEIHDFS:** Extents contínuos para streaming de áudio/vídeo. |

---

## 3. Engenharia Reversa Detalhada do MBR e Setor 0 (LBA 0 & LBA 57344)

### Análise Hexadecimal do Setor 0 (Offset 0x00000000)
A leitura dos 512 bytes do LBA 0 apresenta a seguinte disposição em memória:

```text
Offset    00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F  ASCII
00000000: 00 00 00 00 01 00 53 48 62 8C 6E F5 1B 2C 36 66  ......SHb.n..,6f
00000010: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
...
000001F0: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00  ................
```

### Decodificação Estrutural de Campos

```
                      ESTRUTURA DO HEADER PROPRIETÁRIO PANASONIC (16 BYTES)
   0x00        0x03 0x04  0x05 0x06  0x07 0x08                                0x0F
  ┌────────────────┬──────────┬──────────┬────────────────────────────────────────┐
  │  00 00 00 00   │  01 00   │  53 48   │  62 8C 6E F5 1B 2C 36 66               │
  └────────────────┴──────────┴──────────┴────────────────────────────────────────┘
     Status/Flags    Versão     Tag Magic               Volume Disk GUID
      (Reservado)    Rev 1.0     ASCII "SH"         (Sessão de Formatação Única)
```

1. **Bytes `0x00 .. 0x03` (`00 00 00 00`):**
   Campo de flags de status e controle de inicialização do volume. Um valor zerado indica volume normal montável e sincronizado.
2. **Bytes `0x04 .. 0x05` (`01 00`):**
   Versão da tabela de particionamento da Panasonic em formato Little-Endian (`0x0001` = Revisão 1.0).
3. **Bytes `0x06 .. 0x07` (`53 48`):**
   Identificador mágico em caracteres ASCII: `"SH"`. Significa **Storage Header** ou **System Harddisk**. É o primeiro marcador testado pelo bootloader IPL da Spansion após emitir o comando ATA `IDENTIFY DEVICE`.
4. **Bytes `0x08 .. 0x0F` (`62 8C 6E F5 1B 2C 36 66`):**
   Identificador global único de 64 bits (*Volume Disk GUID / Formatting Session Hash*). Este valor é gerado de forma pseudo-aleatória pela CPU AM34 no momento da formatação interna pelo menu de serviço. O firmware valida se este GUID é idêntico no LBA 0 e na tabela espelho no LBA 57.344.
5. **Bytes `0x10 .. 0x1FF` (502 bytes preenchidos com `0x00`):**
   Todos os demais bytes do setor 0 são rigorosamente zerados.

### Por que o Windows e Linux não reconhecem o disco?
* **Ausência do Magic 0x55AA:** O MBR padrão de computadores IBM-PC exige os bytes `55 AA` nos offsets `0x1FE .. 0x1FF`. No Panasonic, esses bytes são `00 00`.
* **Ausência de Entradas de Partição no Offset 0x1BE:** A tabela de partição padrão MBR situa-se entre os bytes 446 e 510 (`0x01BE .. 0x01FD`), consistindo em 4 entradas de 16 bytes descrevendo CHS/LBA inicial e contagem de setores. No Panasonic, toda essa região contém zeros absolutos.
* **Diagnóstico dos Sistemas Operacionais:**
  - **Microsoft Windows:** O utilitário *Gerenciamento de Disco* (`diskmgmt.msc`) detecta a falta de assinatura `55 AA` e marca o dispositivo como **"Disco Desconhecido, Não Inicializado"**. O Windows exibe um pop-up sugerindo: *"Você deve inicializar um disco antes que o Gerenciador de Discos Lógicos possa acessá-lo (MBR ou GPT)"*. **Atenção:** Permitir a inicialização pelo Windows sobrescreve imediatamente o LBA 0, destruindo o cabeçalho `"SH"`, o que inutiliza o disco para o DMR-EH55.
  - **GNU/Linux:** O kernel emite durante o attach do dispositivo SCSI/ATA a mensagem: `sdX: unable to read partition table` ou `unknown partition table`.

### Tabela Espelho / Backup em LBA 57.344 (Offset 0x01C00000 = 28 MB)
A Panasonic implementou um mecanismo de tolerância a falhas por redundância de hardware. Exatamente no offset `0x01C00000` (29.360.128 bytes, equivalente a 28 MB ou LBA 57.344), existe um setor de backup idêntico bit-a-bit ao LBA 0.

* **Hash SHA-256 de LBA 0:** `61162c24c9240408ac807cd4307c9c5833d440715a982126c536aa10aa7530f2`
* **Hash SHA-256 de LBA 57344:** `61162c24c9240408ac807cd4307c9c5833d440715a982126c536aa10aa7530f2`
* **Status de Validação:** **Identidade estrita de 100% (512/512 bytes coincidentes)**.

Se o LBA 0 for corrompido por uma inicialização inadvertida no Windows, o firmware do DMR-EH55 é capaz de restaurar o cabeçalho original a partir de LBA 57.344 ao entrar em modo de serviço de recuperação.

---

## 4. Metadados de Alocação e Journaling Transacional (MEIHDFS Allocation Table)

### Setores 24 a 31 (0x00003000 a 0x00003FFF — 4.096 Bytes): MEIHDFS Allocation Table Encodings
A tabela de alocação MEIHDFS e os descritores de blocos gravados nos setores `LBA 24..31` (`0x00003000..0x00003FFF`) armazenam seus campos invertidos bit a bit via complemento lógico `NOT` / `XOR 0xFF` (`byte ^ 0xFF`).
Os 8 setores localizados entre LBA 24 e 31 contêm tabelas compactadas com registros de 16 bytes que mapeiam descritores de blocos de serviço do disco rígido (hard drive).

* **Arquitetura Bit-Invertida (One's Complement / XOR 0xFF):**  
  Uma característica marcante e fundamental do sistema de baixo nível da Matsushita é o armazenamento dos descritores em complemento de 1 bitwise (`byte ^ 0xFF`). A razão de engenharia para essa inversão deliberada reside no alinhamento semântico com células virgens de Flash/EPROM (onde `0xFF` representa o estado não programado ou bloco livre/não alocado).  
  Quando a tabela de 4.096 bytes é submetida ao XOR `0xFF`, emergem exatamente **256 registros estruturados de 16 bytes**, categorizados por 6 tags mestras que cobrem 100% da tabela:
  - **Tag `0x5D` (65 registros):** Extents principais, contagem de setores alocados e ponteiros LBA de blocos de áudio e vídeo.
  - **Tag `0x55` (64 registros):** Registros de sincronismo e continuação de extents fragmentados.
  - **Tag `0x57` (47 registros):** Ponteiros de LBA e vetores de endereçamento físico.
  - **Tag `0xE0` (42 registros):** Entradas de diretório e metadados de arquivos do sistema MEIHDFS.
  - **Tag `0xE1` (26 registros):** Nós virtuais (*V-Nodes*) e identificadores de inode do filesystem.
  - **Tag `0x5F` (12 registros):** Descritores de geometria de blocos e pool de mídia física.
  *(Total exato: $65 + 64 + 47 + 42 + 26 + 12 = 256\text{ registros}$ de 16 bytes).*

* **Ferramenta de Decodificação Automatizada:**  
  A suíte `panadiga` implementa a decodificação completa e instantânea dessa tabela através do comando:
  ```bash
  cargo run --release -- hdd decode-descriptors "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"
  ```

* **Espelhamento em 28 MB:** Esta tabela de 4.096 bytes é rigorosamente duplicada nos setores **LBA 57.368 a 57.375 (Offset 0x01C03000 a 0x01C03FFF)**.
  - **SHA-256 Setores 24..31:** `4b604ad2a689bd39cc8667b2479e70e304d574a78fcf72bfdeb91ca138fdce24`
  - **SHA-256 Setores 57368..57375:** `4b604ad2a689bd39cc8667b2479e70e304d574a78fcf72bfdeb91ca138fdce24`
  - **Status:** **100% idêntico bit-a-bit** com integridade confirmada.

### Catálogo e Journal de 10 Blocos (0x00800000 a 0x00960000)
A região a partir de 8 MB (`0x00800000`) atua como a área de log de transações atômicas (*Write-Ahead Journaling*) do gravador, garantindo que o sistema de arquivos sobreviva a desligamentos repentinos:

1. **LBA 16.384 (0x00800000) — Catálogo de Blocos:**
   Os primeiros 8 bytes contêm:
   - Bytes `0x00 .. 0x03`: `0A 00 00 00` (Inteiro Little-Endian de 32 bits = `10` blocos no pool de journal).
   - Bytes `0x04 .. 0x07`: `09 00 00 00` (Inteiro Little-Endian de 32 bits = índice máximo `9`, baseado em zero).
2. **Os 10 Blocos de Alocação de 128 KB (LBA 16.640 a 19.199):**
   A partir do offset `0x00820000`, sucedem-se 10 blocos consecutivos de exatamente 256 setores (128 KB = 131.072 bytes) cada. Cada bloco possui a assinatura inicial `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF`:

| Bloco | Offset Inicial | LBA Inicial | Tamanho | Header (16 Bytes Hex) | Trailer de Sequência (8 Bytes) |
| :---: | :---: | :---: | :---: | :--- | :--- |
| **0** | `0x00820000` | 16.640 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `D1 81 C8 0A 00 00 00 01` (Seq 1) |
| **1** | `0x00840000` | 16.896 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `5C FC 6E 0A 00 00 00 02` (Seq 2) |
| **2** | `0x00860000` | 17.152 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `47 06 23 0A 00 00 00 03` (Seq 3) |
| **3** | `0x00880000` | 17.408 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `71 F3 B9 0A 00 00 00 04` (Seq 4) |
| **4** | `0x008A0000` | 17.664 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `55 D5 9F 0A 00 00 00 05` (Seq 5) |
| **5** | `0x008C0000` | 17.920 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `55 54 C0 0A 00 00 00 06` (Seq 6) |
| **6** | `0x008E0000` | 18.176 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `55 57 7E 0A 00 00 00 07` (Seq 7) |
| **7** | `0x00900000` | 18.432 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `55 51 03 0A 00 00 00 08` (Seq 8) |
| **8** | `0x00920000` | 18.688 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `74 60 32 08 00 00 00 09` (Seq 9) |
| **9** | `0x00940000` | 18.944 | 128 KB | `5D FF FF FF FF FF FF FF FF FF FF FF FF FF FF FF` | `17 3F 9B 09 00 00 00 0A` (Seq 10)|

Ao final do Bloco 9 (em `0x00960000`), a região é finalizada com preenchimento em zeros limpos.

---

## 5. Engenharia das Partições de Firmware Proprietárias

### A Assinatura de Partição Universal
Ao contrário dos sistemas FAT/ext/NTFS, as partições executáveis da Matsushita no DMR-EH55 não são referenciadas por uma tabela de alocação de partição com descritores CHS/LBA no MBR. Em vez disso, **cada partição inicia-se fisicamente com uma assinatura mágica de 8 bytes de sincronização de hardware**:

$$\mathbf{\text{Panasonic Partition Magic: }}\quad \mathtt{51\ 87\ 13\ 8D\ 5B\ 0A\ FB\ 22}$$

O bootloader primário na Flash Spansion salta diretamente para os endereços fixos **`0x02000000`** e **`0x04000000`** no disco rígido, lê os primeiros 8 bytes e valida se coincidem com esta assinatura antes de carregar o binário na memória SDRAM do gravador.

```
                     ESTRUTURA DAS PARTIÇÕES DE FIRMWARE NO HDD
    0x02000000 (32 MB, LBA 65.536)
   ┌────────────────────────────────────────────────────────────────────────┐
   │ 51 87 13 8D 5B 0A FB 22 ... E5 D1 F3 4A 51 8B 26 90 (+0x38 Sync)       │
   │                                                                        │
   │ PARTIÇÃO 1: FIRMWARE PRINCIPAL & CODECS DIGA                           │
   │ Payload Útil: 0x5E3250 (6.173.264 bytes = ~5,89 MB)                    │
   │ Padding Setor: 432 bytes 0x00 até 0x025E3400 (12.058 setores LBA)     │
   │ Entropia: 7,9990 bits/byte (Binário Criptografado / Comprimido)        │
   └────────────────────────────────────────────────────────────────────────┘

    0x04000000 (64 MB, LBA 131.072)
   ┌────────────────────────────────────────────────────────────────────────┐
   │ 51 87 13 8D 5B 0A FB 22 ... E5 D1 F3 4A 51 8B 26 90 (+0x30/0x38 Sync) │
   │                                                                        │
   │ PARTIÇÃO 2: SUBSISTEMA, FONTES OSD & MICROÓDIGO                        │
   │ Payload Útil: 0xAFFD0 (720.848 bytes) + 48 bytes 0xFF (Total: 0xB0000) │
   │ Tamanho: 720.896 bytes = 704 KB (1.408 setores LBA)                    │
   │ Entropia: 7,9929 bits/byte (Binário Criptografado / Comprimido)        │
   └────────────────────────────────────────────────────────────────────────┘
```

### Partição 1: Firmware do Sistema Operacional & Codecs (Offset 0x02000000)
* **Localização Física:** Offset `0x02000000` (exatamente a 32,00 MB da mídia), correspondendo ao setor **LBA 65.536**.
* **Tamanho do Payload Útil:** `6.173.264` bytes (`0x5E3250` bytes = aproximadamente **5,887 MBytes**).
* **Tamanho Alinhado ao Setor:** `6.173.696` bytes (`0x5E3400` bytes = exatamente **12.058 setores LBA**). Os últimos 432 bytes do setor 77.593 são preenchidos com zeros (`0x00`).
* **Marcadores Internos de Sincronismo:**
  - Offset `0x00 .. 0x07`: `51 87 13 8D 5B 0A FB 22` (Assinatura de Partição).
  - Offset `0x38 .. 0x3F`: `E5 D1 F3 4A 51 8B 26 90` (Marcador de verificação de chave / integridade).
* **Entropia de Shannon:** `7,9990` bits/byte. A densidade de distribuição de bytes beira o limite teórico máximo ($8,0000$), evidenciando que o código é compactado por algoritmo de alta densidade proprietário da Matsushita ou criptografado contra engenharia reversa.
* **Hashes Criptográficos Oficiais:**
  - **Payload Útil (0x5E3250):**
    * MD5: `393e52d939eee76bbf06a98f2415253f`
    * SHA-256: `6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898`
  - **Setor Alinhado (0x5E3400):**
    * MD5: `85af47c916ac085a0b77619e7d4fe05e`
    * SHA-256: `d77fa2af9860edc694bf9fa7250514c1bf7a2e1821d69ffdac33454940c6950b`

### Partição 2: Subsistema, Fontes OSD e Microcódigo (Offset 0x04000000)
* **Localização Física:** Offset `0x04000000` (exatamente a 64,00 MB da mídia), correspondendo ao setor **LBA 131.072**.
* **Tamanho Total da Partição:** `720.896` bytes (`0xB0000` bytes = exatamente **704 KB** = **1.408 setores LBA**).
* **Tamanho dos Dados Úteis:** `720.848` bytes (`0xAFFD0` bytes), seguidos por exatamente 48 bytes (`0x30`) de preenchimento em `0xFF`.
* **Marcadores Internos de Sincronismo:**
  - Offset `0x00 .. 0x07`: `51 87 13 8D 5B 0A FB 22` (Assinatura de Partição).
  - Offset `0x30 .. 0x37`: `E5 D1 F3 4A 51 8B 26 90`.
  - Offset `0x38 .. 0x3F`: `E5 D1 F3 4A 51 8B 26 90` (Duplicado consecutivamente).
  - Trailer Final (`0xAFFC0 .. 0xAFFCF`): Padrão de fechamento `03 50 65 C8 7B B8 E0 90` repetido 2 vezes antes dos 48 bytes de `0xFF`.
* **Entropia de Shannon:** `7,9929` bits/byte.
* **Hashes Criptográficos Oficiais (0xB0000):**
  - MD5: `ee465f12930bef1599bb4fc256346ef0`
  - SHA-256: `487d5951d4aa7bc45e5f634c4ca079f369db06d5cb62e41a173575cecd68dbae`

---

## 6. Investigação Tecnológica do Filesystem MEIHDFS & Variantes UDF

### Origem e Razão Histórica do MEIHDFS
O acrônimo **MEIHDFS** designa o **Matsushita Electric Industrial Hard Disk File System**. Ele foi concebido pela Panasonic entre 2002 e 2004 para superar as limitações críticas dos sistemas de arquivos padronizados da época quando aplicados à gravação contínua de televisão em tempo real:

1. **A Falha do FAT32:** O limite intrínseco de 4 GB por arquivo impedia a gravação contínua de filmes ou programas com mais de duas horas na taxa de dados XP (10 Mbps). Além disso, a alocação baseada em tabela FAT fragmentava o disco rapidamente, causando paradas súbitas na cabeça de gravação (*seek latency*) e perda de quadros (*dropped frames*).
2. **A Inviabilidade do NTFS / ext3:** Sistemas com journaling de uso geral consomem muitos ciclos de CPU e memória RAM para gerenciar árvores B-Tree e permissões POSIX/ACL, além de não possuírem garantias de streaming em tempo real (QoS para gravação de áudio/vídeo).
3. **A Transição do UDF:** Nos primeiros modelos de DVD-RAM (como DMR-E85H), a Panasonic gravava diretamente em UDF 1.5/2.01 no disco óptico e tentou emular o mesmo formato no HD. Entretanto, o overhead de descritores de alocação curtos e a incapacidade de gerenciar discos rígidos com dezenas de gigabytes de forma performática levaram ao projeto do **MEIHDFS-V2.0** (usado no DMR-EH55 e séries subsequentes).

### Mecanismos Operacionais do MEIHDFS-V2.0

```
                     ORGANIZAÇÃO DE INODES E EXTENTS DO MEIHDFS
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │                           MEIHDFS SUPERBLOCK                                │
  │   - Número Mágico no Offset 0x0C                                            │
  │   - Ponteiro Raiz da Tabela de Inodes (`itbl`)                              │
  │   - Tamanho do Cluster Base de Extent (128 KB ou 2 MB)                     │
  └──────────────────────────────────────┬──────────────────────────────────────┘
                                         │
                                         ▼
  ┌─────────────────────────────────────────────────────────────────────────────┐
  │                     TABELA DE INODES & DIRETÓRIOS                           │
  │                                                                             │
  │   INODE 1: VR_MANGR.IFO                                                     │
  │   ├── Metadados: Tamanho, Data, Flags de Vídeo                              │
  │   └── Extent Map: LBA 204.800 (Tamanho: 64 KB)                             │
  │                                                                             │
  │   INODE 2: VR_MOVIE.VRO (Streaming MPEG-2 de Vídeo)                         │
  │   ├── Extent 0: LBA 205.056 a 409.600    (Tamanho: 100 MB contínuos)        │
  │   ├── Extent 1: LBA 800.000 a 1.200.000  (Tamanho: 200 MB contínuos)        │
  │   └── Extent N: ... (Alocação em macro-blocos sem perda de quadros)         │
  └─────────────────────────────────────────────────────────────────────────────┘
```

1. **Alocação por Extents Contínuos (*Continuous Extent Allocation*):**
   Ao iniciar a gravação de um canal de TV, o driver MEIHDFS pré-aloca blocos massivos e contíguos (macro-blocos de 128 KB, 2 MB ou 32 MB). A cabeça do disco grava o fluxo MPEG-2 de forma linear, garantindo taxa de transferência constante sem qualquer vibração ou latência mecânica.
2. **Edição Não-Linear Instantânea (NLE Sem Recópia):**
   Quando o usuário utiliza a função de edição do gravador (por exemplo, "Corte de Comerciais" / *A-B Erase* ou "Divisão de Título"):
   - O gravador **não reescreve nem move** o arquivo de vídeo de vários gigabytes.
   - O MEIHDFS apenas divide a entrada de extents no inode do arquivo, removendo o intervalo de setores LBA cortado da lista de reprodução e devolvendo esses setores ao mapa de blocos livres.
3. **Indexação de Vídeo e Áudio:**
   - **`VR_MOVIE.VRO`:** Arquivo contêiner que agrupa os fluxos de vídeo MPEG-2 (Video Elementary Stream) e áudio Dolby Digital AC-3 ou LPCM multiplexados em pacotes PES (Packetized Elementary Stream).
   - **`VR_MANGR.IFO` / `VR_MANGR.BUP`:** Tabelas de gerenciamento DVD-VR. Contêm o **Time Map (TMAP)**, que relaciona intervalos de tempo de exibição a blocos físicos chamados **VOBU (Video Object Units)**. Cada VOBU contém um pacote de navegação (*NV_PCK*) que informa o LBA do próximo quadro-chave (I-Frame), viabilizando avanço rápido e retrocesso suave.
4. **Proteção de Conteúdo e Anti-Cópia (CPRM):**
   O fato de o MEIHDFS não possuir MBR compatível com PC constitui uma barreira deliberada contra cópia pirata. O padrão CPRM (*Content Protection for Recordable Media*) exigia que transmissões protegidas com sinal *Copy-Once* fossem inacessíveis por sistemas operacionais de computadores convencionais.

### Estado da Arte da Comunidade de Engenharia Reversa
A engenharia reversa do MEIHDFS foi liderada por pesquisadores da comunidade de preservação de mídia:

* **Projeto `panasonic-rec` (`leecher1337`):**
  Disponível no GitHub, é a referência técnica aberta mais completa. O projeto desenvolveu o utilitário **`extract_meihdfs`**, capaz de abrir a imagem bruta do disco, ler o superblock do MEIHDFS-V2.0, decodificar a tabela de inodes (`itbl`) e extrair os arquivos de vídeo intactos.
* **Ferramenta `dvd-vr`:**
  Como o arquivo `VR_MOVIE.VRO` extraído muitas vezes possui descontinuidades provocadas por edições não-lineares, a ferramenta `dvd-vr` reconstrói os fluxos de áudio e vídeo e gera arquivos `.VOB` e `.MPG` padrão compatíveis com reprodutores modernos (como o VLC Media Player).
* **Soluções Periciais / Forenses:**
  Ferramentas forenses comerciais (como o **IsoBuster Pro**) incorporaram parsers proprietários para discos Panasonic MEIHDFS, permitindo a peritos judiciais e órgãos de segurança pública a extração de vídeos de vigilância registrados em aparelhos DIGA apreendidos.

---

## 7. Referência Histórica Python vs Comando Nativo Rust (`panadiga hdd`)

> [!NOTE]
> O utilitário legado `scripts/parse_panasonic_hdd_partitions.py` foi consolidado integralmente na engine nativa em Rust ([`crates/panadiga-core/src/hdd/`](../../crates/panadiga-core/src/hdd/mod.rs)) via comando `panadiga hdd inspect` / `panadiga hdd inspect-capacity`. Os exemplos históricos abaixo são mantidos apenas como registro da fase inicial de pesquisa:

```bash
# Comando atual em produção (Rust nativo):
panadiga hdd inspect -i "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"

# Comando histórico legado (descontinuado após consolidação Python -> Rust):
# python scripts/parse_panasonic_hdd_partitions.py [OPÇÕES] [CAMINHO_DA_IMAGEM]
```

### Argumentos Suportados
* `image`: Caminho para a imagem de disco (padrão: `firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin`).
* `--extract`: Habilita a extração automática das partições e setores-chave para arquivos binários separados.
* `--outdir DIR`: Define o diretório de destino dos arquivos extraídos (padrão: `firmware_and_images/extracted_partitions`).
* `--json ARQUIVO`: Exporta um relatório técnico estruturado em JSON contendo todos os dados, offsets e hashes.
* `--verbose` / `-v`: Exibe detalhes avançados da análise estrutural no console.

### Exemplo de Execução Completa
```bash
python scripts/parse_panasonic_hdd_partitions.py --extract --outdir firmware_and_images/extracted_partitions --json firmware_and_images/extracted_partitions/hdd_analysis_report.json
```

### Manifesto de Arquivos Extraídos (`SHA256SUMS.txt`)
A execução do extrator gera os seguintes artefatos técnicos no diretório de destino:

```text
61162c24c9240408ac807cd4307c9c5833d440715a982126c536aa10aa7530f2  hdd_mbr_master_lba0.bin
61162c24c9240408ac807cd4307c9c5833d440715a982126c536aa10aa7530f2  hdd_mbr_mirror_lba57344.bin
4b604ad2a689bd39cc8667b2479e70e304d574a78fcf72bfdeb91ca138fdce24  hdd_allocation_descriptors_0x00003000.bin
4c28d51cde8bcbcf2d2a86d30f4351f50bf509228fe804a09601205fa06f73a2  hdd_journal_blocks_0x00820000.bin
6ec63de059c00b287a0e8a1a4102a8007bbccf3210a552bfb45118a03b85c898  hdd_partition_firmware_0x02000000.bin
d77fa2af9860edc694bf9fa7250514c1bf7a2e1821d69ffdac33454940c6950b  hdd_partition_firmware_0x02000000_sector_aligned.bin
487d5951d4aa7bc45e5f634c4ca079f369db06d5cb62e41a173575cecd68dbae  hdd_partition_firmware_0x04000000.bin
```

---

## 8. Roteiro Prático de Substituição de Disco Rígido / SSD

Quando o disco original de um Panasonic DMR-EH55 falha mecanicamente ou quando o usuário deseja substituí-lo por um SSD (utilizando adaptadores PATA-to-SATA com chipset Marvell ou JMicron), o procedimento de restauração deve seguir rigorosamente as etapas abaixo:

```
               FLUXOGRAMA DE RESTAURAÇÃO DE DISCO RÍGIDO / SSD NO DMR-EH55
  ┌────────────────────────┐
  │  Novo HD / SSD Virgem  │
  └───────────┬────────────┘
              │
              ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ ETAPA 1: Gravação dos Primeiros 100 MB via DD no Computador            │
  │ dd if=panasonic_hdd_100.bin of=/dev/sdX bs=1M count=100 status=progress│
  │ (Cria LBA 0, Mirror a 28 MB e as Partições de Firmware a 32MB e 64MB)   │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
                                      ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ ETAPA 2: Instalação no Gravador Panasonic DMR-EH55                     │
  │ Conexão ao cabo flat IDE de 40 vias e alimentação Molex                │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
                                      ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ ETAPA 3: Boot Inicial Sem Erro "DRV NG"                                │
  │ O processador AM34 valida o "SH", carrega o Firmware 1 e Firmware 2    │
  └───────────────────────────────────┬────────────────────────────────────┘
                                      │
                                      ▼
  ┌────────────────────────────────────────────────────────────────────────┐
  │ ETAPA 4: Formatação pelo Menu de Serviço Panasonic                     │
  │ Functions ➔ To Others ➔ Setup ➔ Others ➔ HDD Management ➔ Format       │
  │ (O DMR-EH55 aloca a capacidade total do novo disco para o MEIHDFS)     │
  └────────────────────────────────────────────────────────────────────────┘
```

1. **Gravação da Imagem de Bootstrap (100 MB):**
   Conecte o disco de destino (HD ou SSD) ao computador através de um adaptador IDE/SATA para USB.
   No Linux/macOS:
   ```bash
   sudo dd if="panasonic DMR EH-55 Firmware hdd_100.bin" of=/dev/sdX bs=1M count=100 status=progress
   sudo sync
   ```
   No Windows, utilize utilitários de escrita bruta em nível de setor (como `Win32DiskImager` ou `BalenaEtcher`).
2. **Instalação no Chassi DIGA:**
   Instale a mídia na gaveta de montagem do DMR-EH55, certificando-se de configurar os jumpers do disco PATA ou da ponte SATA-PATA (`StarTech IDE2SAT2`) explicitamente em **`Master`** (`Device 0` no Canal Secundário `ch = 1` — **nunca** `CS`/`Cable Select` nem `Slave`).
3. **Inicialização do Sistema:**
   Ao ligar o aparelho, o display frontal exibirá a sequência padrão `HELLO` $\rightarrow$ `PLEASE WAIT` e entrará normalmente em modo de operação ou exibirá o aviso `UNFORMAT`. O display **não exibirá mais `DRV NG` nem `HDDERR`**, pois o MBR e as partições de firmware necessárias ao AM34 estarão presentes.
4. **Formatação Oficial do Volume de Gravação:**
   Pelo controle remoto original ou teclado frontal:
   - Pressione a tecla **FUNCTIONS**.
   - Navegue até: **To Others** $\rightarrow$ **Setup** $\rightarrow$ **Others** $\rightarrow$ **HDD Management** $\rightarrow$ **Format**.
   - Confirme a formatação. O gravador preservará intactos os primeiros 100 MB com os firmwares e alocará todos os setores restantes da mídia (seja ela de 80 GB, 160 GB, 250 GB ou 500 GB) para o novo espaço de gravação MEIHDFS.

---

## 9. Desbloqueio Dinâmico de Capacidade de HDD (`src/hdd_cmd.c` + MEIHDFS1.0)

Por padrão de fábrica, a pilha de armazenamento MEIHDFS / LBA48 do Panasonic DMR-EH55 suporta discos rígidos grandes nativamente no nível ATA, mas o módulo principal (`MAIN`) executa um truncamento artificial em software compilado a partir de **`src/hdd_cmd.c`**. Nessa rotina, o número físico de blocos de 2 KB medidos via ATA `IDENTIFY DEVICE` (`PhysicalBlocks2KB`) é comparado com um teto global em SDRAM (`MaxProductLBA`); caso o disco ultrapasse esse teto (~200 GB na variante EU/retail), o firmware reduz `PhysicalBlocks2KB` para `MaxProductLBA - 1`.

### 9.1 Anatomia Completa & Disassembly Byte-a-Byte MN103S (`src/hdd_cmd.c`)

A análise em disassembly e bytes brutos identificou o bloco básico completo de **117 bytes (`+0x00..+0x74`)**, cuja janela estrutural fixa **`WIN_LEN = 0x72` (`114` bytes)** é ancorada pelos 14 bytes iniciais `FD AA 00 8A 00 00 FB E9 00 80 C8 1D FC CC` tanto no `MAIN3060NP` quanto no `EH55_MAIN_3410PQ`:

| Campo / Símbolo | `MAIN3060NP` (`main3060np_unpacked.bin`) | `EH55_MAIN_3410PQ` (`EH55_MAIN_3410PQ.bin`) | Detalhe Técnico |
| :--- | :--- | :--- | :--- |
| **Tamanho / SHA-256 (`MAIN` raw)** | `13.427.852 B` / `075f552e...61ddcca7` | `13.003.052 B` / `c3275d39...5dc991b1` | Módulo `MAIN` descomprimido (base `0x57000000`) |
| **Endereço Inicial (`VA` / File Off.)** | `0x574E499C` (`+0x004E499C`) | `0x574DD1FC` (`+0x004DD1FC`) | Início de `WIN_LEN = 0x72` (`114` B) em `src/hdd_cmd.c` |
| **Variável Global `MaxProductLBA`** | `0x5300750C` | `0x5311B9AC` | Variável global em SDRAM com o teto em blocos 2 KB |
| **Variável Global `PhysicalBlocks2KB`** | `0x530077E0` | `0x5311BC80` | Blocos físicos LBA48 detectados (`sectors_512b / 4`) |
| **String `400GB` / `200GB`** | `0x579F9F13` / `0x579F9F2D` | `0x57A5E957` / `0x57A5E971` | `"[RTSD] M7920(HDD 400GB) \n"` / `"M7920(HDD 200GB) \n"` |
| **Log Erro `DevInfo` / `MaxLBA`** | `0x579F9EEC` / `0x579F5853` | `0x57A5E930` / `0x57A5A299` | `"[RTSD] Read DevInfo(option) Err=%08x \n"` / `"[RTSD] MaxProductLBA = %08x \n"` |

Sequência exata de instruções MN103S dentro do bloco (`+0x00..+0x74`, validada em ambos os binários):

| Offset | Bytes (`EH55_MAIN_3410PQ`) | Instrução MN103S Equivalente | Semântica para o Engenheiro de Plantão |
| :--- | :--- | :--- | :--- |
| **`+0x00..+0x03`** | `FD AA 00 8A` | `movhu (0x8a, a2), d2` | Lê halfword de opção `DevInfo(option)+0x8A`. |
| **`+0x04..+0x09`** | `00 00 FB E9 00 80` | `and 0x00008000, d2` | Testa bit `0x8000` de modelo de alta capacidade (`M7920` 400 GB). |
| **`+0x0A..+0x0B`** | `C8 1D` | `beq +29` (`-> +0x27`) | Se bit `0x8000 == 0`, salta para o slot padrão de **200 GB**. |
| **`+0x0C..+0x11`** | `FC CC 74 3B A4 0B` | `mov 0x0BA43B74, d0` | Carrega `slot_400gb` (`0x0BA43B74` = `195.312.500` × 2 KB = 400,00 GB). **`+0x0E` é o offset patcheado**. |
| **`+0x12..+0x17`** | `FC 81 AC B9 11 53` | `mov d0, (0x5311B9AC)` | Grava `MaxProductLBA` (`0x5300750C` na `3060NP`). **`+0x14` = `MaxProductLBA` VA**. |
| **`+0x18..+0x24`** | `FC CC <str> DD <disp32>` | `mov <str_400gb>, d0; call log` | Imprime `"[RTSD] M7920(HDD 400GB) \n"`. |
| **`+0x25..+0x26`** | `CA 29` | `bra +41` (`-> +0x4E`) | Salta direto para a comparação final `MaxProductLBA` vs `PhysicalBlocks2KB`. |
| **`+0x27..+0x2C`** | `FC CC BA 1D D2 05` | `mov 0x05D21DBA, d0` | Carrega `slot_200gb` (`0x05D21DBA` = `97.656.250` × 2 KB = 200,00 GB). **`+0x29` é o offset patcheado**. |
| **`+0x2D..+0x3F`** | `FC 81 <var> FC CC <str> DD <disp32>` | `mov d0, (MaxProductLBA); call log` | Grava `MaxProductLBA = 200 GB` e imprime `"[RTSD] M7920(HDD 200GB) \n"`. |
| **`+0x40..+0x41`** | `CA 0E` | `bra +14` (`-> +0x4E`) | Salta para a comparação final. |
| **`+0x42..+0x4D`** | `FC CC <fb> FC 81 <var>` | `mov <slot_fb>, d0; mov d0, (MaxProductLBA)` | Fallback se não for `M7920`: `0x05D0DDBA` (~199,83 GB em `3060NP`) / `0x04A6D7C8` (~159,83 GB em `3410PQ`). **`+0x44` é o offset patcheado**. |
| **`+0x4E..+0x53`** | `FC A5 AC B9 11 53` | `mov (MaxProductLBA), d1` | Carrega teto `MaxProductLBA` em `d1`. |
| **`+0x54..+0x59`** | `FC A4 80 BC 11 53` | `mov (PhysicalBlocks2KB), d0` | Carrega capacidade física LBA48 real do disco em `d0`. **`+0x56` = `PhysicalBlocks2KB` VA**. |
| **`+0x5A`** | `A1` | `cmp d1, d0` | Compara `d0` (`PhysicalBlocks2KB`) contra `d1` (`MaxProductLBA`). |
| **`+0x5B..+0x5C`** | **`C6 0C`** (`0xC6` -> **`0xCA`**) | **`bls +12`** (`-> +0x67`) | Se `d0 <= d1`, salta o truncamento e mantém o disco físico real. **Ao patchear `+0x5B` para `0xCA` (`bra +12`), o truncamento é ignorado para qualquer HDD**. |
| **`+0x5D..+0x64`** | `29 FF FC 85 80 BC 11 53` | `add -1, d1; mov d1, (PhysicalBlocks2KB)` | **Bloco de trava original:** se `d0 > d1`, sobrescreve `PhysicalBlocks2KB` com `MaxProductLBA - 1`. |
| **`+0x65..+0x66`** | `CA 0A` | `bra +10` (`-> +0x6F`) | Encerra o bloco após aplicar o teto. |
| **`+0x67..+0x6E`** | `28 FF FC 81 80 BC 11 53` | `add -1, d0; mov d0, (PhysicalBlocks2KB)` | **Caminho livre (desbloqueado):** ajusta `PhysicalBlocks2KB -= 1` mantendo **100% dos setores físicos do disco**. |
| **`+0x6F..+0x74`** | `FC A4 80 BC 11 53` | `mov (PhysicalBlocks2KB), d0` | Recarrega o total final para as rotinas de inicialização MEIHDFS (primeiros 3 B `FC A4 80` compõem o byte `114` de `WIN_LEN`). |

### 9.2 Estratégias de Desbloqueio (`dynamic`, `cap`, `factory_200gb`) & Fórmulas

1. **Modo Dinâmico Auto-Detect (`dynamic`, recomendado):**
   Substitui `+0x5B` de `0xC6` (`bls +12`) para **`0xCA`** (`bra +12` incondicional) e eleva `+0x0E`, `+0x29`, `+0x44` para `0x3FFFFFFF` (`~2.199 GB` em blocos de 2 KB) ou para o alvo calibrado quando `--target-gb` for informado. O sistema passa a aceitar **qualquer disco SATA/PATA LBA48 (250 GB, 320 GB, 500 GB, 1 TB, 2 TB+)** com 100% da capacidade física reportada via `IDENTIFY DEVICE`.
2. **Modo Teto Explícito / Preset RTSD (`cap`):**
   Mantém `0xC6` (`bls +12`) e crava os 3 slots de `MaxProductLBA` no limite escolhido (ex.: **`250 GB` = `0x07466528`**). Para tamanhos customizados fora da tabela oficial de 9 itens:
   $$N_{\text{blocos 2KB}} = \left\lfloor \frac{\text{Capacidade}_{\text{GB}} \times 10^9}{2048} \right\rfloor, \qquad N_{\text{setores 512B}} = N_{\text{blocos 2KB}} \times 4$$
3. **Modo Restauração (`factory_200gb`):**
   Reverte `+0x5B` para `0xC6` e restaura `slot_400gb = 0x0BA43B74`, `slot_200gb = 0x05D21DBA` e `slot_fallback = 0x05D0DDBA`.

### 9.3 Tabela Oficial RTSD `Change HDD Capacity`

Além do clamp em `src/hdd_cmd.c`, o menu de serviço **RTSD (`Change HDD Capacity`)** possui um despachante `switch/case` com 9 presets descoberto em `0x574CA054` (`MAIN3060NP`) e `0x574C267B` (`EH55_MAIN_3410PQ`), gravando a escolha em RAM `0x52FEE95C` (`3060NP`) / `0x53102DFC` (`3410PQ`) e logando `"[RTSD] MaxProductLBA = %08x \n"`:

| Item RTSD | Capacidade Comercial | Blocos 2 KB (Hex) | Blocos 2 KB (Dec) | Setores LBA 512 B (`× 4`) |
| :---: | :---: | :---: | :---: | :---: |
| `#1` | `400GB HDD (RTSD)` | `0x0BA3FB74` | `195.296.116` | `781.184.464` |
| `#2` | `300GB HDD (RTSD)` | `0x08BAEC97` | `146.467.991` | `585.871.964` |
| **`#3`** | **`250GB HDD (RTSD)`** | **`0x07466528`** | **`122.053.928`** | **`488.215.712`** |
| `#4` | `200GB HDD (RTSD)` | `0x05D1DDBA` | `97.639.866` | `390.559.464` |
| `#5` | `160GB HDD (RTSD)` | `0x04A7D7C8` | `78.108.616` | `312.434.464` |
| `#6` | `137GB HDD (RTSD)` | `0x03FC7AC3` | `66.878.147` | `267.512.588` |
| `#7` | `120GB HDD (RTSD)` | `0x037DD1D6` | `58.577.366` | `234.309.464` |
| `#8` | `100GB HDD (RTSD)` | `0x02E8CEDD` | `48.811.741` | `195.246.964` |
| `#9` | `80GB HDD (RTSD)` | `0x0253CBE4` | `39.046.116` | `156.184.464` |

### 9.4 Pipeline Criptográfico (`EXTRHEAD`/`EXTRFOOT`) & Automação na Suíte Panadiga

O módulo [`hdd/capacity_patch/`](../../crates/panadiga-core/src/hdd/capacity_patch/mod.rs) detecta automaticamente a entrada e preserva integralmente a cadeia de validação do bootloader `BOOT.BIN`:
1. **`RAW_MAIN_BINARY`**: binário `MAIN` plano descomprimido (ex.: `EH55_MAIN_3410PQ.bin`, `main3060np_unpacked.bin`).
2. **`EXTRHEAD_CONTAINER`**: partição de firmware 1 (`hdd_partition_firmware_0x02000000.bin`). O código decifra o cabeçalho de `0x40` bytes (`EXTRHEAD` + tag + `dst_addr = 0x57000000` + `build_name`), descomprime `zlib`/raw deflate, aplica o patch MN103S, recomprime, recalcula o **Adler-32 strided (`step=16`, equivalente à rotina `FUN_4401e5a3`)**, assina o rodapé **`EXTRFOOT`**, cifra em **Feistel 64-bit ECB de 16 rodadas (`PANA_FEISTEL_ROUND_KEY_32B` + `PANA_FEISTEL_SBOX_256`)** e executa round-trip completo via `unpack_extrhead` garantindo `checksum_valid == true`.
3. **`FULL_HDD_IMAGE_P1`**: imagem de 100 MB (como `panasonic DMR EH-55 Firmware hdd_100.bin` vinda do Internet Archive), regravando a Partição 1 (`0x02000000`) sem exceder o limite de `0x04000000` (`32 MB` de janela), preservando `zeros_after_header_clean` em imagens de fábrica e sincronizando os setores `total_sectors` em `LBA 0` e `LBA 57.344` apenas quando a imagem é procedural (`mkfs`).

### 9.5 Runbook de Plantão: CLI, Desktop UI & Jumper Obrigatório `MASTER` (`StarTech IDE2SAT2`)

> [!CAUTION]
> **Alerta de Hardware (`Jumper OBRIGATÓRIO em MASTER — NUNCA CS e NUNCA Slave` na ponte `StarTech IDE2SAT2` / HDD do `DMR-EH55`):** Na arquitetura real do **`DMR-EH55`** (comprovada pelos esquemáticos `vol2_schematics.pdf` pp. 46, 52, 113, 125–126 e pelo disassembly de `eh55_frm_PROG061_BOOT.bin` e `EH55_MAIN_3410PQ.bin`), o **DVD-RAM ODD** e o **HDD NÃO compartilham o mesmo cabo IDE**: o ODD opera no **Canal Primário Master (`ch=0, dev=0`, MMIO `0x89000500`)** via front-end interno (`FEP`), e o HDD opera em cabo flat dedicado de 40 vias (`Extension FFC`) no **Canal Secundário MASTER (`ch=1, dev=0`, MMIO `0x89000D00`)**. Em `FUN_50414908` (`PROG061`) e `FUN_574d1de8` (`EH55_MAIN`), a tabela de topologia fixa `acStack_70[4] = { 2, 0, 1, 0 }` (`80 02 f8 92 34 0c f8 9e 35 80 01 f8 92 36 f8 9e 37`) define exclusivamente `acStack_70[2] = 1` (**Secondary Master**) e deixa `acStack_70[3] = 0` (**Secondary Slave** vazio/ignorado — a referência `DAT_505733d4 == 0` pertence apenas ao `DMR-EH58`, não existe no `DMR-EH55`).
> - Se o jumper ficar em **`Slave` (`dev = 1`)**, o firmware do `DMR-EH55` ignora o disco em `acStack_70[3] == 0`.
> - Se o jumper ficar em **`CS` (`Cable Select`, Pino 28 `CSEL`)** na ponte `StarTech IDE2SAT2`, `DAVIS_Connect_IDE(1)` (`FUN_5041f61b` / `FUN_5041f782` / `FUN_5710c6c3`) pode sofrer seleção indeterminada de `DEV` em `0x89000D38`, colisão `0x8000040d` ou travamento de `MultiWord DMA MODE2`. Desligar o gravador durante travamento/atualização no barramento IDE em `CS` pode corromper setores lógicos da região NOR Flash **BGO (`0x84200000..0x843FFFFF`)**, levando ao erro **`BGO driver init failed[1].`** e **`U99`**.
> - **Regra de ouro no `DMR-EH55`:** configure o jumper do HDD / `StarTech IDE2SAT2` **SEMPRE explicitamente como `MASTER` (`Device 0`)** — **NUNCA use `CS` e NUNCA use `Slave`**.

Comandos CLI (`panadiga-cli`, aceitando argumento posicional ou `-i`, e `-o` opcional):
```bash
# Inspecionar ponto de clamp e slots atuais (posicional ou via -i)
panadiga hdd inspect-capacity -i firmware_and_images/extracted_partitions/hdd_partition_firmware_0x02000000.bin

# Aplicar desbloqueio dinâmico LBA48 (250 GB até 2 TB+; -o é opcional -> *_PATCHED_DYNAMIC.bin)
panadiga hdd patch-capacity -i hdd_partition_firmware_0x02000000.bin --mode dynamic

# Ou gravar teto RTSD explícito de 250 GB (0x07466528 -> *_PATCHED_250GB.bin)
panadiga hdd patch-capacity -i hdd_partition_firmware_0x02000000.bin --mode cap --target-gb 250
```

No **Panadiga Desktop (`HddStorageView`)**, a operação está disponível na aba **`Desbloqueio de Capacidade (250GB / 2TB)`** (`HddCapacityPatcherCard`), acompanhada pelo seletor de capacidade alvo (`Auto (Físico)`, `250 GB`, `300 GB`, `400 GB`, `500 GB`, `1 TB`, `2 TB` ou customizado) na aba **`Gravação Física & mkfs`** (`HddWriterCard`).

---

## 10. Conclusão Técnica

A análise de engenharia reversa do arquivo `panasonic DMR EH-55 Firmware hdd_100.bin` comprova de maneira inequívoca a arquitetura simbiótica entre a memória Flash da placa principal e o disco rígido na plataforma Panasonic DIGA. 

O disco rígido não é um mero repositório de arquivos do usuário, mas uma extensão física da memória de execução do sistema operacional AM34. A existência de cabeçalhos de redundância em espelho nos exatos 28 MB (`0x01C00000`), o catálogo transacional de 10 blocos de 128 KB em 8 MB (`0x00800000`), a partição principal de firmware de ~5,89 MB em 32 MB (`0x02000000`) e a partição secundária de 704 KB em 64 MB (`0x04000000`) constituem um sistema de armazenamento de alta sofisticação projetado para durabilidade, tolerância a quedas de energia e fidelidade absoluta na gravação audiovisual.
