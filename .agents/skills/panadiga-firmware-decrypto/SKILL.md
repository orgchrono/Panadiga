---
name: panadiga-firmware-decrypto
description: >-
  Criptoanálise e descriptografia universal de contêineres Panasonic e OEM (PANA_DVD.FRM,
  PANA__SD.FRM, DIGADIAG.FRM, PANAEUSB.FRM, .exe SFX, EXTRHEAD/EXTRFOOT, MainListHeader,
  SDBOOT/SDDL.SEC e STB CRC:), ataques ECB codebook, keyring keys.ukf (12 chaves Feistel +
  AES-128-CBC 2016/2018 + NetworkKeyIV), compressão Okumura LZSS, extração de partições
  Linux fmaX (RomFS/CramFS/ELFs) e metodologia para novos firmwares. Use ao analisar ou
  reempacotar qualquer contêiner .FRM ou investigar novas chaves criptográficas.
---

# Criptoanálise, Descriptografia Universal & Repack (`.FRM` / `EXTRHEAD`)

Esta skill unifica o motor Rust `panadiga-core::crypto` (compatível com `unixtract`, `pana-fw` e `sddl_dec`) com os fluxos de lote `scripts/re/` para analisar, extrair, patchear e reempacotar imagens de firmware.

## 1. Quando Usar & Handoffs
- Ative ao lidar com `.FRM`, `.exe` SFX de atualização, `EXTRHEAD`/`EXTRFOOT`, `MainListHeader`, partições Linux `fmaX` (`RomFS`/`CramFS`) ou ataques ECB/key-schedule.
- Após extrair binários executáveis (`PROG`, `DRV`, `MAIN`, `fmaX` ELFs), passe para `.agents/skills/panadiga-ghidra-workflow/SKILL.md`.
- Para checar autenticidade de dump físico NOR antes de decifrar, acione `.agents/skills/panadiga-hardware-forensics/SKILL.md`.
- Primitivas, chaves exatas e ataques ECB em [crypto-primitives-and-keys.md](./references/crypto-primitives-and-keys.md); layouts de contêineres, LZSS e `MainListHeader` em [container-formats-and-compression.md](./references/container-formats-and-compression.md).

## 2. Invariantes Não-Negociáveis
1. **Copy-on-Write Obrigatório:** Jamais modifique `.FRM` ou `.exe` originais *in-place*; use `--out` / `-o` separado com `--verify=true` (padrão no `frm pack`/`frm repack`).
2. **Alerta Destrutivo `--patch-format-hdd`:** Em `frm pack` e `frm repack`, a flag `--patch-format-hdd` grava `FormatFlag=1` no módulo `PROG*` e faz o gravador **formatar o HDD no alvo** — nunca habilite sem pedido explícito.
3. **Cascata de Keyring Confiável:** `FrmKeyring` resolve chaves em ordem fixa: embutidas (`12` Feistel + `2` AES pairs) -> `PANADIGA_KEY_FILE` -> `./keys.local.ukf` -> `--key-file <UKF>`.
4. **Compatibilidade OEM:** Nunca tente gravar binários DIGA/Feistel em placas Funai/Medion/Philips incompatíveis (`FUNAI_UPG`).

## 3. Workflow Canônico (CLI, Python & MCP)

### A. Triagem Criptográfica, ECB Codebook & Unpack Universal
```bash
# 1. Entropia de Shannon, vazamentos ECB 64-bit e correlação com HDD
cargo run -q -p panadiga-cli -- frm analyze firmware_and_images/PANA_DVD.FRM --hdd "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" -j -
cargo run -q -p panadiga-cli -- frm codebook firmware_and_images/PANA_DVD.FRM --hdd "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" -j -
cargo run -q -p panadiga-cli -- frm dump-toc firmware_and_images/PANA_DVD.FRM -j -

# 2. Unpack universal (Legacy DVD 48B/64B, SD Plain 48B/64B, Blu-ray Feistel/AES) + split fmaX
cargo run -q -p panadiga-cli -- frm unpack-all firmware_and_images/PANA_DVD.FRM -o tmp/unpacked --split-main -j -
cargo run -q -p panadiga-cli -- frm unpack tmp/EXTRHEAD_BLOB.bin -o tmp/plain.bin --mode auto -j -

# 3. Extrair partições Linux RomFS/CramFS ou apenas ELFs (seguro e read-only no Windows)
cargo run -q -p panadiga-cli -- frm extract-fs <FRM_OU_FMAX> -o tmp/fs_out --elf-only -j -
```

### B. Orquestração em Lote (`fleet_decryptor.py`), Retentativa & Repack
```bash
# Decriptar árvore inteira com deduplicação SHA-256 e extração de ELFs UniPhier
python -m scripts.re.fleet_decryptor --input firmware_and_images --out-dir tmp/decrypted_fleet --split-main --extract-rootfs-elfs --json tmp/fleet_report.json

# Após adicionar chave descoberta em keys.local.ukf, reprocessar apenas locked_containers:
python -m scripts.re.fleet_decryptor --retry-locked tmp/decrypted_fleet/fleet_decrypted_catalog.json --out-dir tmp/decrypted_fleet --json -

# Catalogar metadados T2 em profiles/firmware_trains/ e inspecionar plano runtime de bancada:
python -m scripts.re.frm_train_harvester --frm <PATH_FRM> --unpack-missing --json -
cargo run -q -p panadiga-cli -- re bench-runtime -i <FRM> -o tmp/bench_plan -m DMR-EH55 --json

# Repack seguro (ATENÇÃO: a flag de entrada é -i / --in)
cargo run -q -p panadiga-cli -- frm repack -i tmp/orig.FRM -o tmp/mod.FRM --patch-partition fma5=tmp/rootfs.romfs --patch-force --drop-aes --verify=true -j -
```
- **MCP:** Use `panadiga_inspect_frm` para inspeção rápida de TOC e entropia sem escrever arquivos temporários.

## 4. Armadilhas & Anti-Padrões
- Usar `--input` em `frm repack` (a flag real em `args/crypto.rs` é **`-i` / `--in`**).
- Esquecer a assimetria `0x2800` vs `0x1400` no edge-crypt de `MainListHeader` ou o offset `+0x08` do `DRV J147` (`DMR-EH50`) ou os últimos `48` bytes limpos em `DRV1`/`DRV2`.
- Confundir `algo` do `EXTRHEAD` em `panadiga-core` (`0=Zlib`, `1=Gzip`, `2=Okumura LZSS`, `3=LZMA`) com ordenações diferentes citadas em ferramentas externas.

## 5. Daqui pra Frente & Metodologia para Novos Projetos
- **Fronteira aberta (`DMR-EX97`):** Os contêineres `PANAEUSB.FRM` do `EX97` ficam registrados em `locked_containers` até a extração da chave específica a partir de `libfmupre.so` / loader ARMv7 USB, reprocessáveis com `--retry-locked`.
- **Método portátil para qualquer firmware novo:** (1) Comparar entropia do pacote vs NOR/ROM XIP (paradoxo Root-of-Trust); (2) Executar ataque KPA/ECB codebook buscando blocos `0xFF*8`/`0x00*8`; (3) Localizar S-boxes não-afins e key-schedules na ROM XIP; (4) Isolar chaves em keyring em cascata com roundtrip `--verify`.

## 6. Verificação
- Todo `frm unpack-all` valida checksums TOC (`list_sum` 16-bit ou Adler-32) e `EXTRHEAD` Adler-32; `frm repack --verify=true` confirma roundtrip sem perdas antes de encerrar.
