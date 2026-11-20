package panadiga;

import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressFactory;
import ghidra.program.model.address.AddressSetView;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.listing.Program;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryAccessException;
import ghidra.program.model.pcode.HighFunction;
import ghidra.util.exception.CancelledException;
import ghidra.util.task.TaskMonitor;

import java.util.ArrayList;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;

/**
 * Shared support utilities, verified Panasonic MN103S target catalogs, and SLEIGH Trap #8 detectors
 * for Panadiga's consolidated Ghidra Java analyzers.
 */
public final class Mn103sAuditSupport {

    private Mn103sAuditSupport() {}

    /** Curated function target entry across BOOT.BIN (ver0.63) and PROG061 (ver0.61). */
    public record TargetEntry(long address, String symbol, String group, String description) {}

    /** Result of resolving a target address to a Ghidra Function without blind mutation. */
    public record ResolvedFunction(
            Function function,
            Address requestedAddress,
            long offsetFromEntry,
            boolean createdNow,
            String note) {}

    /** MN103S AM33/AM34 SLEIGH Trap #8 finding from instruction byte patterns. */
    public record TrapFinding(
            Address address,
            String trapKind,
            String rawHex,
            String disassembly,
            String hardwareSemantics,
            String sleighBugNote,
            String containingFunction) {}

    /** Diagnostic metadata for historical misaligned probe addresses from early ad-hoc scripts. */
    public record ProbeDiagnostic(long probeAddress, long realEntry, String explanation) {}

    /** Normalized CLI arguments supporting both {@code --k=v} and PowerShell-split {@code --k v}. */
    public record CliArgs(Map<String, String> flags, List<String> positional) {
        public String flag(String key, String fallback) {
            return flags.getOrDefault(key.toLowerCase(Locale.ROOT), fallback);
        }

        public boolean hasFlag(String key) {
            return flags.containsKey(key.toLowerCase(Locale.ROOT));
        }
    }

    public static CliArgs parseCliArgs(String[] args) {
        Map<String, String> flags = new LinkedHashMap<>();
        List<String> positional = new ArrayList<>();
        if (args == null) {
            return new CliArgs(flags, positional);
        }
        for (int i = 0; i < args.length; i++) {
            String raw = args[i];
            if (raw == null || raw.isBlank()) {
                continue;
            }
            String trimmed = raw.trim();
            if (!trimmed.startsWith("--")) {
                positional.add(trimmed);
                continue;
            }
            int eq = trimmed.indexOf('=');
            if (eq > 2) {
                String key = trimmed.substring(2, eq).toLowerCase(Locale.ROOT);
                flags.put(key, trimmed.substring(eq + 1));
                continue;
            }
            String key = trimmed.substring(2).toLowerCase(Locale.ROOT);
            boolean hasNextValue = i + 1 < args.length
                && args[i + 1] != null
                && !args[i + 1].trim().startsWith("--")
                && !isBooleanSwitch(key);
            if (hasNextValue) {
                flags.put(key, args[++i].trim());
            } else {
                flags.put(key, "true");
            }
        }
        return new CliArgs(Collections.unmodifiableMap(flags), Collections.unmodifiableList(positional));
    }

    private static boolean isBooleanSwitch(String key) {
        return "allow-create".equals(key) || "no-trap-scan".equals(key);
    }

