---
name: panadiga-extract-partitions
description: >-
  Análise forense, extração de partições, patch de capacidade (>200 GB até 2 TB/4 TiB) e
  gravação segura de discos Panasonic DIGA (MEIHFS-V2.0/V2.1, MBR 'SH', espelhos,
  MAIN_LBA) com lockouts multi-SO e diagnóstico físico de cabo P58001/IDE2SAT2. Use ao
  inspecionar dumps HDD, liberar capacidade acima de 200 GB ou gravar HDDs físicos.
---

# Extração, Patch de Capacidade & Gravação Forense MEIHFS

Esta skill cobre a engenharia forense do sistema de arquivos **MEIHFS-V2.0/V2.1**, o desbloqueio de capacidade no `MAIN`, a gravação segura multi-SO em discos físicos e o diagnóstico da ponte `P58001`.

## 1. Quando Usar & Handoffs
- Ative para inspecionar dumps de HDD (`hdd_100.bin`), extrair partições `32 MB`/`64 MB`, liberar discos `>200 GB` (`250 GB` RTSD = `0x07466528`) ou gravar unidades físicas.
- Para diagnóstico vivo via `PANAMON1>` no canal `DAVIS CH1`, combine com `.agents/skills/panadiga-bench-operations/SKILL.md` (`pico davis-probe`).
- Para triagem de outros chips/mídias ou custódia, use `.agents/skills/panadiga-hardware-forensics/SKILL.md` e `.agents/skills/panadiga-forensic-archival/SKILL.md`.

## 2. Invariantes Não-Negociáveis
1. **Dumps Imutáveis:** Nunca edite imagens brutas originais (*in-place*); gere cópias novas e valide `SHA256SUMS.txt`.
2. **Lockout Multi-SO Obrigatório:**
   - **Windows:** `\\.\PhysicalDriveN` com token `CONFIRM-PhysicalDriveN` (Admin). `PhysicalDrive0` e discos `BusTypeNvme (17)` são **bloqueados permanentemente**.
   - **Linux:** `/dev/sdX` ou `/dev/mmcblkN` com `CONFIRM-/dev/sdX` (root/`disk`); discos raiz via `/proc/mounts` + `/dev/mapper` e `/dev/sda*` são bloqueados.
   - **macOS:** `/dev/diskN` ou `/dev/rdiskN` com `CONFIRM-/dev/diskN` (`sudo`); `/dev/disk0*` e `/dev/rdisk0*` são bloqueados incondicionalmente.
3. **Backup CoW & Readback SHA-256:** `hdd flash-eh55` e `hdd flash-composed` exigem `--backup-dir` (salva 64 KiB + espelhos), `Mn103Vm` pre-write gate e conferem readback SHA-256.
4. **Lei Física `P58001`:** O conector FFC de 40 vias do EH55 inverte a ordem ATA: **`Pin_P58001(k) = ATA(41 - k)`**.

## 3. Workflow Canônico (CLI & MCP)

### A. Inspeção Read-Only, Extração & Desbloqueio de Capacidade
```bash
# Inspecionar MBR 'SH', descritores LBA 24..31 e mapa de densidade
cargo run -q -p panadiga-cli -- hdd inspect "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --density -j -

# Extrair MBR, espelhos, journal e partições 32MB/64MB + SHA256SUMS.txt
cargo run -q -p panadiga-cli -- hdd extract "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --outdir tmp/extracted_partitions -j -

# Decodificar descritores XOR 0xFF (One's Complement) e journal
cargo run -q -p panadiga-cli -- hdd decode-descriptors "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --journal -j -

# Auditar e patchear limite MaxProductLBA (5 GB .. 2 TB, teto 4 TiB; 3410PQ @ 0x574DD1FC / 3060NP @ 0x574E499C)
cargo run -q -p panadiga-cli -- hdd inspect-capacity -i tmp/MAIN.bin -j -
cargo run -q -p panadiga-cli -- hdd patch-capacity -i tmp/MAIN.bin -o tmp/MAIN_250gb.bin -m dynamic -g 250
```

