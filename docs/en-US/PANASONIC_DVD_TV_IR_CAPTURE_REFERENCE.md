> 🌐 **Language / Idioma:** **English (US)** | [Português (BR)](../pt-BR/REFERENCIA_CAPTURA_IR_PANASONIC_DVD_TV.md)
> 🧭 **Portal:** [Docs Index](../INDEX.md) | [Complete Audit](../../COMPLETE_AUDIT.md) | [Historical Audits](../audits/README.md) | [Ghidra Pseudocode](../pseudocode/INDICE_EVIDENCIA_PSEUDOCODIGO_GHIDRA.md) | [Debug Logs](../debug_logs/README.md)

# Technical IR Capture Reference: Panasonic DVD/TV (`Panasonic_DVD_TV.ir`)
## 48-Bit LSB-First Kaseikyo Decoding, Complete 50-Signal Inventory, and Comparison vs. `EUR7659Y70`, Teletext, and Android CIR

**Document:** `PANASONIC_DVD_TV_IR_CAPTURE_REFERENCE.md`  
**Source Artifact:** [`firmware_and_images/Panasonic_DVD_TV.ir`](../../firmware_and_images/Panasonic_DVD_TV.ir) (`4,943 bytes`, `305` lines, `50` parsed signals)  
**Verified SHA-256:** `82acb0a7611c89ea56799094bf3c71b9e366f38c0301b457cf5c8266279a9b99`  
**Origin / Credit:** `Flipper-IRDB` (`DVD_Players/Panasonic/Panasonic_DVD_TV.ir`, credit `emptythevoid` — `https://github.com/emptythevoid`)  
**Date:** November 2026  

---

## 1. Executive Summary & Artifact Metadata

[`firmware_and_images/Panasonic_DVD_TV.ir`](../../firmware_and_images/Panasonic_DVD_TV.ir) is a real **Flipper Zero IR (`Filetype: IR signals file`, `Version: 1`, `protocol: Kaseikyo`)** capture from a North American Panasonic DVD/HDD + TV combo remote control. Despite its generic filename `Panasonic_DVD_TV`, the capture includes dedicated Panasonic **DIGA** HDD/DVD recorder commands (`Time_slip`, `Cm_skip`, `Direct_navigator`, `Create_chapter`, `Rec_mode`, `F_rec`), totaling **50 parsed signals** (`lines 7–305`):

* **TV Subsystem (`dev = 0x80`):** **6 signals** (`Tv_power`, `Tv_video`, `Tv_chan_up`, `Tv_chan_down`, `Tv_vol_up`, `Tv_vol_down`).
* **DVD / DIGA HDD Recorder Subsystem (`dev = 0xB0`):** **44 signals** (power, `0`–`9` keypad, transport, OSD navigation, scheduling, and recorder-specific controls).

---

## 2. Flipper `.ir` Kaseikyo Decoding Formula

Every `type: parsed` / `protocol: Kaseikyo` record in `Panasonic_DVD_TV.ir` stores two 4-byte little-endian fields:

```text
address: <dev> 02 20 <hi>
command: <c0> <c1> 00 00
```

### 2.1 Flipper Field Layout vs. 48-Bit (6-Byte) Physical Wire Frame

The **Panasonic Kaseikyo** IR protocol (`38 kHz` carrier, time unit `T ≈ 432 µs`, leader `8T` mark / `4T` space) transmits **48 bits (6 bytes) LSB-first**:

```text
Byte 0 = 0x02                 (Panasonic Vendor ID LSB: 0x2002)
Byte 1 = 0x20                 (Panasonic Vendor ID MSB: 0x2002)
Byte 2 = <dev>                (Device / Genre: 0xB0 = DVD/DIGA HDD | 0x80 = TV)
Byte 3 = 0x00                 (Subdevice = 0x00 / SET 1 across all 50 signals in this file)
Byte 4 = <cmd_byte>           (8-bit command code)
Byte 5 = <dev> ^ <cmd_byte>   (XOR checksum: dev ^ subdevice ^ cmd_byte, with subdevice = 0x00)
```