    private static final List<TargetEntry> BOOT_TARGETS = List.of(
        // Stage / SD loader top-level & debug logging
        new TargetEntry(0x4400a97cL, "sd_diag_load", "boot-loader", "Loads 0x20000 B of DIGADIAG.FRM into 0x93fc0000"),
        new TargetEntry(0x440085d9L, "sd_diag_boot_jump", "boot-loader", "Decompresses / launches secondary image at 0x50400000"),
        new TargetEntry(0x4400b3d9L, "fat_sd_debug_printf", "fat-api", "FAT/SD debug printf wrapper forwarding to uart0_printf"),
        // Public FAT API
        new TargetEntry(0x4400c696L, "boot_fat_mount", "fat-api", "Public FAT mount & FAT12 (<0xff6) / FAT16 (<=0xfff5) / FAT32 reject"),
        new TargetEntry(0x4400c879L, "boot_fat_open", "fat-api", "Public FAT file open"),
        new TargetEntry(0x4400c922L, "boot_fat_read", "fat-api", "Public FAT file read"),
        new TargetEntry(0x4400c94dL, "boot_fat_seek", "fat-api", "Public FAT file seek"),
        new TargetEntry(0x4400c984L, "boot_fat_close", "fat-api", "Public FAT handle close (slot 200)"),
        new TargetEntry(0x4400c9c0L, "boot_fat_seek_internal", "fat-api", "Internal seek implementation (whence 1/2/3)"),
        // Mount / MBR / BPB / VBR & Directory parsing
        new TargetEntry(0x4400cb21L, "boot_fat_mount_mbr_vbr", "fat-mount", "Reads LBA 0 MBR / partition table and VBR (+6 FAT32 backup retry)"),
        new TargetEntry(0x4400c0a8L, "boot_fat_validate_vbr_bpb", "fat-mount", "Validates 512 B BPB signature 0x55AA, BytesPerSec=0x200, FAT12/16 strings"),
        new TargetEntry(0x4400c35aL, "boot_fat_bpb_helper_c35a", "fat-mount", "BPB/VBR helper (contains legacy probe address 0x4400c436)"),
        new TargetEntry(0x4400ced2L, "boot_fat_cache_init", "fat-mount", "Initializes 3-sector FAT table cache window"),
        new TargetEntry(0x4400cfdeL, "boot_fat_parse_8_3_name", "fat-dir", "Normalizes filename to 8.3 upper-case format"),
        new TargetEntry(0x4400d11eL, "boot_fat_alloc_handle", "fat-dir", "Allocates single file handle slot 200 (-10011 if busy)"),
        new TargetEntry(0x4400d16bL, "boot_fat_open_internal", "fat-dir", "Internal file open + 'ASF' special path vs standard FAT"),
        new TargetEntry(0x4400dce2L, "boot_fat_dir_scan", "fat-dir", "Clustered FAT directory lookup"),
        new TargetEntry(0x4400e51fL, "boot_fat_cluster_to_lba", "fat-dir", "Converts FAT cluster index to partition-relative LBA"),
        new TargetEntry(0x4400e592L, "boot_fat_root_dir_lookup", "fat-dir", "Fixed FAT12/FAT16 root directory lookup"),
        new TargetEntry(0x4400c510L, "boot_fat_unpack_direntry", "fat-dir", "Unpacks 32 B FAT directory entry"),
        // File I/O, FAT cluster chain & block staging
        new TargetEntry(0x4400d32fL, "boot_fat_read_internal", "fat-io", "Main FAT read loop (contains legacy probe address 0x4400d335)"),
        new TargetEntry(0x4400dfc8L, "boot_fat_read_asf", "fat-io", "Contiguous/ASF special file read path"),
        new TargetEntry(0x4400e737L, "boot_fat_next_cluster", "fat-io", "3-sector FAT cache lookup for next cluster (FAT12/16/32)"),
        new TargetEntry(0x4400edbfL, "boot_fat_scan_contig_clusters", "fat-io", "Bounded contiguous cluster run scanner"),
        new TargetEntry(0x4400ef58L, "boot_fat_scan_contig_alt", "fat-io", "Alternative contiguous cluster scanner"),
        new TargetEntry(0x4400dc64L, "boot_fat_read_part_sector", "fat-io", "Partition-relative sector read wrapper (adds PartitionStartLBA)"),
        new TargetEntry(0x4400ca59L, "boot_fat_read_sectors_chunked", "fat-io", "Chunked <=0x1fc-sector SD reader staging via 0x91c00000"),
        // Block device dispatcher & SD low-level hardware driver
        new TargetEntry(0x440098fcL, "blkdev_read_dispatch", "sd-driver", "Block device dispatcher (dev 0 -> FUN_44009980; dev 3 -> SD)"),
        new TargetEntry(0x440262cbL, "sd_read_blocks_262cb", "sd-driver", "SD controller block read (Card type 2 byte/block LBA, CMD17/CMD18)"),
        new TargetEntry(0x4402724fL, "sd_send_ocr_rca_2724f", "sd-driver", "SD OCR (ACMD41), MMC fallback (CMD1), Card type 1/2/3 & CMD2/CMD3 RCA")
    );

