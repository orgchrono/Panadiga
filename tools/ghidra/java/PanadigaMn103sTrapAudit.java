// MN103S SLEIGH Trap #8 (am33.sinc SUB bugs) & Historical Probe Alignment Auditor
// Read-only structural auditor across curated FAT/SD functions or all executable memory blocks
//@author Panadiga Engineering Team
//@category Panadiga.MN103S
//@description Scans MN103S instructions for am33.sinc Trap #8 SUB bugs (fb 9c operand inversion & fb/fd/fe 98 ADD-for-SUB) and verifies legacy probe alignments.

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSet;
import ghidra.program.model.listing.Function;
import panadiga.Mn103sAuditSupport;
import panadiga.Mn103sAuditSupport.CliArgs;
import panadiga.Mn103sAuditSupport.ProbeDiagnostic;
import panadiga.Mn103sAuditSupport.ResolvedFunction;
import panadiga.Mn103sAuditSupport.TargetEntry;
import panadiga.Mn103sAuditSupport.TrapFinding;

import java.util.List;
import java.util.Locale;

public class PanadigaMn103sTrapAudit extends GhidraScript {

    @Override
    public void run() throws Exception {
        if (currentProgram == null) {
            printerr("No active program loaded.");
            return;
        }

        String scope = parseScope(getScriptArgs());
        String stage = Mn103sAuditSupport.detectStage(currentProgram);
        AddressSet targetSet = buildTargetSet(scope, stage);

        println("################################################################################");
        println(String.format(
            "PanadigaMn103sTrapAudit | program=%s | stage=%s | scope=%s | coveredBytes=%d",
            currentProgram.getName(), stage, scope, targetSet.getNumAddresses()));
        println("################################################################################");

        List<TrapFinding> findings = Mn103sAuditSupport.scanRangeForSubTraps(currentProgram, targetSet, monitor);
        println(String.format("Total am33.sinc Trap #8 SUB findings: %d", findings.size()));
        for (TrapFinding finding : findings) {
            println("  * " + Mn103sAuditSupport.formatTrapCompact(finding));
        }

        if ("boot".equals(stage)) {
            reportHistoricBootProbes();
        }
    }

    private AddressSet buildTargetSet(String scope, String stage) {
        if ("exec".equals(scope)) {
            return new AddressSet(currentProgram.getMemory().getExecuteSet());
        }
        AddressSet set = new AddressSet();
        List<TargetEntry> entries = Mn103sAuditSupport.selectCatalogTargets(stage, "all");
        for (TargetEntry entry : entries) {
            Address addr = Mn103sAuditSupport.toAddress(currentProgram, entry.address());
            if (!Mn103sAuditSupport.containsAddress(currentProgram, addr)) {
                continue;
            }
            ResolvedFunction rf = Mn103sAuditSupport.resolveFunction(currentProgram, addr, this, false);
            if (rf.function() != null) {
                set.add(rf.function().getBody());
            }
        }
        return set;
    }

    private void reportHistoricBootProbes() {
        println("--------------------------------------------------------------------------------");
        println("Historical BOOT.BIN probe alignment audit (read-only resolution):");
        long[] probes = { 0x4400d335L, 0x4400c436L, 0x4400c760L };
        for (long probeAddr : probes) {
            Address addr = Mn103sAuditSupport.toAddress(currentProgram, probeAddr);
            ProbeDiagnostic diag = Mn103sAuditSupport.lookupHistoricProbe(probeAddr);
            ResolvedFunction rf = Mn103sAuditSupport.resolveFunction(currentProgram, addr, this, false);
            Function fn = rf.function();
            String resolvedText = fn != null ? (fn.getName() + " (" + rf.note() + ")") : rf.note();
            println(String.format("  - 0x%08x -> %s", probeAddr, resolvedText));
            if (diag != null) {
                println("    context: " + diag.explanation());
            }
        }
    }

    private static String parseScope(String[] args) {
        CliArgs cli = Mn103sAuditSupport.parseCliArgs(args);
        String scopeFlag = cli.flag("scope", "");
        if (!scopeFlag.isBlank()) {
            String clean = scopeFlag.toLowerCase(Locale.ROOT);
            if ("exec".equals(clean) || "catalog".equals(clean)) {
                return clean;
            }
        }
        for (String pos : cli.positional()) {
            String clean = pos.toLowerCase(Locale.ROOT);
            if ("exec".equals(clean) || "catalog".equals(clean)) {
                return clean;
            }
        }
        return "catalog";
    }
}