Inside Flipper Zero's packed representation:
1. **`address[0]` (`<dev>`)** holds the device byte (`0xB0` or `0x80`; its low nibble `0x0` also matches the Kaseikyo vendor-nibble parity `0^2^2^0 = 0x0`).
2. **`address[1..2]` (`02 20`)** retain the two Matsushita/Panasonic Vendor ID bytes (`0x2002`).
3. **`address[3]` (`<hi>`)** ranges over `0x00..0x03`, encoding the **top 2 bits `[7:6]`** of `<cmd_byte>`.
4. **`command[0]` (`<c0>`)** always has low nibble `0x0` (reflecting `subdevice = 0x00`), while its upper nibble (`c0 >> 4`) encodes the **low 4 bits `[3:0]`** of `<cmd_byte>`.
5. **`command[1]` (`<c1>`)** ranges over `0x00..0x03`, encoding the **middle 2 bits `[5:4]`** of `<cmd_byte>`, while `command[2..3]` are fixed `00 00`.

### 2.2 Exact Reconstruction Equation

Deterministic extraction of `<cmd_byte>` and the complete 6-byte LSB-first wire frame uses:

```text
cmd_byte = ((hi & 0x03) << 6) | ((c1 & 0x0F) << 4) | (c0 >> 4)

48-Bit Wire Frame (6 bytes, LSB-first):
  02 20 <dev> 00 <cmd_byte> <dev ^ cmd_byte>
```

#### Worked Examples Directly From the File
* **`Dvd_power` (lines 7–11):** `address: B0 02 20 00`, `command: D0 03 00 00`  
  `cmd_byte = ((0x00 & 0x03) << 6) | ((0x03 & 0x0F) << 4) | (0xD0 >> 4) = 0x00 | 0x30 | 0x0D = 0x3D`  
  Checksum: `0xB0 ^ 0x3D = 0x8D` → Wire frame: **`02 20 B0 00 3D 8D`**.
* **`Audio` (`AUDIO [A]`, lines 259–263):** `address: B0 02 20 00`, `command: 30 03 00 00`  
  `cmd_byte = ((0x00 & 0x03) << 6) | ((0x03 & 0x0F) << 4) | (0x30 >> 4) = 0x00 | 0x30 | 0x03 = 0x33`  
  Checksum: `0xB0 ^ 0x33 = 0x83` → Wire frame: **`02 20 B0 00 33 83`**.
* **`Display` (`DISPLAY [B]`, lines 265–269):** `address: B0 02 20 02`, `command: 20 01 00 00`  
  `cmd_byte = ((0x02 & 0x03) << 6) | ((0x01 & 0x0F) << 4) | (0x20 >> 4) = 0x80 | 0x10 | 0x02 = 0x92`  
  Checksum: `0xB0 ^ 0x92 = 0x22` → Wire frame: **`02 20 B0 00 92 22`**.
* **`F_rec` (lines 301–305):** `address: B0 02 20 03`, `command: 70 00 00 00`  
  `cmd_byte = ((0x03 & 0x03) << 6) | ((0x00 & 0x0F) << 4) | (0x70 >> 4) = 0xC0 | 0x00 | 0x07 = 0xC7`  
  Checksum: `0xB0 ^ 0xC7 = 0x77` → Wire frame: **`02 20 B0 00 C7 77`**.

---

## 3. Complete Inventory of All 50 Commands in `Panasonic_DVD_TV.ir`

### 3.1 Panasonic TV Subsystem (`dev = 0x80` — 6 Signals)

Shared functions (`Tv_power`, `Tv_chan_up`, `Tv_chan_down`) reuse the exact same command codes (`0x3D`, `0x34`, `0x35`) as the DVD/DIGA subsystem, changing only the device byte from `0xB0` to `0x80` (and its resulting XOR checksum).

