> [!NOTE]
> **Snapshot Histórico (`2026-11-11`):** Esta auditoria foi capturada antes da refatoração modular final em `crates/`, `firmware/pico-p7501/` e `scripts/`. Para o estado atual consolidado, consulte [`../../AUDITORIA_COMPLETA.md`](../../AUDITORIA_COMPLETA.md).

# 🔬 Auditoria Forense: Firmware Pico + Usabilidade + Pipeline de 1ª Conexão e Handshake DIGA Panasonic

> **Data:** 2026-11-11 | **Commit base:** `0866a03` | **Build:** `dist/panadiga.exe` (14.1 MB, 2026-11-11 08:12)
> **Escopo:** Firmware C11 embarcado no RP2040/RP2350, camada Rust de conectividade, camada TypeScript de UI/ViewModel, e pipeline `run.cmd` de primeira inicialização.

---

## 1. Firmware C11 Embarcado — `dwire32a_pico_main.c`

### 1.1 Binários UF2 (`src/pico/firmware_bins/`)

| Arquivo | Blocos UF2 | Tamanho | Magic0 | Magic1 | MagicEnd | FamilyID | Válido? |
|---|---|---|---|---|---|---|---|
| `panasonic_dwire32a_rp2040.uf2` | 228 | 116 736 B | `0x0A324655` ✅ | `0x9E5D5157` ✅ | `0x0AB16F30` ✅ | `0xE48BFF56` (RP2040 SDK real) ✅ | **✅ VÁLIDO** |
| `panasonic_dwire32a_rp2350.uf2` | 213 | 109 056 B | `0x0A324655` ✅ | `0x9E5D5157` ✅ | `0x0AB16F30` ✅ | `0xE48BFF57` (RP2350 SDK real) ✅ | **✅ VÁLIDO** |

> [!NOTE]
> `0xE48BFF56` / `0xE48BFF57` são os Family IDs reais gerados pelo `elf2uf2` do Pico SDK oficial, distintos dos IDs do spec Microsoft UF2. São reconhecidos pelo bootrom RP2040/RP2350 e pelo `picotool`.
> A constante `RP2040_FAMILY_ID = 0xE4849208` em `flasher.rs` é do spec Microsoft — funciona para validação manual mas **não casa com os binários reais**.

### 1.2 Bug Crítico: Validação UF2 Falsa Positiva