    private static final List<TargetEntry> PROG061_TARGETS = List.of(
        new TargetEntry(0x5040e9aeL, "sd_decrypt_toc_48b", "prog061-sd", "Decrypts 48-byte SD firmware header / TOC block"),
        new TargetEntry(0x5040ea30L, "sd_list_sum_48b", "prog061-sd", "Validates 48-byte SD table header checksum (prologue at 0x5040ea1c)"),
        new TargetEntry(0x5040ea7dL, "sd_find_firm_entry", "prog061-sd", "Finds matching model/version entry in parsed TOC"),
        new TargetEntry(0x5040eb35L, "bytes_sum32", "prog061-sd", "32-bit additive checksum helper for FMU/SD images"),
        new TargetEntry(0x5040eb78L, "dvdram_read_eb78", "prog061-sd", "DVD-RAM sector read helper for Update-Disc"),
        new TargetEntry(0x5040ebfeL, "sd_open_pana_sd_frm", "prog061-sd", "Opens PANA__SD.FRM and parses 48-byte TOC"),
        new TargetEntry(0x5040eca9L, "prog061_sd_diag_load", "prog061-sd", "Loads DIGADIAG.FRM (0x40000 B -> 0x90400000) and PANA__SD.FRM"),
        new TargetEntry(0x5040ed7cL, "sd_read_firm_list", "prog061-sd", "Reads PANA__SD.FRM list table for Update-Disc"),
        new TargetEntry(0x5040fe60L, "prog061_fat_debug_printf", "prog061-sd", "PROG061 FAT/SD debug print wrapper"),
        new TargetEntry(0x50404dcdL, "prog061_uart0_printf", "prog061-sd", "PROG061 UART0 formatted console print"),
        new TargetEntry(0x50405b8cL, "prog061_update_disc_scan", "prog061-sd", "Update-Disc / SD lookup handler"),
        new TargetEntry(0x50405cd6L, "prog061_update_disc_exec", "prog061-sd", "Update-Disc / SD flash update sequence"),
        new TargetEntry(0x50406129L, "prog061_update_disc_tail", "prog061-sd", "Update-Disc / Drive-Firm post handler"),
        new TargetEntry(0x50407bd8L, "p61_verify_and_stage_fmu_7bd8", "prog061-sd", "Verifies FMU checksum and stages payload before NOR write")
    );

    private static final Map<Long, ProbeDiagnostic> HISTORIC_PROBES = buildProbeDiagnostics();

    private static Map<Long, ProbeDiagnostic> buildProbeDiagnostics() {
        Map<Long, ProbeDiagnostic> map = new LinkedHashMap<>();
        map.put(0x4400d335L, new ProbeDiagnostic(
            0x4400d335L, 0x4400d32fL,
            "Legacy DecompileFat.java probe: byte +3 inside [fa fe 7c ff] ADD -0x84,SP of FUN_4400d32f"));
        map.put(0x4400c436L, new ProbeDiagnostic(
            0x4400c436L, 0x4400c35aL,
            "Legacy DecompileFat3.java probe: byte +1 inside [f8 03 0c] MOV (0xc,A3),D0 of helper 0x4400c35a..0x4400c50f"));
        map.put(0x4400c760L, new ProbeDiagnostic(
            0x4400c760L, 0x4400c75fL,
            "Legacy DumpAsm.java start: byte +1 inside [f8 11 18] MOV D0,(0x18,A1) in FUN_4400c696"));
        return Collections.unmodifiableMap(map);
    }