| # | Lines | `Name` | Flipper `Address` | Flipper `Command` | `Cmd` | Hex Wire 48-bit (6 bytes) | Practical Function (TV / Combo) |
| :-: | :---: | :--- | :--- | :--- | :-: | :--- | :--- |
| 2 | 13–17 | `Tv_power` | `80 02 20 00` | `D0 03 00 00` | `0x3D` | `02 20 80 00 3D BD` | Panasonic TV Power On / Standby (`TV POWER`) |
| 3 | 19–23 | `Tv_video` | `80 02 20 00` | `50 00 00 00` | `0x05` | `02 20 80 00 05 85` | TV input source toggle (`TV/VIDEO`) |
| 4 | 25–29 | `Tv_chan_up` | `80 02 20 00` | `40 03 00 00` | `0x34` | `02 20 80 00 34 B4` | TV Channel Up (`TV CH +`) |
| 5 | 31–35 | `Tv_chan_down` | `80 02 20 00` | `50 03 00 00` | `0x35` | `02 20 80 00 35 B5` | TV Channel Down (`TV CH -`) |
| 6 | 37–41 | `Tv_vol_up` | `80 02 20 00` | `00 02 00 00` | `0x20` | `02 20 80 00 20 A0` | TV Volume Up (`VOL +`) |
| 7 | 43–47 | `Tv_vol_down` | `80 02 20 00` | `10 02 00 00` | `0x21` | `02 20 80 00 21 A1` | TV Volume Down (`VOL -`) |

---

### 3.2 DVD / DIGA HDD Recorder Subsystem (`dev = 0xB0` — 44 Signals)

All 44 commands below target `dev = 0xB0` and `subdevice = 0x00` (factory default `SET 1` on the Panasonic DMR-EH55):

