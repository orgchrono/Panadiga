---
name: panadiga-extract-partitions
description: >-
  Executa a análise forense, extração de partições e gravação segura de discos Panasonic (MEIHFS, MBR 'SH', mirrors e partições de firmware de 32MB/64MB).
  Use esta skill para inspecionar dumps de HDD, extrair partições binárias, formatar/gravar alvos físicos com lockout multi-SO ou validar manifestos SHA-256.
---

# Extração, Auditoria & Gravação Forense de Storage MEIHFS

Esta skill documenta o procedimento seguro de mapeamento, extração e gravação de discos rígidos Panasonic DIGA (`DMR-EH55`) em **Windows**, **Linux** e **macOS**.

## 1. Inspeção & Extração Read-Only (Sem Alterar o Dump Original)
**Regra Forense:** Nunca edite ou trunque imagens brutas originais (*in-place*).

```bash
# Inspecionar MBR, descritores LBA 24..31 e mapa de densidade:
cargo run -p panadiga-cli -- hdd inspect "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --density

# Extrair MBR, mirrors, journal e partições de firmware + SHA256SUMS.txt:
cargo run -p panadiga-cli -- hdd extract "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --outdir firmware_and_images/extracted_partitions

# Decodificar descritores XOR 0xFF (One's Complement) e journal MEIHFS:
cargo run -p panadiga-cli -- hdd decode-descriptors "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin" --journal

# Auditar ou patchear limite de capacidade (>200 GB em Stage-2 MAIN):
cargo run -p panadiga-cli -- hdd inspect-capacity "firmware_and_images/panasonic DMR EH-55 Firmware hdd_100.bin"
cargo run -p panadiga-cli -- hdd patch-capacity --input <INPUT> --output <OUTPUT> --mode dynamic
```

## 2. Descoberta & Gravação Segura Multi-SO (`list-drives` / `flash-eh55` / `format-meihdfs`)
```bash
# Listar unidades classificadas por papel (hdd | sd | system | all):
cargo run -p panadiga-cli -- hdd list-drives --role hdd
cargo run -p panadiga-cli -- hdd list-sd-cards

# Gravar imagem EH55 verificada com backup prévio obrigatório de 64 KiB + espelho MAIN + readback SHA-256:
cargo run -p panadiga-cli -- hdd flash-eh55 \
  --drive "<DEVICE_PATH>" \
  --image "<EH55_100M_IMG>" \
  --backup-dir "<BACKUP_DIR>" \
  --confirm-physical-drive "<CONFIRM_TOKEN>"
```

### Caminhos de Dispositivo & Travas de Segurança por Sistema Operacional
- **Windows**: `\\.\PhysicalDriveN` (token `CONFIRM-PhysicalDriveN`, exige privilégios de Administrador). `PhysicalDrive0` e discos `BusTypeNvme (17)` são **bloqueados permanentemente** (`is_unconditional_os_boot_disk`).
- **Linux**: `/dev/sdX`, `/dev/mmcblkN` (token `CONFIRM-/dev/sdX`, exige UID 0 / grupo `disk`). O sistema detecta os discos raiz dinamicamente via `/proc/mounts` + `/dev/mapper`, bloqueia `/dev/sda*` como fallback seguro e desmonta partições via `udisksctl`/`umount`.
- **macOS**: `/dev/diskN` ou `/dev/rdiskN` (token `CONFIRM-/dev/diskN`, exige `sudo`/root). `/dev/disk0*` e `/dev/rdisk0*` são **bloqueados incondicionalmente**; `diskutil unmountDisk` é executado apenas para nós `/dev/[r]diskN` (nunca em arquivos `.bin` de teste).

## 3. Estruturas Extraídas e Mapeadas
1. `LBA 0` (`0x000000`): MBR proprietário (assinatura `"SH"` / `0x53 0x48` e Disk GUID).
2. `LBA 24..31`: Descritores de alocação MEIHFS (One's Complement XOR `0xFF`).
3. `LBA 16384` (`8 MB / 0x00800000`): Journal de alocação de vídeo.
4. `LBA 57344` (`28 MB / 0x01C00000`) & `LBA 57368`: Espelhos redundantes do MBR e descritores.
5. `0x02000000` (`32 MB`) & `0x04000000` (`64 MB`): Partições de firmware primária e secundária (`51 87 13 8D 5B 0A FB 22`).
6. `MAIN_LBA = ((disk_bytes / 512) & !3) - 0x20000`: Posição do espelho `MAIN` de fim de disco no DMR-EH55.
7. `SHA256SUMS.txt`: Manifesto criptográfico gerado para cadeia de custódia forense.