    public static List<TargetEntry> bootTargets() {
        return BOOT_TARGETS;
    }

    public static List<TargetEntry> prog061Targets() {
        return PROG061_TARGETS;
    }

    public static ProbeDiagnostic lookupHistoricProbe(long address) {
        return HISTORIC_PROBES.get(address);
    }

    public static String detectStage(Program program) {
        if (program == null) {
            return "boot";
        }
        String name = program.getName().toLowerCase(Locale.ROOT);
        Address minAddr = program.getMinAddress();
        if (name.contains("prog061") || (minAddr != null && minAddr.getOffset() >= 0x50000000L && minAddr.getOffset() < 0x51000000L)) {
            return "prog061";
        }
        return "boot";
    }

    public static List<TargetEntry> selectCatalogTargets(String stage, String groupFilter) {
        List<TargetEntry> pool = new ArrayList<>();
        if ("boot".equals(stage) || "all".equals(stage)) {
            pool.addAll(BOOT_TARGETS);
        }
        if ("prog061".equals(stage) || "all".equals(stage)) {
            pool.addAll(PROG061_TARGETS);
        }
        if (groupFilter == null || groupFilter.isBlank() || "all".equalsIgnoreCase(groupFilter)) {
            return pool;
        }
        String want = groupFilter.trim().toLowerCase(Locale.ROOT);
        List<TargetEntry> filtered = new ArrayList<>();
        for (TargetEntry entry : pool) {
            if (entry.group().equalsIgnoreCase(want)) {
                filtered.add(entry);
            }
        }
        return filtered;
    }

    public static long parseHex(String text) {
        String clean = text.trim();
        if (clean.startsWith("0x") || clean.startsWith("0X")) {
            clean = clean.substring(2);
        }
        return Long.parseUnsignedLong(clean, 16);
    }

    public static Address toAddress(Program program, long value) {
        AddressFactory af = program.getAddressFactory();
        return af.getDefaultAddressSpace().getAddress(value);
    }

    public static boolean containsAddress(Program program, Address addr) {
        return addr != null && program.getMemory().contains(addr);
    }

