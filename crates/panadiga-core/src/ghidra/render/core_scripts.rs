//! Renders core Ghidra AM34 memory map and symbol importer scripts.

use super::{render_memory_regions, render_symbol_specs};
use crate::ghidra::specs::{memory_region_specs, symbol_specs};

/// Generates the Ghidra Memory Map setup Python script (Jython 2.7 + Python 3 compatible).
pub fn generate_ghidra_memory_map() -> String {
    let regions_block = render_memory_regions(&memory_region_specs());
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Memory Map Setup
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)
#
# Configures the complete memory map of Panasonic AM34 / MN103S (UniPhier)
# matching the DMR-EH55 VEP79138K digital board layout (Jython 2.7 + Py3 safe).

from __future__ import print_function

def _range_overlaps_existing(mem, start, size):
    end = start + size - 1
    for blk in mem.getBlocks():
        blk_start = blk.getStart().getOffset() & 0xFFFFFFFF
        blk_end = blk.getEnd().getOffset() & 0xFFFFFFFF
        if start <= blk_end and blk_start <= end:
            return blk.getName()
    return None

def setup_memory_map(prog=None):
    if prog is None:
        prog = currentProgram
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()

    regions = [
        # (Name, StartHex, SizeBytes, Read, Write, Execute, IsVolatile)
{regions_block}    ]

    print("[Panadiga] Configuring Panasonic DMR-EH55 Memory Map...")
    created = 0
    for name, start, size, r, w, x, vol in regions:
        addr = default_space.getAddress(start)
        overlap = _range_overlaps_existing(mem, start, size)
        if overlap is None:
            try:
                block = mem.createUninitializedBlock(name, addr, size, False)
                block.setRead(r)
                block.setWrite(w)
                block.setExecute(x)
                block.setVolatile(vol)
                block.setComment("Panasonic DMR-EH55 HW Subsystem: " + name)
                created += 1
                print("  [+] Created block: %s at 0x%08X (size: 0x%X)" % (name, start, size))
            except Exception as e:
                print("  [-] Could not create block %s: %s" % (name, str(e)))
        else:
            print("  [*] Overlap skipped for %s at 0x%08X (covered by %s)" % (name, start, overlap))

    print("[Panadiga] Memory Map setup complete! (%d new blocks)" % created)
    return created

if __name__ == "__main__":
    setup_memory_map()
"#
    )
}

/// Generates the Ghidra Symbols and Peripheral importer script (Jython 2.7 + Python 3 safe).
pub fn generate_ghidra_symbols() -> String {
    let symbols_block = render_symbol_specs(&symbol_specs());
    format!(
        r#"# Ghidra Script: Panasonic DMR-EH55 Symbol & Peripheral Importer
# @category Forensics.Panasonic
# @author Panadiga Engineering Team (Generated from native Rust SSOT)

from __future__ import print_function

try:
    from ghidra.program.model.listing import CodeUnit
    from ghidra.program.model.symbol import SourceType
except ImportError:
    CodeUnit = None
    SourceType = None

def get_panadiga_symbol_specs():
    return [
{symbols_block}    ]

def import_symbols(prog=None):
    if prog is None:
        prog = currentProgram
    sym_table = prog.getSymbolTable()
    listing = prog.getListing()
    mem = prog.getMemory()
    addr_factory = prog.getAddressFactory()
    default_space = addr_factory.getDefaultAddressSpace()
    plate_comment = CodeUnit.PLATE_COMMENT if CodeUnit is not None else 3

    symbols = get_panadiga_symbol_specs()
    print("[Panadiga] Annotating Panasonic DMR-EH55 Symbols and Peripherals...")
    imported_count = 0
    skipped_unmapped = 0

    for addr_val, name, comment, is_primary in symbols:
        addr = default_space.getAddress(addr_val)
        if mem.getBlock(addr) is None:
            skipped_unmapped += 1
            continue
        try:
            sym = sym_table.createLabel(addr, name, SourceType.USER_DEFINED)
            if is_primary and sym is not None:
                sym.setPrimary()
            listing.setComment(addr, plate_comment, comment)
            imported_count += 1
            print("  [+] Labeled 0x%08X -> %s" % (addr_val, name))
        except Exception as e:
            listing.setComment(addr, plate_comment, comment)
            print("  [*] Updated comment at 0x%08X -> %s (%s)" % (addr_val, name, str(e)))

    print("[Panadiga] Symbol import complete! Annotated %d symbols (%d unmapped skipped)." % (imported_count, skipped_unmapped))
    return imported_count

if __name__ == "__main__":
    import_symbols()
"#
    )
}