| # | Lines | `Name` | Flipper `Address` | Flipper `Command` | `Cmd` | Hex Wire 48-bit (6 bytes) | Practical Function on DMR-EH55 / Combos |
| :-: | :---: | :--- | :--- | :--- | :-: | :--- | :--- |
| 1 | 7–11 | `Dvd_power` | `B0 02 20 00` | `D0 03 00 00` | `0x3D` | `02 20 B0 00 3D 8D` | DVD/HDD recorder Power On / Standby (`POWER`) |
| 8 | 49–53 | `1` | `B0 02 20 00` | `00 01 00 00` | `0x10` | `02 20 B0 00 10 A0` | Digit `1` (tuner & `SERV` diagnostic codes `00..99`) |
| 9 | 55–59 | `2` | `B0 02 20 00` | `10 01 00 00` | `0x11` | `02 20 B0 00 11 A1` | Digit `2` (tuner & `SERV` diagnostic codes `00..99`) |
| 10 | 61–65 | `3` | `B0 02 20 00` | `20 01 00 00` | `0x12` | `02 20 B0 00 12 A2` | Digit `3` (tuner & `SERV` diagnostic codes `00..99`) |
| 11 | 67–71 | `4` | `B0 02 20 00` | `30 01 00 00` | `0x13` | `02 20 B0 00 13 A3` | Digit `4` (tuner & `SERV` diagnostic codes `00..99`) |
| 12 | 73–77 | `5` | `B0 02 20 00` | `40 01 00 00` | `0x14` | `02 20 B0 00 14 A4` | Digit `5` (tuner & `SERV` diagnostic codes `00..99`) |
| 13 | 79–83 | `6` | `B0 02 20 00` | `50 01 00 00` | `0x15` | `02 20 B0 00 15 A5` | Digit `6` (tuner & `SERV` diagnostic codes `00..99`) |
| 14 | 85–89 | `7` | `B0 02 20 00` | `60 01 00 00` | `0x16` | `02 20 B0 00 16 A6` | Digit `7` (tuner & `SERV` diagnostic codes `00..99`) |
| 15 | 91–95 | `8` | `B0 02 20 00` | `70 01 00 00` | `0x17` | `02 20 B0 00 17 A7` | Digit `8` (tuner & `SERV` diagnostic codes `00..99`) |
| 16 | 97–101 | `9` | `B0 02 20 00` | `80 01 00 00` | `0x18` | `02 20 B0 00 18 A8` | Digit `9` (tuner & `SERV` diagnostic codes `00..99`) |
| 17 | 103–107 | `0` | `B0 02 20 00` | `90 01 00 00` | `0x19` | `02 20 B0 00 19 A9` | Digit `0` (tuner & `SERV` diagnostic codes `00..99`) |
| 18 | 109–113 | `Ch_up` | `B0 02 20 00` | `40 03 00 00` | `0x34` | `02 20 B0 00 34 84` | Next channel / page up in lists (`CH +`) |
| 19 | 115–119 | `Ch_down` | `B0 02 20 00` | `50 03 00 00` | `0x35` | `02 20 B0 00 35 85` | Previous channel / page down in lists (`CH -`) |
| 20 | 121–125 | `Add_del_ch` | `B0 02 20 00` | `60 03 00 00` | `0x36` | `02 20 B0 00 36 86` | Add/delete tuner channel preset (`ADD/DELETE`) |
| 21 | 127–131 | `Delete` | `B0 02 20 01` | `40 00 00 00` | `0x44` | `02 20 B0 00 44 F4` | Erase selected title/chapter/item (`ERASE` / `DELETE`) |
| 22 | 133–137 | `Cancel_asterisk` | `B0 02 20 02` | `30 00 00 00` | `0x83` | `02 20 B0 00 83 33` | Cancel numeric entry or operation (`CANCEL` / `*`) |
| 23 | 139–143 | `Input_select` | `B0 02 20 01` | `00 01 00 00` | `0x50` | `02 20 B0 00 50 E0` | Cycle AV input (`AV1..AV4` / `DV` / Tuner; useful in TBC bypass) |
| 24 | 145–149 | `Skip_back` | `B0 02 20 01` | `90 00 00 00` | `0x49` | `02 20 B0 00 49 F9` | Skip to previous chapter/track (`\|<< SKIP`) |
| 25 | 151–155 | `Skip_forward` | `B0 02 20 01` | `A0 00 00 00` | `0x4A` | `02 20 B0 00 4A FA` | Skip to next chapter/track (`>>\| SKIP`) |
| 26 | 157–161 | `Slow_back` | `B0 02 20 00` | `40 00 00 00` | `0x04` | `02 20 B0 00 04 B4` | Reverse search / slow reverse (`<< SLOW/SEARCH`) |
| 27 | 163–167 | `Slow_forward` | `B0 02 20 00` | `50 00 00 00` | `0x05` | `02 20 B0 00 05 B5` | Forward search / slow forward (`>> SLOW/SEARCH`) |
| 28 | 169–173 | `Stop` | `B0 02 20 00` | `00 00 00 00` | `0x00` | `02 20 B0 00 00 B0` | Stop playback or recording (`STOP`) |
| 29 | 175–179 | `Pause` | `B0 02 20 00` | `60 00 00 00` | `0x06` | `02 20 B0 00 06 B6` | Pause playback or recording (`PAUSE`) |
| 30 | 181–185 | `Play` | `B0 02 20 00` | `A0 00 00 00` | `0x0A` | `02 20 B0 00 0A BA` | Start playback (`PLAY`) |
| 31 | 187–191 | `Time_slip` | `B0 02 20 01` | `B0 00 00 00` | `0x4B` | `02 20 B0 00 4B FB` | Time Slip jump during playback/recording (`TIME SLIP`) |
| 32 | 193–197 | `Cm_skip` | `B0 02 20 01` | `C0 00 00 00` | `0x4C` | `02 20 B0 00 4C FC` | Commercial skip / +60s forward jump (`CM SKIP`) |
| 33 | 199–203 | `Direct_navigator` | `B0 02 20 02` | `B0 01 00 00` | `0x9B` | `02 20 B0 00 9B 2B` | Open recorded HDD/DVD title list (`DIRECT NAVIGATOR`) |
| 34 | 205–209 | `Schedule` | `B0 02 20 01` | `C0 03 00 00` | `0x7C` | `02 20 B0 00 7C CC` | Open timer recording schedule (`PROG/CHECK`) |
| 35 | 211–215 | `Functions` | `B0 02 20 01` | `70 01 00 00` | `0x57` | `02 20 B0 00 57 E7` | Main `FUNCTIONS` menu (Setup, Dubbing, Disc Mgmt, `SET 2/3`) |
| 36 | 217–221 | `Up` | `B0 02 20 02` | `50 00 00 00` | `0x85` | `02 20 B0 00 85 35` | Directional navigation Up (`UP`) |
| 37 | 223–227 | `Left` | `B0 02 20 02` | `70 00 00 00` | `0x87` | `02 20 B0 00 87 37` | Directional navigation Left (`LEFT`) |
| 38 | 229–233 | `Right` | `B0 02 20 02` | `80 00 00 00` | `0x88` | `02 20 B0 00 88 38` | Directional navigation Right (`RIGHT`) |
| 39 | 235–239 | `Down` | `B0 02 20 02` | `60 00 00 00` | `0x86` | `02 20 B0 00 86 36` | Directional navigation Down (`DOWN`) |
| 40 | 241–245 | `Enter` | `B0 02 20 02` | `20 00 00 00` | `0x82` | `02 20 B0 00 82 32` | Confirm OSD selection (`ENTER` / `OK`) |
| 41 | 247–251 | `Sub_menu` | `B0 02 20 02` | `00 00 00 00` | `0x80` | `02 20 B0 00 80 30` | Context menu / softkey `SUB MENU [S]` |
| 42 | 253–257 | `Return` | `B0 02 20 02` | `10 00 00 00` | `0x81` | `02 20 B0 00 81 31` | Return to previous screen/menu (`RETURN`) |
| 43 | 259–263 | `Audio` | `B0 02 20 00` | `30 03 00 00` | `0x33` | `02 20 B0 00 33 83` | Audio track selector / softkey `AUDIO [A]` (**NOT** `[RED]`) |
| 44 | 265–269 | `Display` | `B0 02 20 02` | `20 01 00 00` | `0x92` | `02 20 B0 00 92 22` | On-screen status overlay / softkey `DISPLAY [B]` |
| 45 | 271–275 | `Create_chapter` | `B0 02 20 01` | `80 01 00 00` | `0x58` | `02 20 B0 00 58 E8` | Insert manual chapter marker (`CREATE CHAPTER`) |
| 46 | 277–281 | `Setup` | `B0 02 20 02` | `40 01 00 00` | `0x94` | `02 20 B0 00 94 24` | Direct shortcut to system configuration (`SETUP`) |
| 47 | 283–287 | `Rec` | `B0 02 20 00` | `80 00 00 00` | `0x08` | `02 20 B0 00 08 B8` | Start immediate recording (`REC`) |
| 48 | 289–293 | `Rec_mode` | `B0 02 20 01` | `10 01 00 00` | `0x51` | `02 20 B0 00 51 E1` | Cycle recording quality `XP/SP/LP/EP` (`REC MODE`) |
| 49 | 295–299 | `Status` | `B0 02 20 01` | `F0 01 00 00` | `0x5F` | `02 20 B0 00 5F EF` | Extended disc/signal status display (`STATUS`) |
| 50 | 301–305 | `F_rec` | `B0 02 20 03` | `70 00 00 00` | `0xC7` | `02 20 B0 00 C7 77` | Flexible Recording matching remaining disc space (`F REC`) |