    public static ResolvedFunction resolveFunction(
            Program program,
            Address addr,
            GhidraScript script,
            boolean allowCreate) {
        FunctionManager fm = program.getFunctionManager();
        Function exact = fm.getFunctionAt(addr);
        if (exact != null) {
            return new ResolvedFunction(exact, addr, 0L, false, "exact entry");
        }

        Function containing = fm.getFunctionContaining(addr);
        if (containing != null) {
            long delta = addr.subtract(containing.getEntryPoint());
            return new ResolvedFunction(
                containing,
                addr,
                delta,
                false,
                String.format(
                    "offset +0x%x inside %s (entry 0x%s)",
                    delta, containing.getName(), Long.toHexString(containing.getEntryPoint().getOffset())));
        }

        Listing listing = program.getListing();
        Instruction containingInsn = listing.getInstructionContaining(addr);
        if (containingInsn != null && !containingInsn.getAddress().equals(addr)) {
            Function viaInsn = fm.getFunctionContaining(containingInsn.getAddress());
            if (viaInsn != null) {
                long delta = addr.subtract(viaInsn.getEntryPoint());
                return new ResolvedFunction(
                    viaInsn,
                    addr,
                    delta,
                    false,
                    String.format(
                        "mid-instruction (+%d B in %s @ 0x%s), offset +0x%x inside %s",
                        addr.subtract(containingInsn.getAddress()),
                        containingInsn.getMnemonicString(),
                        Long.toHexString(containingInsn.getAddress().getOffset()),
                        delta,
                        viaInsn.getName()));
            }
        }

        Function nearbyBefore = script.getFunctionBefore(addr);
        if (!allowCreate) {
            String hint = nearbyBefore != null
                ? String.format("no function covers 0x%s in DB (nearest prior: %s @ 0x%s..0x%s; pass --allow-create to define)",
                    Long.toHexString(addr.getOffset()),
                    nearbyBefore.getName(),
                    Long.toHexString(nearbyBefore.getEntryPoint().getOffset()),
                    Long.toHexString(nearbyBefore.getBody().getMaxAddress().getOffset()))
                : "no function in database (read-only mode)";
            return new ResolvedFunction(null, addr, -1L, false, hint);
        }

        if (program.getDomainFile() != null && program.getDomainFile().isReadOnly()) {
            return new ResolvedFunction(null, addr, -1L, false, "domain file is read-only; skipped createFunction");
        }

        try {
            if (listing.getInstructionContaining(addr) == null) {
                script.disassemble(addr);
            }
            Function created = script.createFunction(addr, null);
            if (created != null) {
                return new ResolvedFunction(created, addr, 0L, true, "disassembled + created function");
            }
            Function nowContaining = fm.getFunctionContaining(addr);
            if (nowContaining != null) {
                long delta = addr.subtract(nowContaining.getEntryPoint());
                return new ResolvedFunction(
                    nowContaining,
                    addr,
                    delta,
                    false,
                    String.format(
                        "resolved after disassemble: +0x%x inside %s",
                        delta, nowContaining.getName()));
            }
        } catch (Exception ex) {
            return new ResolvedFunction(null, addr, -1L, false, "createFunction failed: " + ex.getMessage());
        }
        return new ResolvedFunction(null, addr, -1L, false, "could not find or create function");
    }

    @FunctionalInterface
    public interface DecompilerAction {
        void accept(DecompInterface decomp) throws Exception;
    }

    /** Opens a configured DecompInterface and disposes it deterministically in finally. */
    public static void withDecompiler(Program program, DecompilerAction action) throws Exception {
        DecompInterface decomp = new DecompInterface();
        try {
            if (!decomp.openProgram(program)) {
                throw new IllegalStateException("DecompInterface failed to open " + program.getName());
            }
            action.accept(decomp);
        } finally {
            decomp.dispose();
        }
    }

    public static void decompileAndPrint(
            Program program,
            DecompInterface decomp,
            TargetEntry entry,
            GhidraScript script,
            int timeoutSeconds,
            boolean allowCreate,
            boolean scanTraps) {
        Address addr = toAddress(program, entry.address());
        ProbeDiagnostic probe = lookupHistoricProbe(entry.address());

        script.println("================================================================================");
        script.println(String.format(
            "TARGET 0x%08x | %s [%s] - %s",
            entry.address(), entry.symbol(), entry.group(), entry.description()));
        if (probe != null) {
            script.println("NOTE (Historic Probe): " + probe.explanation()
                + String.format(" (expected entry: 0x%08x)", probe.realEntry()));
        }

        if (!containsAddress(program, addr)) {
            script.println("SKIP: Address 0x" + Long.toHexString(entry.address())
                + " is outside memory map of " + program.getName());
            return;
        }

        ResolvedFunction resolved = resolveFunction(program, addr, script, allowCreate);
        Function func = resolved.function();
        if (func == null) {
            script.println("UNRESOLVED at 0x" + Long.toHexString(entry.address()) + " (" + resolved.note() + ")");
            printSurroundingDisassembly(program, addr, script);
            return;
        }

        script.println(String.format(
            "RESOLVED: %s @ 0x%08x (%s, bodySize=%d B)",
            func.getName(),
            func.getEntryPoint().getOffset(),
            resolved.note(),
            func.getBody().getNumAddresses()));
        script.println("================================================================================");

        if (scanTraps) {
            List<TrapFinding> traps = scanRangeForSubTraps(program, func.getBody(), TaskMonitor.DUMMY);
            if (!traps.isEmpty()) {
                script.println(String.format(
                    "[TRAP #8 ALERT] Detected %d AM33/MN103S SLEIGH SUB anomaly site(s) in %s:",
                    traps.size(), func.getName()));
                for (TrapFinding trap : traps) {
                    script.println("  -> " + formatTrapCompact(trap));
                }
                script.println("");
            }
        }

        DecompileResults res = decomp.decompileFunction(func, timeoutSeconds, script.getMonitor());
        if (res != null && res.decompileCompleted()) {
            HighFunction hf = res.getHighFunction();
            if (hf != null && res.getDecompiledFunction() != null) {
                script.println(res.getDecompiledFunction().getC());
                return;
            }
        }
        String err = res != null ? res.getErrorMessage() : "null DecompileResults";
        script.println("DECOMPILATION FAILED: " + err);
    }