**Localização:** [`src/pico/flasher.rs`](file:///C:/Users/danie/Documents/Panadiga/src/pico/flasher.rs#L73-L81)

```rust
// ATUAL — errado para binários reais do SDK:
let family_id = u32::from_le_bytes(data[28..32].try_into().unwrap());
let family_name = match family_id {
    RP2040_FAMILY_ID => "Raspberry Pi RP2040".to_string(),  // 0xE4849208 — NÃO bate
    // ...
    other => format!("Pico-Compatible Controller (Family ID 0x{:08X})", other),
};
```

Os firmwares embutidos **não passam pelo branch `RP2040_FAMILY_ID`** — caem no `other`. Isso não bloqueia o flash (os bytes são válidos), mas a mensagem de retorno ao usuário fica como `"Pico-Compatible Controller (Family ID 0xE48BFF56)"` em vez de `"Raspberry Pi RP2040"`.

**Fix:** adicionar os Family IDs reais do SDK.

### 1.3 Protocolo HANDSHAKE — Firmware ↔ Rust

O firmware C11 imprime na saída CDC quando o comando `HANDSHAKE` é bem-sucedido:

```
DCSR: 0x00000001
[AUTODETECT] Passive Address Aliasing Probe: 8 MB NOR Flash detected (EEMUL NVRAM @ 0x807F0000).
[HANDSHAKE] SUCCESS! Matsushita AM34 core is HALTED in On-Chip Debug mode (Mode 2).
```

A função `read_serial_response_complete()` em Rust ([`serial.rs:56-59`](file:///C:/Users/danie/Documents/Panadiga/src/desktop/pico/serial.rs#L56-L59)) aguarda exatamente `[HANDSHAKE] SUCCESS`, `[HANDSHAKE] FAILED`, `[HANDSHAKE] ABORT`, `[HANDSHAKE] ACTIVE` — **compatibilidade perfeita**.

### 1.4 Pipeline de 1ª Conexão — Sequência Correta

```
pico_connect_and_test()
  ├── Abre porta serial @ 115 200 bps, timeout 1,5s
  ├── Envia "\r\nPING\r\n"
  ├── Lê banner de resposta (até 1024B)
  ├── Envia "STATUS\r\n"
  ├── Lê VDD voltage (parse do valor em volts)
  └── Retorna PicoStatusResponse { connected, port_name, baud_rate, vdd_voltage, firmware_banner }
```

> [!IMPORTANT]
> O `PING` enviado (minúsculo `\r\nPING\r\n`) é convertido para uppercase no firmware (`toupper()`), portanto o firmware responde `PONG` corretamente.

### 1.5 Problemas Identificados no Firmware

| ID | Severidade | Localização | Problema |
|---|---|---|---|
| FW-01 | **MÉDIO** | `flasher.rs:74-78` | FamilyID do SDK (`0xE48BFF56`) não reconhecido como RP2040 — cai em `other` branch |
| FW-02 | **BAIXO** | `serial.rs:156-163` | `PING`/`STATUS` fazem dois `port.read()` bloqueantes separados (não usa `read_serial_response_complete`) — sujeito a timing race no Windows |
| FW-03 | **BAIXO** | `backup.rs:52-55` | `save_live_backup_files()` salva no diretório de trabalho corrente sem caminho absoluto — em produção o CWD pode não ser o esperado |
| FW-04 | **INFO** | `flasher.rs:46` | `!data.len().is_multiple_of(UF2_BLOCK_SIZE)` usa `is_multiple_of` que é `nightly` em Rust <1.73; em stable deve ser `data.len() % UF2_BLOCK_SIZE != 0` |

---

## 2. Rust Backend — Conectividade e Handshake

### 2.1 Fluxo Completo de Conexão Real (Hardware)

```
Frontend (TS)                    Tauri IPC                    Rust Backend
─────────────                    ─────────                    ────────────
toggleConnection()
  └─ invokeCommand('pico_connect_and_test', {port_name, baud_rate: 115200})
                                    │
                                    └─► hardware.rs::pico_connect_and_test()
                                           └─► serial.rs::pico_connect_and_test()
                                                  ├─ serialport::new(port, 115200).open()
                                                  ├─ write("\r\nPING\r\n")
                                                  ├─ read(1024B) → banner
                                                  ├─ write("STATUS\r\n")
                                                  ├─ read(512B) → vdd_text → parse float
                                                  └─ return PicoStatusResponse { connected: true, ... }
                                    │
                           setDeviceStatus({ connected: true, vdd_volts, firmware_version })
```

### 2.2 Fluxo Completo de Handshake DIGA

```
Frontend: sendCommand("HANDSHAKE") ou executeFirstContactBackup()
  └─► pico_send_serial_command(port, "HANDSHAKE")
        └─► serial.rs::pico_send_raw_serial_command()
              ├─ write("HANDSHAKE\r\n")
              └─ read_serial_response_complete(timeout=8s)
                   └─ aguarda: "[HANDSHAKE] SUCCESS|FAILED|ABORT|ACTIVE"

  ou

  executeFirstContactBackup()
    └─► backup.rs::pico_execute_first_contact_backup()
          ├─ open_and_handshake_dwire()      ← abre porta + envia HANDSHAKE
          ├─ parse_autodetected_nvram_base() ← extrai endereço NVRAM do texto
          ├─ dump_dwire_region_bytes(nvram1) ← DUMP hex → bytes (read 1)
          ├─ dump_dwire_region_bytes(nvram2) ← DUMP hex → bytes (read 2)
          ├─ sha256(nvram1) == sha256(nvram2) → double-read verification
          ├─ dump SRAM 0x00000000
          ├─ dump IPL  0x80000000
          └─ return FirstContactBackupResult
```

### 2.3 Problemas de Conectividade Identificados

| ID | Severidade | Localização | Problema |
|---|---|---|---|
| CN-01 | **ALTO** | `serial.rs:275-281` | `core_state` ERRADO: retorna `HALTED` quando `target_powered=true` e `RUNNING` quando `target_powered=false` — é o inverso; quando alimentado sem OCD ativo é RUNNING, quando halted pelo OCD é HALTED mas o alvo PODE estar powered |
| CN-02 | **MÉDIO** | `serial.rs:156-163` | Primeira conexão usa `port.read()` raw (buffer único, sem retry) em vez de `read_serial_response_complete()` — pode truncar resposta do banner em 64B USB |
| CN-03 | **MÉDIO** | `erase/executor.rs:115` | `voltage_before: 3.30` e `voltage_after: 3.29` são hardcoded no executor real — não lê ADC antes/depois da operação |
| CN-04 | **BAIXO** | `hardware.rs:56-58` | `pico_get_status` chama `pico_connect_and_test()` internamente a cada poll — abre e fecha a porta serial a cada 1500ms se a UI fizer polling |

---

## 3. Frontend / ViewModel — Usabilidade

### 3.1 Fluxo de UI na Primeira Conexão

```
PicoBenchView
  └─► PicoBenchTopBar
        ├─ Col 2: Dropdown porta serial (vazio = "Nenhuma porta serial detectada")
        ├─ Col 3: Botão "Conectar Serial" (cinza se !selectedPort)
        ├─ [Tier 1] Botão "Flash Pico (BOOTSEL)" → pico_flash_firmware
        └─ [Tier 1] Botão "BOOTSEL Reset" → pico_trigger_bootsel_reboot
```

**Fluxo correto na prática:**
1. Plugar Pico com firmware DWire32A → aparece como COM port
2. Clicar ↻ Atualizar → `list_pico_serial_ports()` popula dropdown
3. Selecionar porta COM → `selectedPort` atualizado
4. Clicar "Conectar Serial" → `toggleConnection()` → `pico_connect_and_test()`
5. Badge muda para "CONECTADO" ✅ → botões "Backup 1º Contato" e "Injetar & Limpar" ficam habilitados

### 3.2 Problemas de Usabilidade Identificados

| ID | Severidade | Componente | Problema |
|---|---|---|---|
| UX-01 | **ALTO** | `PicoBenchTopBar.tsx:61` | `pico_flash_firmware({})` não passa `port_name` → `target_family: None` → auto-detecta drive BOOTSEL; mas se o usuário ainda não clicou "BOOTSEL Reset", não há drive montado → erro silencioso |
| UX-02 | **MÉDIO** | `PicoBenchTopBar.tsx:145-171` | Botão "BOOTSEL Reset" não verifica se dispositivo está conectado antes de enviar 1200-baud touch — se porta inválida, o erro é capturado mas não orienta o usuário a conectar primeiro |
| UX-03 | **MÉDIO** | `usePicoDevice.ts:98-107` | `refreshPorts` auto-seleciona `realPorts[0]` mas não filtra portas com prefixo VID/PID label — pode selecionar `"COM3 [Raspberry Pi Pico (RP2040) DWire32A Host - VID:2E8A PID:000A]"` como `selectedPort`, e `extract_port_name()` vai extrair corretamente `COM3`, então está OK |
| UX-04 | **MÉDIO** | `mockPico.ts:29-57` | `mockInitialSerialLogs` mostra logs de handshake com `connected: true` como estado inicial — em Tauri real não há log inicial até a primeira conexão; pode confundir usuário em demo |
| UX-05 | **BAIXO** | `PicoBenchView.tsx:57-72` | Notificação de backup exibe `backupResult.sram_bytes` como "SRAM" mas pode incluir `ipl_bytes` não listados; mensagem incompleta para o usuário |
| UX-06 | **BAIXO** | `usePicoDevice.ts:263-268` | Alerta de tensão VDD: threshold `3.0V–3.35V` muito estreito — target alimentado por fonte DC tipicamente varia entre `3.1V` e `3.4V`; falsos alertas possíveis |

### 3.3 Estado do Fluxo de Flash de Firmware (Pipeline Completo)

```
Usuário clica "BOOTSEL Reset" (com Pico conectado em COMX):
  1. pico_trigger_bootsel_reboot(port_name: "COM3")
     └─► flasher.rs::trigger_1200_baud_bootsel("COM3")
           ├─ serialport::new("COM3", 1200).open()
           ├─ write_data_terminal_ready(false)
           ├─ write_request_to_send(false)
           ├─ sleep(150ms)
           └─ drop(port) → sleep(800ms)  [espera USB re-enumerar como MSD]
  2. Toast "BOOTSEL Reboot Initiated" ✅
  3. Pico reinicia como RPI-RP2 drive (D:\)

Usuário clica "Flash Pico (BOOTSEL)":
  4. pico_flash_firmware({})
     └─► flasher.rs::flash_embedded_firmware(None)
           ├─ detect_bootsel_drives()  → ["D:\\"]
           ├─ read INFO_UF2.TXT → detecta RP2040 vs RP2350
           ├─ seleciona EMBEDDED_RP2040_UF2 ou EMBEDDED_RP2350_UF2
           ├─ validate_uf2_buffer(payload)  ← aqui a family ID cai em "other"
           └─ fs::write("D:\\panadiga_dwire32a.uf2", bytes)  ← Pico reinicia
  5. Toast "Firmware Pico Gravado ✅ (228 blocks, 116736 bytes)"
  6. Pico reinicia rodando DWire32A firmware
  7. Usuário clica ↻ Atualizar → COM port aparece → "Conectar Serial"
```

> [!TIP]
> O fluxo completo **funciona corretamente** quando executado em sequência. A única fricção é que o usuário precisa aguardar ~1-3s entre "BOOTSEL Reset" e "Flash Pico" para o drive aparecer.

---

## 4. Pipeline `run.cmd` — Primeira Execução

### 4.1 Estado atual

```cmd
run.cmd
  ├─ Verifica dist\panadiga.exe  → existe ✅ (14.1 MB)
  ├─ Verifica dist\WebView2Loader.dll → existe ✅ (165 KB, x64)
  └─ Executa dist\panadiga.exe
```

### 4.2 Problema de Timestamp no `dist\WebView2Loader.dll`

A DLL foi copiada de `target\release\build\webview2-com-sys-*\out\x64\WebView2Loader.dll` que tem timestamp original de `1973-11-29` (unix epoch zero). Isso causou o erro `DateTimeOffset não pode ser convertido em carimbo ZIP` ao tentar criar `panadiga-windows-x86_64.zip`.

**Fix aplicado nesta sessão:** timestamp corrigido via `LastWriteTime = Get-Date`. O `SHA256SUMS.txt` foi gerado mas o `.zip` **não foi criado** (Compress-Archive falhou silenciosamente porque a DLL não existia ainda na pasta `dist\` no momento da execução do script PowerShell inline).

### 4.3 Build Summary

| Etapa | Status | Output |
|---|---|---|
| [1/4] Frontend Vite | ✅ OK | `apps/desktop/dist/` (1746 módulos, 687ms) |
| [2/4] Ghidra SSOT | ✅ OK | `tools/` (Python scripts exportados) |
| [3/4] Rust Release | ✅ OK | `target/release/panadiga.exe` (1m44s, 20 cores) |
| ARM GCC opcional | ✅ SKIP | cmake não encontrado → usando UF2 pré-compilados embutidos |
| [4/4] Packaging | ⚠️ PARCIAL | `dist/panadiga.exe` ✅, `dist/WebView2Loader.dll` ✅, `.zip` ❌ (erro de timestamp ZIP) |

---

## 5. Resumo de Achados e Prioridades

### CRÍTICO (bloqueia uso em produção)
> Nenhum achado crítico que impeça a operação básica.

### ALTO
| ID | Descrição | Fix |
|---|---|---|
| CN-01 | `core_state` invertido: `HALTED` quando powered, `RUNNING` quando despowered | Trocar lógica: `connected && !target_powered → RUNNING`, `connected && target_powered → HALTED` |
| UX-01 | "Flash Pico" sem feedback de "drive não encontrado" | Detectar drives antes, mostrar toast informativo se vazio |

### MÉDIO
| ID | Descrição | Fix |
|---|---|---|
| FW-01 | FamilyID do SDK Pico (`0xE48BFF56`) não reconhecido em `validate_uf2_buffer` | Adicionar `0xE48BFF56` e `0xE48BFF57` como constantes reconhecidas |
| CN-02 | `pico_connect_and_test` usa `read()` raw em vez de `read_serial_response_complete` | Substituir por read completo com retry |
| CN-03 | Tensão pré/pós erase hardcoded (`3.30V`, `3.29V`) | Ler ADC via `STATUS` antes e depois |
| BUILD-01 | `dist/panadiga-windows-x86_64.zip` não gerado por erro timestamp DLL | Corrigir timestamp antes do `Compress-Archive`, ou usar `7z` |

### BAIXO / INFO
| ID | Descrição |
|---|---|
| FW-02 | Leitura de PING/STATUS sem `read_serial_response_complete` |
| FW-03 | Backup salvo no CWD relativo |
| FW-04 | `is_multiple_of` pode ser `nightly-only` em Rust < 1.73 |
| UX-02 | BOOTSEL Reset sem verificação de conexão prévia |
| UX-04 | mockInitialLogs aparecem como se já conectado |
| UX-06 | Threshold VDD muito estreito (3.0–3.35V) |

---

## 6. Próximos Fixes Recomendados (em ordem)

```
1. flasher.rs: adicionar 0xE48BFF56 / 0xE48BFF57 como Family IDs reconhecidos
2. serial.rs: corrigir lógica core_state (CN-01)
3. serial.rs: usar read_serial_response_complete em pico_connect_and_test (CN-02)
4. build.cmd: corrigir timestamp da DLL antes do Compress-Archive + fix ZIP (BUILD-01)
5. PicoBenchTopBar: verificar detect_bootsel_drives antes de flash (UX-01)
```

---

## 7. Verificação de Integridade do Executável

| Artefato | Tamanho | Timestamp |
|---|---|---|
| `dist/panadiga.exe` | 14 813 696 B (14.1 MB) | 2026-11-11 08:12 |
| `dist/WebView2Loader.dll` | 165 336 B (x64) | 2026-11-11 08:13 (corrigido) |
| `dist/run.cmd` | presente | presente |
| `dist/firmware/` | vazio (cmake ausente) | — |
| `dist/releases/*.zip` | **AUSENTE** ❌ | erro de criação |

> [!CAUTION]
> O `.zip` de release não foi gerado. Para distribuir, é necessário rodar o fix do BUILD-01 ou criar manualmente o zip.