---

## 4. Comparative Note: North American Remotes vs. Teletext `[RED]`/`ND2`, `EUR7659Y70` & Android CIR

`Panasonic_DVD_TV.ir` definitively resolves a common bench question regarding key differences across Panasonic DIGA remote control regions:

1. **North American Combo Remotes (`Panasonic_DVD_TV.ir`) vs. EU/JDM Teletext Row (`[RED]` / `ND2`):**
   * On North American remotes, the lower row of **4 Teletext colour buttons** found on European/Japanese models (`EUR7659Y60` / `EUR7659YC0`) was omitted at the factory and replaced with Guide/VCR+ softkeys labeled **`AUDIO [A]`**, **`DISPLAY [B]`**, and **`SUB MENU [S]`**.
   * As directly proven by signals #43 (`Audio`), #44 (`Display`), and #41 (`Sub_menu`) in `Panasonic_DVD_TV.ir`:
     * **`AUDIO [A]`** transmits **`cmd = 0x33`** (`sub = 0x00`, wire `02 20 B0 00 33 83`).
     * **`DISPLAY [B]`** transmits **`cmd = 0x92`** (`sub = 0x00`, wire `02 20 B0 00 92 22`).
     * **`SUB MENU [S]`** transmits **`cmd = 0x80`** (`sub = 0x00`, wire `02 20 B0 00 80 30`).
   * **None of these buttons replace Teletext or extended service codes**, which live in **`subdevice = 0x01`** (entirely absent from all 50 captures in `Panasonic_DVD_TV.ir`):
     * **Teletext `[RED]`:** code **`0x141`** (`sub = 0x01`, `cmd = 0x41`, wire `02 20 B0 01 41 F0`; `[BLUE]` = `0x140`, `[GREEN]` = `0x142`, `[YELLOW]` = `0x143`).
     * **`ND2` (`[5]+[9]+[RED]` Service Non-Disclosure Mode 2):** code **`0x182`** (`sub = 0x01`, `cmd = 0x82`, wire `02 20 B0 01 82 33`).