    private static void printSurroundingDisassembly(Program program, Address target, GhidraScript script) {
        Instruction insn = program.getListing().getInstructionContaining(target);
        if (insn != null) {
            long delta = target.subtract(insn.getAddress());
            script.println(String.format(
                "MISALIGNED: falls +%d byte(s) inside instruction @ 0x%08x: [%s] %s",
                delta, insn.getAddress().getOffset(), formatInstructionBytes(insn), insn));
            return;
        }
        byte[] sample = new byte[16];
        try {
            int n = program.getMemory().getBytes(target, sample);
            if (n > 0) {
                byte[] trimmed = java.util.Arrays.copyOf(sample, n);
                script.println(String.format("RAW UNDEFINED BYTES @ 0x%08x: [%s]", target.getOffset(), formatBytes(trimmed)));
            }
        } catch (MemoryAccessException ex) {
            script.println("RAW BYTES UNAVAILABLE: " + ex.getMessage());
        }
    }

    /**
     * Scans instructions within {@code range} for MN103S SLEIGH Trap #8 patterns in am33.sinc:
     * <ul>
     *   <li>{@code fb 9c ..} (am33.sinc:184): 3-operand {@code SUB Rm2, Rn0, Rd2} swaps subtraction operands ({@code Rm2 - Rn0} instead of {@code Rn0 - Rm2}).</li>
     *   <li>{@code fb/fd/fe 98 ..} (am33.sinc:186-188): {@code SUB SIMM8/24/32, Rn0} calls {@code update_add} instead of {@code update_sub}.</li>
     * </ul>
     */
    public static List<TrapFinding> scanRangeForSubTraps(
            Program program,
            AddressSetView range,
            TaskMonitor monitor) {
        List<TrapFinding> findings = new ArrayList<>();
        Listing listing = program.getListing();
        FunctionManager fm = program.getFunctionManager();
        InstructionIterator it = listing.getInstructions(range, true);

        while (it.hasNext() && !monitor.isCancelled()) {
            Instruction inst = it.next();
            TrapFinding finding = classifyTrapInstruction(inst, fm);
            if (finding != null) {
                findings.add(finding);
            }
        }
        return findings;
    }

