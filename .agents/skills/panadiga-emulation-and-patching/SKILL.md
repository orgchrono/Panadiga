---
name: panadiga-emulation-and-patching
description: >-
  Patching binário Copy-on-Write seguro, verificação determinística no emulador Mn103Vm e no
  emulador de drive óptico Matshita R6-MV4, montagem de stubs SDRAM (PANAMON1>, TBC M0,
  dumpers NOR), ponte GDB RSP (gdb-bridge, rsp-codec) e limites entre emulação ISA e silício
  real. Use ao criar/validar patches de firmware, testar em VM ou operar depuração GDB.
---

# Emulação (`Mn103Vm` / `R6-MV4`), Patching CoW & Depuração GDB RSP

Esta skill conduz a geração segura de patches binários (CoW), a validação automatizada na VM `Mn103Vm` e a depuração interativa via GDB RSP sem expor o silício a comandos cegos.

## 1. Quando Usar & Handoffs
- Ative ao aplicar ou verificar patches de BGO, capacidade HDD (`>200 GB`), compatibilidade regional/`PAL-M`, AV passthrough TBC, stubs de captura NOR, `Matshita R6-MV4` ou sessões GDB RSP.
- Antes de construir um patch, confirme endereços/opcodes em `.agents/skills/panadiga-ghidra-workflow/SKILL.md`.
- Ao persistir artefatos recompilados, passe pelo bucket `recompiled/` em `.agents/skills/panadiga-forensic-archival/SKILL.md`.
- Para executar em hardware real, entregue para `.agents/skills/panadiga-bench-operations/SKILL.md`.
- Matriz de limites `Mn103Vm` vs silício, tabela de VAs verificados e protocolo `PANAMON1>` em [emulation-limits-and-silicon-abi.md](./references/emulation-limits-and-silicon-abi.md).

## 2. Invariantes Não-Negociáveis
1. **`Mn103Vm` ≠ Prova em Hardware Real (`N2_EMULATED` vs `N4_HARDWARE`):** A VM valida fluxo ISA `MN103S` e fronteiras estáticas de memória — **não** valida timing CFI Spansion (`~700 ms`), mascaramento atômico `CLI`, coerência I/D-cache, DMA ou watchdog `IC7501`.
2. **Proibição de Escrita no IPL & MMIO Letal:** `rsp-codec` e `gdb-bridge` mantêm `SA0..SA10` (`0x80000000..0x8003FFFF` / `0x84000000..0x8403FFFF`) bloqueados (`E05`) e proíbem acesso a `0xC000_100C`.
3. **Armadilha GDB vs `PANAMON1>`:** **JAMAIS** envie `$m...#B1` quando o alvo está parado em `PANAMON1>` — o caractere `'B'` em `#B1` aciona `'B'` (`OK:BYPASS`) no monitor! Sempre sonde com `?\r` antes e escolha `--target-mode` exato.
4. **Enquadramento Honesto de `PANAMON2` (`firmware/panadiga-blackbox-fw`):** Protótipo experimental de interoperabilidade em SDRAM (`0x50400000`), **não** *clean-room* jurídico; compilável apenas em container `mn10300-elf-gcc`.

## 3. Workflow Canônico (CLI, Launchers & MCP)

### A. Suíte E2E em `Mn103Vm` & Patches CoW por Subsistema
```bash
# 1. Suíte determinística completa (CFI/IPL E05, FRM retarget, BGO/PANAMON1, TBC E-E, região, HDD >200GB)
cargo run -q -p panadiga-cli -- re verify-patches --json

# 2. Patches CoW individuais (sempre saída -o separada)
cargo run -q -p panadiga-cli -- unbrick bgo -i tmp/PROG061.bin -o tmp/PROG061_mon.bin -m uart_monitor -j -
cargo run -q -p panadiga-cli -- hdd patch-capacity -i tmp/MAIN.bin -o tmp/MAIN_250.bin -m dynamic -g 250
cargo run -q -p panadiga-cli -- unbrick passthrough -i tmp/MAIN.bin -o tmp/MAIN_tbc.bin -m av_passthrough -j -
cargo run -q -p panadiga-cli -- unbrick tbc-plan --mode stage15_digadiag_tbc --standard pal-m -j -
cargo run -q -p panadiga-cli -- unbrick nor-stub --start 0x84000000 --len 0x800000 -o tmp/nor_stub.bin --wrap-digadiag -j -

# 3. Compatibilidade regional & bundle TV (ifodev2 tail 0x400..0x7FF permanece read-only)
cargo run -q -p panadiga-cli -- unbrick region-compat inspect-tv-bundle --preset br-pal-m --dev-bin firmware_and_images/eh55_ifodev2_m6930lb_2048b.bin -j -
cargo run -q -p panadiga-cli -- unbrick region-compat patch-main -i tmp/MAIN.bin -o tmp/MAIN_unlock.bin --mode bypass -j -
cargo run -q -p panadiga-cli -- unbrick region-compat retarget-frm -i tmp/orig.FRM -o tmp/ret.FRM --hw M6930 --oem M -j -
```

### B. Emulador Matshita `R6-MV4` & Ponte GDB RSP
```bash
# Emular boot óptico ATAPI 0x3B/0xED e serial CLI DRV (PASS / SLOGON)
cargo run -q -p panadiga-cli -- unbrick matshita-emu boot --scenario unlock_disc_frm -j -
cargo run -q -p panadiga-cli -- unbrick matshita-emu serial-cmd --unlock-pass "PASS 0x4d415453 0x48495441" -j -

# Ponte GDB RSP via Pico & validação segura de frames
cargo run -q -p panadiga-cli -- pico gdb-bridge -p COM10 --listen 127.0.0.1:3333 --target-mode panamon1
cargo run -q -p panadiga-cli -- gdb-config -p COM10 -t 3333
cargo run -q -p panadiga-cli -- pico rsp-codec --kind is-ipl-readonly --addr 0x84000000 --json
# Launchers Windows: tools\iniciar_ponte_gdb.cmd | tools\conectar_gdb_multiarch.cmd
```
- **MCP:** `panadiga_bgo_unbrick`, `panadiga_main_passthrough`, `panadiga_region_compat`, `panadiga_tbc_appliance_plan`, `panadiga_nor_dumper_stub`, `panadiga_sd_unbrick_kit(action="matshita_emu")`.

## 4. Armadilhas & Anti-Padrões
- Esquecer `CLI` (mascaramento atômico de IRQ) e tabela de vetores em SDRAM ao gerar código que tira a NOR de `Read Array`.
- Escrever stub à mão violando a ABI `CALL target, [], 0x00` ou usando polling infinito em `SC0STR` (`0xD400200C`).
- Usar `--allow-unsafe-bgo-format` ou `--experimental-ram-chainload` sem entender que removem travas de segurança offline.

## 5. Daqui pra Frente & Verificação
- **Fronteira aberta:** Provar em silício real (além do `Mn103Vm`) `Stage15DigadiagTbc`, `AIOF`/`IC3001` `PAL-M` e keepalive `M0` `TBUS` (`EV-20261120-014`/`015`/`017`).
- **Verificação:** `panadiga re verify-patches --json` com `100%` dos cenários aprovados (`0` violações `E05`).
