# Panadiga 100%-Disassembled Ghidra Forensic Atlas Index

> Gerado deterministicamente por `python scripts/generate_firmware_atlas.py --write`. Verifique deriva com `--check`.
> **Atlas Fingerprint (`SHA-256`):** `8d912b5b373b2a72632dcb8a5914a4324fd2e2a7fb925d880ec1d0e9fda15270`
> **JSON Completo:** `profiles/forensic_atlas/firmware_forensic_atlas.json`

## 1. Cobertura Zero-Undef & Totais Indexados

| Métrica | Valor |
| :--- | ---: |
| Programas MN103S Zero-Undef | 53 / 55 |
| Instruções & Funções Frota Ghidra | 75,031,001 insns · 1,083,676 funcs |
| Funções Curadas (`doc_*.txt`) | 189 |
| Índices `profiles/symbols/` | 1 |
| Constantes Hex / MMIO Catalogadas | 160 |
| Padrões Bitwise Catalogados | 114 |
| Headers, Footers & Magics SSOT | 31 |
| Armadilhas Catalogadas (`profiles/traps/`) | 8 |

## 2. Headers, Footers, Magics & Fronteiras Físicas

| Artefato | Tipo | Assinatura / Janela | Notas |
| :--- | :--- | :--- | :--- |
| `EXTRHEAD` | `container_header` | `4558545248454144 ('EXTRHEAD')` | Encrypted/compressed stage header paired with EXTRFOOT footer |
| `EXTRFOOT` | `container_footer` | `45585452464F4F54 ('EXTRFOOT')` | Trailing EXTRHEAD container footer |
| `MainListHeader` | `toc_header` | `0x2800 (v2/HDD) / 0x1400 (CD/SD)` | Firmware TOC module table header preceding fmaX partitions |
| `dvd_iso9660_level1` | `optical_dvd` | `magic_hex=0x4344303031 @ start_lba=16` | fs=ISO9660_LEVEL1, sector=2048 B |
| `hdd_meihdfs_v1` | `ata_hdd` | `magic_hex=0x5348 @ start_lba=0` | fs=MEIHFS1.0, sector=512 B |
| `sd_fat16b_0x06` | `sd_card` | `magic_hex=0x55AA @ start_lba=63` | fs=FAT16B, sector=512 B |
| `eh50_ecb64_frm:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=le2011_08e0 |
| `loader_ver018_xip_main52df6:IPL_BANNER` | `bootloader_header` | `ascii='Loader ver0.18' [0x0000_0000..0x0004_0000]` | loader_profile_key=ver018 |
| `loader_ver061_toc48_main57:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=pana_dvd_eh55 |
| `loader_ver061_toc48_main57:IPL_BANNER` | `bootloader_header` | `ascii='Loader ver0.61' [0x0000_0000..0x0004_0000]` | loader_profile_key=ver061 |
| `loader_ver063_bootbin:IPL_BANNER` | `bootloader_header` | `ascii='Loader ver0.63' [0x0000_0000..0x0004_0000]` | loader_profile_key=ver063 |
| `lsi_zebra64_vxworks_bal:IPL_BANNER` | `bootloader_header` | `ascii='ZEBRA 64' [0x0000_4000..0x0000_5000]` | loader_profile_key=lsi_vxworks |
| `panaeusb_ex97_encrypted_usb:FRM_TOC` | `encrypted_unknown_non_feistel` | `entry=64B toc_size=0x2000` | key_id=None |
| `prog061_mini_4mb_main57:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `prog071_toc64_main50622:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `prog201_4mb_main50622:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `prog201_8mb_main50622_ez49v:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `prog202_8mb_main50622_ez49v:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `prog205_4mb_main50622:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `toc48_main57_ex80:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=old_df72 |
| `toc64_main57_ex81:FRM_TOC` | `feistel64_ecb` | `entry=64B toc_size=0x2000` | key_id=old_df72 |
| `uniphier_bs850_le2011_nand:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=le2011_08e0 |
| `uniphier_bs880_le2011_nand:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=le2011_08e0 |
| `uniphier_bs885_panasonic_hdr_nand:FRM_TOC` | `panasonic_48b_hdr_feistel64` | `entry=48B toc_size=0x1800` | key_id=le2011_08e0 |
| `uniphier_xw300_le2011_nand:FRM_TOC` | `feistel64_ecb` | `entry=48B toc_size=0x1800` | key_id=le2011_08e0 |
| `eh55_ver061_on_s29jl064h:bgo` | `nor_bgo` | `[0x0013_0000..=0x003F_FFFF]` | BGO (SA26..SA70) |
| `eh55_ver061_on_s29jl064h:eemul` | `nor_eemul` | `[0x007F_0000..=0x007F_FFFF] marker[+5]=0xF1` | EEMUL NVRAM (SA134..SA141) |
| `es35v_ver018_on_8mb:bgo` | `nor_bgo` | `[0x0070_0000..=0x007E_FFFF]` | BGO FTL (0x720600=M6720EE, 0x780000..0x79FFFF erased) |
| `es35v_ver018_on_8mb:eemul` | `nor_eemul` | `[0x007F_0000..=0x007F_FFFF] marker[+5]=0xF1` | EEMUL (0x7F0000 FF; 0x7F2000=02F1; 0x7F4000=01F1) |
| `loader_ver063_4mb_top_bgo35:bgo` | `nor_bgo` | `[0x0020_0000..=0x003E_FFFF]` | BGO Bank 1 4MB/TOP BOOT (0x84200000..0x843EFFFF, 31x64KiB; ver0.63 0x44036520 / PROG061 0x50436E30) |
| `loader_ver063_4mb_top_bgo35:eemul` | `nor_eemul` | `[0x003F_0000..=0x003F_FFFF]` | Top-Boot 8x8KiB (0x843F0000..0x843FFFFF; Bank 2 8x8KiB in PROG061 / EEMUL boundary) |

## 3. Top Constantes Hexadecimais & Endereços MMIO/Flash

| Hex | Barramento / Classe | Ocorrências | Sample VAs |
| :--- | :--- | ---: | :--- |
| `0xFFFFFFFF` | `MMIO_SYS` | 25 | `0x44001C49`, `0x44004749`, `0x4400492E`, `0x44004B06` |
| `0xFFFF` | `SRAM_OR_OFFSET` | 18 | `0x40005ABA`, `0x4002025E`, `0x400209F3`, `0x40021476` |
| `0x10000` | `SRAM_OR_OFFSET` | 8 | `0x40021476`, `0x4002168D`, `0x440018BB`, `0x440065BC` |
| `0x1000` | `SRAM_OR_OFFSET` | 6 | `0x4001FCAF`, `0x4002025E`, `0x401345DE`, `0x44026278` |
| `0x5056B604` | `SDRAM` | 6 | `0x4400D16B`, `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8` |
| `0xFFFFFFFC` | `MMIO_SYS` | 6 | `0x440018BB`, `0x44009EA7`, `0x4400FFED`, `0x5040211F` |
| `0x2000` | `SRAM_OR_OFFSET` | 5 | `0x4001FCAF`, `0x4400A86D`, `0x4400AA5F`, `0x4400AAC2` |
| `0x5056B620` | `SDRAM` | 5 | `0x4400C696`, `0x4400D32F`, `0x4400DFC8`, `0x4400E592` |
| `0x8000` | `SRAM_OR_OFFSET` | 5 | `0x40021476`, `0x4400BB14`, `0x44026549`, `0x4402724F` |
| `0x1637C` | `SRAM_OR_OFFSET` | 4 | `0x440262CB`, `0x4402724F`, `0x4402791A`, `0x44028008` |
| `0x20000` | `SRAM_OR_OFFSET` | 4 | `0x440065BC`, `0x44006CD9`, `0x44007C59`, `0x4400A97C` |
| `0x5050175C` | `SDRAM` | 4 | `0x4400A733`, `0x4400A86D`, `0x4400AA5F`, `0x4400AAC2` |
| `0x5056B6D4` | `SDRAM` | 4 | `0x4400C696`, `0x4400CB21`, `0x4400DCE2`, `0x4400E592` |
| `0xFFFFD8D4` | `MMIO_SYS` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `0xFFFFD8D7` | `MMIO_SYS` | 4 | `0x4400C696`, `0x4400CB21`, `0x4400D32F`, `0x4400DCE2` |
| `0xFFFFFFF6` | `MMIO_SYS` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `0xFFFFFFF7` | `MMIO_SYS` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `0xFFFFFFF8` | `MMIO_SYS` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `0xFFFFFFFB` | `MMIO_SYS` | 4 | `0x440018BB`, `0x44003FD0`, `0x5040211F`, `0x5749EFD3` |
| `0x1637E` | `SRAM_OR_OFFSET` | 3 | `0x4402724F`, `0x44027826`, `0x4402791A` |
| `0x180000` | `CONSTANT` | 3 | `0x44026278`, `0x44028008`, `0x44028348` |
| `0x1FC9E0` | `CONSTANT` | 3 | `0x4002025E`, `0x400209F3`, `0x40021476` |
| `0x44034835` | `CONSTANT` | 3 | `0x4400742C`, `0x440075AA`, `0x44007A12` |
| `0x44034C33` | `CONSTANT` | 3 | `0x44008634`, `0x44008B02`, `0x4400920B` |
| `0x505165DC` | `SDRAM` | 3 | `0x44008634`, `0x44008CC9`, `0x4400920B` |
| `0x505165E0` | `SDRAM` | 3 | `0x44008634`, `0x44008CC9`, `0x4400920B` |
| `0x505165E2` | `SDRAM` | 3 | `0x44008634`, `0x44008CC9`, `0x4400920B` |
| `0x50516DD4` | `SDRAM` | 3 | `0x440018BB`, `0x44008634`, `0x44008CC9` |
| `0x5056B600` | `SDRAM` | 3 | `0x4400C696`, `0x4400CED2`, `0x4400DC64` |
| `0x505716D4` | `SDRAM` | 3 | `0x4400CED2`, `0x4400E737`, `0x4400EDBF` |
| `0x7F000000` | `CONSTANT` | 3 | `0x44009835`, `0x44009B11`, `0x44009CEA` |
| `0x89001848` | `CONSTANT` | 3 | `0x44026549`, `0x4402724F`, `0x44028008` |
| `0xFFFFD8DC` | `MMIO_SYS` | 3 | `0x4400C9C0`, `0x4400CFDE`, `0x4400D32F` |
| `0xFFFFFFF` | `CONSTANT` | 3 | `0x4400D32F`, `0x4400DCE2`, `0x4400E737` |
| `0xFFFFFFFE` | `MMIO_SYS` | 3 | `0x440018BB`, `0x50401F3C`, `0x5040211F` |
| `0x1000000` | `CONSTANT` | 2 | `0x44006AE4`, `0x44027826` |
| `0x10001` | `SRAM_OR_OFFSET` | 2 | `0x440068F5`, `0x440262CB` |
| `0x1400000` | `CONSTANT` | 2 | `0x440047C7`, `0x4400599A` |
| `0x15F97` | `SRAM_OR_OFFSET` | 2 | `0x4402618B`, `0x440262CB` |
| `0x17000` | `SRAM_OR_OFFSET` | 2 | `0x40021476`, `0x4002168D` |

## 4. Top Máscaras & Operações Bitwise (`doc_*.txt`)

| Padrão Bitwise | Funções | Sample VAs |
| :--- | ---: | :--- |
| `& 0xff` | 12 | `0x440046E4`, `0x44008634`, `0x44008CC9`, `0x4401F804` |
| `& 0xffff` | 12 | `0x4400E737`, `0x4402724F`, `0x4400FC3F`, `0x4401A5CD` |
| `& 0x80` | 9 | `0x4402791A`, `0x4001FCAF`, `0x4002025E`, `0x400209F3` |
| `& 0x20` | 7 | `0x4400FC3F`, `0x44027826`, `0x4402791A`, `0x40021476` |
| `<< 0` | 7 | `0x4400C510`, `0x4400C0A8`, `0x4400E737`, `0x4400AB0A` |
| `>> 0` | 7 | `0x4400742C`, `0x4400AB0A`, `0x4401E5A3`, `0x44027826` |
| `& 0xf` | 6 | `0x4400DCE2`, `0x4400E592`, `0x4402791A`, `0x4001FCAF` |
| `| 0x80` | 6 | `0x44004749`, `0x4400FFED`, `0x40004852`, `0x400209F3` |
| `& 0x40` | 5 | `0x4400FFED`, `0x4401AA1F`, `0x4402791A`, `0x4002025E` |
| `>> 1` | 5 | `0x4401F804`, `0x4400E737`, `0x44012191`, `0x40021476` |
| `& 0x10` | 4 | `0x4400FFED`, `0x4401AA1F`, `0x4402791A`, `0x4002025E` |
| `& 0xfffffff6` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `& 0xfffffff7` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `& 0xfffffff8` | 4 | `0x4400D32F`, `0x4400DCE2`, `0x4400DFC8`, `0x4400EDBF` |
| `& 0xfffffffc` | 4 | `0x44009EA7`, `0x4400FFED`, `0x574DD156`, `0x574DD15B` |
| `<< 8` | 4 | `0x4400C510`, `0x4400C0A8`, `0x4400E737`, `0x4400AB0A` |
| `<< 9` | 4 | `0x44006AE4`, `0x4400CA59`, `0x440262CB`, `0x44028008` |
| `>> 2` | 4 | `0x44009EA7`, `0x44012191`, `0x574DD156`, `0x574DD15B` |
| `>> 8` | 4 | `0x44008634`, `0x44008CC9`, `0x40005772`, `0x40005ABA` |
| `& 0x7f000000` | 3 | `0x44009835`, `0x44009B11`, `0x44009CEA` |
| `& 0xffff0000` | 3 | `0x40005ABA`, `0x400209F3`, `0x401345DE` |
| `<< 5` | 3 | `0x4400C696`, `0x4400E592`, `0x5749859A` |
| `>> 9` | 3 | `0x440068F5`, `0x44008634`, `0x44008CC9` |
| `| 0x40` | 3 | `0x4001FCAF`, `0x50404FA7`, `0x5040DF40` |
| `| 0x7f` | 3 | `0x4400FFED`, `0x400209F3`, `0x401343C2` |
| `& 0x18` | 2 | `0x57000E1C`, `0x50400014` |
| `& 0x1f` | 2 | `0x4402791A`, `0x4002025E` |
| `& 0x1ff` | 2 | `0x44008634`, `0x44008CC9` |
| `& 0x2000` | 2 | `0x4402791A`, `0x4001FCAF` |
| `& 0x3fff` | 2 | `0x4402791A`, `0x4002025E` |

## 5. Funções & Callers Curados (`doc_*.txt` — Amostra Direta)

| VA | Símbolo / Alias | Tamanho | Callers | Origem |
| :--- | :--- | ---: | :--- | :--- |
| `0x440018BB` | `nfrom_recover_map` | 810 B | `0x4400403c` | `doc_boot_core.txt` |
| `0x44001C49` | `bgo_ioctl_dispatch` | 260 B | `0x440019a9`, `0x44001a0b`, `0x44001ad7`, `0x44001d67` | `doc_boot_core.txt` |
| `0x44003FD0` | `nfrom_power_fail_recovery` | 513 B | `0x440083bf`, `0x44008c1e`, `0x44008c8e` | `doc_boot_core.txt` |
| `0x440046E4` | `gpio_model_flag_46e4` | 101 B | `0x4400327d`, `0x440032e5`, `0x44003328`, `0x4400335e` | `doc_boot_core.txt` |
| `0x44004749` | `uart0_init` | 86 B | — | `doc_boot_core.txt` |
| `0x4400479F` | `uart0_getc_dead` | 40 B | `0x4400489a`, `0x44004904` | `doc_boot_core.txt` |
| `0x440047C7` | `uart0_putc_poll` | 65 B | `0x4400485c`, `0x44004863`, `0x44004877`, `0x440048c4` | `doc_boot_core.txt` |
| `0x4400488A` | `console_getline_dead` | 164 B | — | `doc_boot_core.txt` |
| `0x4400492E` | `boot_main` | 13 B | — | `doc_boot_core.txt` |
| `0x44004B06` | `feonly_mode` | 214 B | `0x440049eb` | `doc_boot_core.txt` |
| `0x44004BDE` | `fe_drive_mailbox_write` | 300 B | `0x44004b86` | `doc_boot_core.txt` |
| `0x44005086` | `timer_mcu_5086` | 93 B | `0x4400596c` | `doc_boot_core.txt` |
| `0x44005114` | `timer_rx_handler_5114` | 492 B | `0x440061e7` | `doc_boot_core.txt` |
| `0x4400599A` | `vfd_msg_599a` | 125 B | `0x440054c2`, `0x4400564a` | `doc_boot_core.txt` |
| `0x44005CD0` | `vfd_wait_5cd0` | 70 B | `0x440078bc`, `0x440079ef`, `0x4400b3b3` | `doc_boot_core.txt` |
| `0x44006454` | `detect_boot_mode` | 116 B | `0x44004977` | `doc_boot_core.txt` |
| `0x440064C8` | `parse_support_card_hdr` | 197 B | `0x44004a50` | `doc_boot_core.txt` |
| `0x4400658D` | `support_card_jump_658d` | 47 B | `0x4400688a` | `doc_boot_core.txt` |
| `0x440065BC` | `fmu_sd_dispatcher` | 614 B | `0x44004ad9`, `0x44004af1` | `doc_boot_core.txt` |
| `0x44006822` | `hdd_hdr_check_6822` | 211 B | `0x4400671c`, `0x44006786` | `doc_boot_core.txt` |
| `0x440068F5` | `hdd_load_mainsoft_68f5` | 495 B | `0x44006704`, `0x4400676e` | `doc_boot_core.txt` |
| `0x44006AE4` | `hdd_inner_recovery_6ae4` | 501 B | `0x440067b2` | `doc_boot_core.txt` |
| `0x44006CD9` | `boot_route_cd_sd_6cd9` | 777 B | `0x440065ec`, `0x4400662d`, `0x4400668b`, `0x44006816` | `doc_boot_core.txt` |
| `0x44006FE7` | `media_update_gate_6fe7` | 1093 B | `0x44006f11` | `doc_boot_core.txt` |
| `0x44007812` | `boot_panic_or_halt_7812` | 36 B | `0x44004a3c`, `0x44004a86`, `0x44004ace`, `0x44004afa` | `doc_boot_core.txt` |
| `0x44007836` | `vfd_u99_open_tray_7836` | 366 B | `0x44006ec7`, `0x4400774f`, `0x44007808`, `0x4400782c` | `doc_boot_core.txt` |
| `0x440079A4` | `power_off_or_halt_79a4` | 110 B | `0x44006e2b`, `0x440070bf`, `0x4400722e`, `0x4400726f` | `doc_boot_core.txt` |
| `0x44007EA9` | `bgo_flag_clear_7ea9` | 115 B | `0x440066af`, `0x44006e24`, `0x44006eb4`, `0x44006f4b` | `doc_boot_core.txt` |
| `0x44007F1C` | `bgo_flag_clear2_7f1c` | 65 B | `0x440066b6`, `0x44006e1d`, `0x44006ead`, `0x44006f44` | `doc_boot_core.txt` |
| `0x44007FA1` | `bgo_status_7fa1` | 94 B | `0x44006634` | `doc_boot_core.txt` |
| `0x44008075` | `bgo_update_loop_8075` | 409 B | `0x44006653`, `0x44006dc3`, `0x44006dd0`, `0x44006ddd` | `doc_boot_core.txt` |
| `0x4400825C` | `bgo_flag_update_825c` | 275 B | `0x44008036` | `doc_boot_core.txt` |
| `0x4400836F` | `bgo_driver_init` | 140 B | `0x44004a2b`, `0x44004abd`, `0x44004b14` | `doc_boot_core.txt` |
| `0x44008634` | `nvram_read_8634` | 864 B | `0x44004b36`, `0x44006c05`, `0x44007fc6`, `0x440093aa` | `doc_boot_core.txt` |
| `0x44008B02` | `bgo_ctrl_init_8b02` | 455 B | `0x44007ecb`, `0x44007eff`, `0x44007f40`, `0x440090bd` | `doc_boot_core.txt` |
| `0x44008CC9` | `bgo_device_info_check_8cc9` | 989 B | `0x44009101` | `doc_boot_core.txt` |
| `0x440090A6` | `bgo_write_part_90a6` | 153 B | `0x44004cc2`, `0x44006ca7`, `0x44007f76` | `doc_boot_core.txt` |
| `0x4400920B` | `bgo_read_part_920b` | 324 B | `0x44006bde`, `0x4400936d` | `doc_boot_core.txt` |
| `0x4400934F` | `conveyer_check_934f` | 133 B | `0x440065f3`, `0x44006fff` | `doc_boot_core.txt` |
| `0x4400F5CB` | `nvram_read_id` | 196 B | `0x440041ee`, `0x440046fd`, `0x4400472d`, `0x4400473d` | `doc_boot_core.txt` |
| `0x4401F044` | `bgo_nor_part_check_1f044` | 61 B | `0x4401e669`, `0x4401e6ce`, `0x4401e721`, `0x4401e79d` | `doc_boot_core.txt` |
| `0x4401F081` | `bgo_err_log_1f081` | 59 B | `0x4401ef92`, `0x4401f0d4`, `0x4401f4d9`, `0x4401f577` | `doc_boot_core.txt` |
| `0x4401F804` | `bgo_nor_sum_1f804` | 292 B | `0x4401f785` | `doc_boot_core.txt` |
| `0x440237BA` | `bgo_nor_erase_write_237ba` | 395 B | `0x4401ecfa`, `0x44022e69` | `doc_boot_core.txt` |
| `0x440236ED` | `bgo_nor_cfi_cmd_236ed` | 205 B | `0x440237c8` | `doc_boot_core.txt` |
| `0x4400A97C` | `FUN_4400a97c` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C696` | `FUN_4400c696` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C879` | `FUN_4400c879` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C922` | `FUN_4400c922` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C94D` | `FUN_4400c94d` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C9C0` | `FUN_4400c9c0` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400D16B` | `FUN_4400d16b` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400D32F` | `FUN_4400d32f` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400CED2` | `FUN_4400ced2` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400CFDE` | `FUN_4400cfde` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400D11E` | `FUN_4400d11e` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400D16B` | `FUN_4400d16b` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400DCE2` | `FUN_4400dce2` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C510` | `FUN_4400c510` | 0 B | — | `doc_boot_fat_sd_java.txt` |
| `0x4400C0A8` | `FUN_4400c0a8` | 0 B | — | `doc_boot_fat_sd_java.txt` |
