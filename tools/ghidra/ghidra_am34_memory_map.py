# Ghidra Script: Panasonic DMR-EH55 Memory Map Setup
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
        ("INTERNAL_SRAM", 0x00000000, 0x00040000, True, True, True, False),
        ("CORE_MMIO", 0x10000000, 0x00010000, True, True, False, True),
        ("MAIN_SDRAM", 0x40000000, 0x04000000, True, True, True, False),
        ("BOOT_BIN_RAM", 0x44000000, 0x00040000, True, True, True, False),
        ("PROG061_FVU_RAM", 0x50400000, 0x00040000, True, True, True, False),
        ("DEV_SERIAL_DATA", 0x515B2000, 0x00002000, True, True, False, True),
        ("DEV_SERIAL_ISR_CTX", 0x5316A000, 0x00002000, True, True, False, True),
        ("MAIN_APP_RAM", 0x57000000, 0x00E00000, True, True, True, False),
        ("FLASH_NOR_IPL", 0x80000000, 0x007F0000, True, False, True, False),
        ("NVRAM_EEMUL", 0x807F0000, 0x00010000, True, True, False, False),
        ("FLASH_NOR_CS1_IPL", 0x84000000, 0x00200000, True, False, True, False),
        ("FLASH_NOR_CS1_BGO", 0x84200000, 0x00200000, True, True, False, True),
        ("FLASH_NOR_CS1_EH55_BGO", 0x84130000, 0x002D0000, True, True, False, True),
        ("FLASH_NOR_CS1_APP", 0x84400000, 0x003F0000, True, False, True, False),
        ("NVRAM_UNCACHED", 0x847F0000, 0x00010000, True, True, False, False),
        ("DAVIS_IDE_PRI_MMIO", 0x89000500, 0x00000100, True, True, False, True),
        ("DAVIS_IDE_SEC_MMIO", 0x89000D00, 0x00000100, True, True, False, True),
        ("SDCD_HOST_MMIO", 0x89001800, 0x00000100, True, True, False, True),
        ("SDRAM_UNCACHED", 0x90000000, 0x04000000, True, True, True, False),
        ("PROG061_SDRAM_UNCACHED", 0x90400000, 0x00040000, True, True, True, False),
        ("ASIC_MMIO", 0xC0000000, 0x00010000, True, True, False, True),
        ("ATA_IDE_MMIO", 0xD4000000, 0x00001000, True, True, False, True),
        ("SC0_UART0_MMIO", 0xD4002000, 0x00000020, True, True, False, True),
    ]

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