    private static TrapFinding classifyTrapInstruction(Instruction inst, FunctionManager fm) {
        byte[] raw;
        try {
            raw = inst.getBytes();
        } catch (MemoryAccessException ex) {
            return null;
        }
        if (raw.length < 2) {
            return null;
        }
        int b0 = raw[0] & 0xFF;
        int b1 = raw[1] & 0xFF;

        Function fn = fm.getFunctionContaining(inst.getAddress());
        String fnName = fn != null ? fn.getName() : "<no_function>";
        String hex = formatBytes(raw);

        if (b0 == 0xFB && b1 == 0x9C && inst.getNumOperands() >= 3) {
            String rm2 = inst.getDefaultOperandRepresentation(0);
            String rn0 = inst.getDefaultOperandRepresentation(1);
            String rd2 = inst.getDefaultOperandRepresentation(2);
            return new TrapFinding(
                inst.getAddress(),
                "TRAP8A_SUB_3OP_SWAP",
                hex,
                inst.toString(),
                rd2 + " = " + rn0 + " - " + rm2,
                "am33.sinc:184 emits (" + rm2 + " - " + rn0 + ") inverted in Ghidra C",
                fnName);
        }

        if ((b0 == 0xFB || b0 == 0xFD || b0 == 0xFE) && b1 == 0x98 && inst.getNumOperands() >= 2) {
            String simm = inst.getDefaultOperandRepresentation(0);
            String rn0 = inst.getDefaultOperandRepresentation(1);
            return new TrapFinding(
                inst.getAddress(),
                "TRAP8B_SUB_IMM_USES_ADD",
                hex,
                inst.toString(),
                rn0 + " = " + rn0 + " - sext(" + simm + ")",
                "am33.sinc:186-188 calls update_add instead of update_sub",
                fnName);
        }
        return null;
    }

    public static void dumpDisassemblyRange(
            Program program,
            Address start,
            Address end,
            GhidraScript script) throws CancelledException {
        if (start.compareTo(end) > 0) {
            Address tmp = start;
            start = end;
            end = tmp;
        }

        Listing listing = program.getListing();
        Memory mem = program.getMemory();
        Address cursor = start;

        Instruction containing = listing.getInstructionContaining(start);
        if (containing != null && containing.getAddress().compareTo(start) < 0) {
            long delta = start.subtract(containing.getAddress());
            script.println(String.format(
                "[ALIGNMENT NOTE] Requested start 0x%08x falls +%d byte(s) inside instruction @ 0x%08x; backing up cleanly.",
                start.getOffset(), delta, containing.getAddress().getOffset()));
            cursor = containing.getAddress();
        }

        FunctionManager fm = program.getFunctionManager();
        while (cursor.compareTo(end) <= 0 && !script.getMonitor().isCancelled()) {
            Instruction inst = listing.getInstructionAt(cursor);
            if (inst != null) {
                TrapFinding trap = classifyTrapInstruction(inst, fm);
                String trapSuffix = trap != null
                    ? "  <-- [" + trap.trapKind() + "] real: " + trap.hardwareSemantics()
                    : "";
                script.println(String.format(
                    "  0x%08x: [%-18s] %s%s",
                    inst.getAddress().getOffset(),
                    formatInstructionBytes(inst),
                    inst.toString(),
                    trapSuffix));
                cursor = cursor.add(inst.getLength());
            } else {
                String rawByte;
                try {
                    rawByte = String.format("%02x", mem.getByte(cursor) & 0xFF);
                } catch (MemoryAccessException ex) {
                    rawByte = "??";
                }
                script.println(String.format("  0x%08x: [%-18s] <undef db 0x%s>", cursor.getOffset(), rawByte, rawByte));
                cursor = cursor.add(1);
            }
        }
    }

    public static String formatInstructionBytes(Instruction inst) {
        try {
            return formatBytes(inst.getBytes());
        } catch (MemoryAccessException ex) {
            return "??";
        }
    }

    public static String formatBytes(byte[] bytes) {
        StringBuilder sb = new StringBuilder(bytes.length * 3);
        for (int i = 0; i < bytes.length; i++) {
            if (i > 0) {
                sb.append(' ');
            }
            sb.append(String.format("%02x", bytes[i] & 0xFF));
        }
        return sb.toString();
    }

    public static String formatTrapCompact(TrapFinding t) {
        return String.format(
            "0x%08x [%s] %-16s (%s) | real: %-18s | %s [%s]",
            t.address().getOffset(),
            t.trapKind(),
            t.rawHex(),
            t.disassembly(),
            t.hardwareSemantics(),
            t.sleighBugNote(),
            t.containingFunction());
    }
}
