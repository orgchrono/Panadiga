// Consolidated MN103S Bootloader FAT/SD & PROG061 sd_load Decompiler Analyzer
// Replaces legacy loose C:\ghidra\DecompileFat*.java, DecompileSdRead.java & DecompileCardType.java
//@author Panadiga Engineering Team
//@category Panadiga.MN103S
//@description Consolidates all verified FAT12/16/32, SD OCR/CMD17/CMD18 and PROG061 sd_load.o targets with read-only function resolution and SLEIGH Trap #8 scanning.

import ghidra.app.script.GhidraScript;
import panadiga.Mn103sAuditSupport;
import panadiga.Mn103sAuditSupport.CliArgs;
import panadiga.Mn103sAuditSupport.TargetEntry;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;

public class PanadigaBootFatSdDecompiler extends GhidraScript {

    @Override
    public void run() throws Exception {
        if (currentProgram == null) {
            printerr("No active program loaded.");
            return;
        }

        CliArgs cli = Mn103sAuditSupport.parseCliArgs(getScriptArgs());
        String rawStage = cli.flag("stage", "auto").toLowerCase(Locale.ROOT);
        String effectiveStage = "auto".equals(rawStage)
            ? Mn103sAuditSupport.detectStage(currentProgram)
            : rawStage;
        String group = cli.flag("group", "all").toLowerCase(Locale.ROOT);
        int timeoutSeconds = Integer.parseInt(cli.flag("timeout", "60"));
        boolean allowCreate = cli.hasFlag("allow-create");
        boolean scanTraps = !cli.hasFlag("no-trap-scan");

        List<TargetEntry> targets = resolveTargets(cli, effectiveStage, group);
        if (targets.isEmpty()) {
            println(String.format(
                "No targets selected (program=%s, stage=%s, group=%s).",
                currentProgram.getName(), effectiveStage, group));
            return;
        }

        println("################################################################################");
        println(String.format(
            "PanadigaBootFatSdDecompiler | program=%s | stage=%s | group=%s | targets=%d | allowCreate=%s | trapScan=%s",
            currentProgram.getName(), effectiveStage, group, targets.size(), allowCreate, scanTraps));
        println("################################################################################");

        Mn103sAuditSupport.withDecompiler(currentProgram, decomp -> {
            for (TargetEntry entry : targets) {
                if (monitor.isCancelled()) {
                    break;
                }
                Mn103sAuditSupport.decompileAndPrint(
                    currentProgram,
                    decomp,
                    entry,
                    this,
                    timeoutSeconds,
                    allowCreate,
                    scanTraps);
            }
        });
    }

    private List<TargetEntry> resolveTargets(CliArgs cli, String effectiveStage, String group) {
        List<Long> customAddresses = new ArrayList<>();
        String addrFlag = cli.flag("addr", "");
        if (!addrFlag.isBlank()) {
            for (String token : addrFlag.split(",")) {
                if (!token.isBlank()) {
                    customAddresses.add(Mn103sAuditSupport.parseHex(token));
                }
            }
        }
        for (String pos : cli.positional()) {
            customAddresses.add(Mn103sAuditSupport.parseHex(pos));
        }

        if (!customAddresses.isEmpty()) {
            List<TargetEntry> custom = new ArrayList<>(customAddresses.size());
            Set<Long> seen = new LinkedHashSet<>();
            for (long raw : customAddresses) {
                if (seen.add(raw)) {
                    custom.add(new TargetEntry(raw, "custom_probe", "custom", "User-specified probe address"));
                }
            }
            return custom;
        }
        return Mn103sAuditSupport.selectCatalogTargets(effectiveStage, group);
    }
}
