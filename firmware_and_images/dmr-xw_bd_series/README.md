# Acervo Oficial — Séries HD DIGA (`DMR-XW300`) e Blu-ray DIGA (`DMR-BS` / `DMR-XS`)

Coleção de pacotes oficiais baixados diretamente do CDN oficial Panasonic (`fdl.jpn.support.panasonic.com`), extraídos sem mutação dos originais e verificados criptograficamente por SHA-256 em [`SHA256SUMS.txt`](SHA256SUMS.txt).

> **Nota:** Conforme [`.gitignore`](../../.gitignore), binários proprietários (`*.exe`, `*.FRM`) não são versionados diretamente no Git; apenas metadados forenses, hashes SHA-256 e resultados de TOC/payload são versionados aqui e refletidos nos perfis `profiles/firmware_trains/`.

## Inventário dos Pacotes Baixados e Extraídos

| Subdiretório | Modelos Cobertos | Instalador `.exe` (Bytes / SHA-256) | `PANA_DVD.FRM` (Bytes / SHA-256) | Cifra & Layout TOC | Módulos Extraídos / Verificados |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `xw300gn/` | `DMR-XW300GN` | `UPDATE_XW300GN_V17.exe`<br>`30,273,386 B`<br>`fc4eec4833de8ba9a0d12e63deb3ced172858961886e1d488288a5349e3abdd7` | `33,290,728 B`<br>`193fd4377a1312b4a46aabd60e2449c7349b18c1ed08b1fee228b7befa1c2633` | `le2011_08e0` (`Dvd48B`, offset `0x0`) | `PROG0.00` (`0x1800`, `262,144 B`, byte-sum `0x01032A66`, vetor MN103S `CA 10 ... FC DC`), `MAIN3700` (`0x41800`, `32,333,836 B`), `DRV1J315` (`0x1F17810`, `688,600 B`, byte-sum `0x053CD831`), `ver=M4220GN`, `grp=BQ-370` |
| `bs850_bs750_xs350eg/` | `DMR-BS750EG`, `DMR-BS850EG`, `DMR-XS350EG` | `UPDATE_BS850_BS750_XS350EG_V16.exe`<br>`45,148,760 B`<br>`3d88bd64635147599f30320948b5e0c9f79f814049cb60fdf51bff5d6cdac092` | `45,049,232 B`<br>`a23cc1288222b2816fb1328c24613686ec890b775c9b44daa91085d56d8f913d` | `le2011_08e0` (`Dvd48B`, offset `0x0`) | `PROG0.00` (`0x1800`, `163,840 B`, `zlib.adler32=0xA5A52CCB`, MN103S/UniPhier PH1 `[SWUP] NAND FLASH`), `MAIN3600` (`0x29800`, `44,879,244 B`), `ver=M3765EG`, `grp=QG-360` |
| `bs880_bs780_xs380eb/` | `DMR-BS780EB`, `DMR-BS880EB`, `DMR-XS380EB` | `UPDATE_BS880_BS780_XS380EB_V141.exe`<br>`43,934,184 B`<br>`07e211977c44ac0cf76f299278b92904af9b37c289680ce8f280bad8db65302e` | `43,836,424 B`<br>`747b607f61dc20a180beac78969cbf2c32743e7636e269110d27a6033f351468` | `le2011_08e0` (`Dvd48B`, offset `0x0`) | `PROG0.00` (`0x1800`, `163,840 B`, `zlib.adler32=0xA4F374FC`, MN103S/UniPhier PH1 `[SWUP] NAND FLASH`), `MAIN3410` (`0x29800`, `43,666,436 B`), `ver=M2863EB`, `grp=QD-341` |
| `bs885_bs785_xs385eg/` | `DMR-BS785EG`, `DMR-BS885EG`, `DMR-XS385EG` | `UPDATE_BS885_BS785_XS385EG_V140.exe`<br>`40,662,216 B`<br>`de55de00aeed76d95361fb84e70d62e82a33e84c2a75e852e3eedda9023cdee6` | `40,562,688 B`<br>`222f213c234c5ec17ba1e026ee6fe9575eade3b8557f257a73b80740d9b30b21` | Header externo `PANASONIC\0\0\0` (48 B) + TOC `le2011_08e0` em `0x30` (offsets relativos ao arquivo) | `PROG0.00` (`0x2000`, `163,840 B`, `zlib.adler32=0x609912FF`, MN103S/UniPhier PH1 `[SWUP] NAND FLASH`), `MAIN3400` (`0x2A000`, `40,386,676 B`), `ver=M2863EG`, `grp=ZE-340` |

## Descobertas Forenses Relevantes

1. **Transição NOR -> NAND Flash (UniPhier PH1):** Os módulos `PROG0.00` descriptografados de `DMR-XW300`, `BS750/BS850/XS350`, `BS780/BS880/XS380` e `BS785/BS885/XS385` mantêm o vetor de reset **Matsushita MN103S** (`0xCA 0x10 ... 0xFC 0xDC`), porém executam rotinas `[SWUP] NAND FLASH` / `[SWUP] SecondBoot Block` (`nand_base.c`) — confirmando arquitetura baseada em **NAND Flash** no lugar da NOR TSOP-48 (`S29JL064H`) utilizada nos gravadores DVD/HDD da geração DMR-EH55/EX.
2. **Envelope `PANASONIC\0\0\0` (`bs885`):** O pacote do `DMR-BS785/885/XS385EG` antepõe 48 bytes plaintext iniciando em `PANASONIC\0\0\0` antes da tabela TOC cifrada por `le2011_08e0`; entretanto, os campos `module_offset` dentro dos descritores TOC (`0x2000`, `0x2A000`) permanecem endereçados a partir do byte `0` do arquivo `PANA_DVD.FRM`.
3. **Evolução do Checksum TOC:** Enquanto `xw300gn` usa soma de bytes de 32 bits (`sum(mod) & 0xFFFFFFFF`), `bs850`, `bs880` e `bs885` adotam `zlib.adler32` padrão nos módulos `PROG0.00`.
