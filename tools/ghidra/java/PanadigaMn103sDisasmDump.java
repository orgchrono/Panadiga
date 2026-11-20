// Consolidated MN103S Disassembly & Raw Opcode Range Dumper
// Replaces legacy C:\ghidra\DumpAsm.java (fixes mid-instruction null failures and emits raw bytes + Trap #8 annotations)
//@author Panadiga Engineering Team
//@category Panadiga.MN103S
//@description Dumps aligned MN103S disassembly with raw machine bytes, undefined-gap byte fallback, and SLEIGH Trap #8 SUB flags.

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import panadiga.Mn103sAuditSupport;
import panadiga.Mn103sAuditSupport.CliArgs;

import java.util.List;
import java.util.Locale;

public class PanadigaMn103sDisasmDump extends GhidraScript {

    private static final long DEFAULT_BOOT_START = 0x4400c755L;
    private static final long DEFAULT_BOOT_END = 0x4400c7b4L;
    private static final long PRESET_BOOT_LEGACY_START = 0x4400c760L;
    private static final long PRESET_BOOT_LEGACY_END = 0x4400c7b0L;
    private static final long PRESET_PROG061_TOC_START = 0x5040e9aeL;
    private static final long PRESET_PROG061_TOC_END = 0x5040ea7cL;

    @Override
    public void run() throws Exception {
        if (currentProgram == null) {
            printerr("No active program loaded.");
            return;
        }

        long[] bounds = resolveBounds(getScriptArgs());
        Address start = Mn103sAuditSupport.toAddress(currentProgram, bounds[0]);
        Address end = Mn103sAuditSupport.toAddress(currentProgram, bounds[1]);

        println("################################################################################");
        println(String.format(
            "PanadigaMn103sDisasmDump | program=%s | range=0x%08x..0x%08x",
            currentProgram.getName(), bounds[0], bounds[1]));
        println("################################################################################");

        if (!Mn103sAuditSupport.containsAddress(currentProgram, start)
                || !Mn103sAuditSupport.containsAddress(currentProgram, end)) {
            printerr(String.format(
                "Requested range 0x%08x..0x%08x lies outside memory map of %s.",
                bounds[0], bounds[1], currentProgram.getName()));
            return;
        }

        Mn103sAuditSupport.dumpDisassemblyRange(currentProgram, start, end, this);
    }

    private long[] resolveBounds(String[] args) {
        CliArgs cli = Mn103sAuditSupport.parseCliArgs(args);
        String range = cli.flag("range", "");
        if (!range.isBlank()) {
            String[] parts = range.split("\\.\\.|:");
            if (parts.length == 2) {
                return new long[] {
                    Mn103sAuditSupport.parseHex(parts[0]),
                    Mn103sAuditSupport.parseHex(parts[1])
                };
            }
            throw new IllegalArgumentException("Invalid --range syntax (use START..END): " + range);
        }

        List<String> pos = cli.positional();
        if (pos.size() >= 2) {
            return new long[] {
                Mn103sAuditSupport.parseHex(pos.get(0)),
                Mn103sAuditSupport.parseHex(pos.get(1))
            };
        }

        String preset = cli.flag("preset", "").toLowerCase(Locale.ROOT);
        if ("boot-legacy".equals(preset)) {
            return new long[] { PRESET_BOOT_LEGACY_START, PRESET_BOOT_LEGACY_END };
        }
        if ("prog061-toc".equals(preset)) {
            return new long[] { PRESET_PROG061_TOC_START, PRESET_PROG061_TOC_END };
        }
        if ("prog061".equals(Mn103sAuditSupport.detectStage(currentProgram)) && preset.isBlank()) {
            return new long[] { PRESET_PROG061_TOC_START, PRESET_PROG061_TOC_END };
        }
        return new long[] { DEFAULT_BOOT_START, DEFAULT_BOOT_END };
    }
}