2. **Comparison with the `EUR7659Y70` Physical Remote (DMR-EH55):**
   * The physical `EUR7659Y70` remote (Americas/Brazil) likewise lacks `[RED]` (`0x141`) and `ND2` (`0x182`), sharing the same `sub = 0x00` command table as `Panasonic_DVD_TV.ir` for digits `0..9` (`0x10..0x19`), power (`0x3D`), navigation, `FUNCTIONS` (`0x57`), `DIRECT NAVIGATOR` (`0x9B`), `AUDIO` (`0x33`), and `DISPLAY` (`0x92`), while adding dedicated physical DIGA keys absent from this combo capture (such as `OPEN/CLOSE` = `0x01` and `DRIVE SELECT` `DVD/HDD`).
   * On the **Panasonic DMR-EH55** (Service Manual `DSD0603003CE`), lacking `[RED]` does **not** block diagnostics: Service Mode (**`SERV`**) is entered from Standby using the front panel buttons (**`[CH UP] + [OPEN/CLOSE] + [REC]` held for 5 s**), and all service routines (`00`..`99`) require only the standard `0..9` keys (`0x10..0x19`).
3. **Programmable Emitters (Android CIR & Pico `IRTX`):**
   * Whenever `sub = 0x01` codes (`[RED]` `0x141`, `ND2` `0x182`), alternate address sets `SET 1/2/3` (`sub = 0x00 / 0x10 / 0x20`), or multi-step macros such as Region Free (`1..0 -> AUDIO -> MEDIA SERVER`) are required, use either an **Android device with a built-in IR blaster** (`ConsumerIrManager` with `.irplus` profiles like `Panasonic_DMR_EH55_Commander.irplus` from `android-remote-panasonic`) or the Pico 38 kHz PWM synthesizer **`IRTX`** (`firmware/pico-p7501/diag/p7501_ir_codec.c`, restricted to safe spare GPIOs `GP6..9` / `GP16..22`).

---

## 5. Related Documents

* [`firmware_and_images/README.md`](../../firmware_and_images/README.md) — Binary & bench capture asset catalog (Section #6).
* [`GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md`](GLOBAL_COMMUNITY_COMPENDIUM_DIGA_FIRMWARE_AND_HARDWARE.md) — Section `4.9` (Practical `EUR7659Y70`, `AUDIO [A]` vs. `[RED]`, Android CIR & `IRTX` map).
* [`THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md`](THEORY_AND_PRACTICE_U99_UNBRICK_MATSUSHITA_CRYPTO.md) — `U99` root cause, Spansion NOR map, and recovery workflow.