### B. Descoberta & Gravação Segura em Unidade Física
```bash
# Listar unidades e papel (hdd | sd | system | all) + simulação read-only via MCP panadiga_hdd_flash_inspect
cargo run -q -p panadiga-cli -- hdd list-drives --role hdd
cargo run -q -p panadiga-cli -- hdd list-sd-cards

# Gravação verificada EH55 (imagem 100 MiB + espelho tail MAIN_LBA = ((disk_bytes / 512) & !3) - 0x20000)
cargo run -q -p panadiga-cli -- hdd flash-eh55 \
  --drive "<DEVICE_PATH>" \
  --image "<EH55_100M_IMG>" \
  --backup-dir "<BACKUP_DIR>" \
  --confirm-physical-drive "<CONFIRM_TOKEN>" \
  -j -

# Outras operações protegidas: hdd format-meihdfs --dry-run | hdd write-image | hdd restore-mirrors | hdd flash-composed
```
- **MCP:** `panadiga_inspect_hdd`, `panadiga_hdd_capacity`, `panadiga_hdd_flash_inspect` (strictly dry-run).

## 4. Geometria MEIHFS & Descobertas de Hardware (`P58001` / `250 GB`)
- **Offets HDD EH55:** `LBA 0` (`0x000000` MBR `"SH"` + Disk GUID, `[16:512]=0`), `LBA 24..31` (descritores `XOR 0xFF`), `LBA 16384` (`8 MB` journal 10 blocos), `LBA 57344` & `57368` (`28 MB` espelhos), `0x02000000` (`32 MB`) & `0x04000000` (`64 MB`, assinatura `51 87 13 8D 5B 0A FB 22`), e espelho tail `MAIN_LBA` (ex.: `LBA 488266096` no HDD `250 GB` gravado em `PhysicalDrive1`).
- **MEIHFS-V2.0 / V2.1 (`meihdfs1.h`):** `BSIZE = 0x800` (2 KiB), `BCNT = 0x180`, `ASIZE = 768 KiB` (zero fragmentação de vídeo), `ISIZE = 4 KiB` (`320` extents `block_run`), época base `1980-01-01` (`TIME_OFFSET = 315532800`), `ITBL_START = ASIZE + 0x6000` (V2.0) / `+ 0xB000` (V2.1).
- **Presets & Patch `src/hdd_cmd.c`:** `0xC6 (bls +12)` -> `0xCA (bra +12)` cobre `5 GB` (`0x0024C388` `SV0511D`) até `2 TB` (`0x3FFF_FFFF`, extensão `--allow-extended-4tb`); `#3 250 GB RTSD = 0x07466528` (`488.215.712` LBAs).
- **Ponte `P58001` (`VEK0K00` -> `N5EZZ0000003` -> `IDE2SAT2`):** Com cabo `Type A` + adaptador girado `180°`, o sistema lê `0x00002004` (`DD13|DD2`). A solução low-profile para fechar o chassi combina **`Type B` 40-pin 0.5mm FFC** com `N5EZZ0000003` girado `180°` (duas involuções `41 - (41 - k) = k`) e jumper da `IDE2SAT2` obrigatoriamente em **`MASTER`**.

## 5. Daqui pra Frente & Verificação
- **Fronteira aberta:** Isolar por que o `boot` frio `RTSC` ainda reporta `80000404` / `FUN_5041CD4D(1)` `is_ata_m=0` mesmo com `0xEC IDENTIFY` (`SAMSUNG HM251JI`) funcionando no probe MMIO `hdmi_flat` de `2026-11-22`.
- **Verificação:** Confira `SHA256SUMS.txt` após extrações, readback verde em `flash-eh55` e valide em `Mn103Vm` (`panadiga re verify-patches`) antes de levar `MAIN` modificado à bancada.
