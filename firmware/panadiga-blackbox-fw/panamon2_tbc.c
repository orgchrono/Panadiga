/**
 * Open-Source Hardware-Interoperability Prototype (Matsushita AM34 / MN103S)
 * EXPERIMENTAL FEASIBILITY SPIKE -- NOT LEGALLY CLEAN-ROOM.
 * Target: Stage-1.5 SDRAM (0x50400000) via DIGADIAG.FRM / P7501 UART0 (115200 8N1).
 *
 * Silicon safety notes (enforced by static guards + bench discipline):
 * 1. AM34 Stage-1.5 has no hardware MPU: always access MMIO/Flash via am34_safe_* wrappers.
 * 2. Before any future Spansion S29JL064H NOR erase/program command, callers MUST mask
 *    maskable IRQs (CLI) and remap exception/IRQ vectors to SDRAM (never fetch from NOR
 *    while Flash is outside Read Array mode).
 * 3. Physical VEP79138 boards require IC7501 TBUS keepalive frames to prevent WDT shutdown.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/* Verified AM34 Memory Map & Safety Boundaries */
#define AM34_UART0_TX_REG        0x10000000u
#define AM34_UART0_STAT_REG      0x10000004u
#define AM34_AIOF_TBC_BASE       0x88000000u
#define AM34_AIOF_TBC_END        0x88001000u
#define AM34_LETHAL_MMIO_START   0xC000100Cu
#define AM34_LETHAL_MMIO_END     0xC0001010u
#define AM34_DAVIS_SAFE_END      0x89000080u
#define AM34_DAVIS_DANGER_END    0x8A000000u
#define AM34_IPL_CS0_END         0x80040000u
#define AM34_IPL_CS1_BASE        0x84000000u
#define AM34_IPL_CS1_END         0x84040000u

_Static_assert(AM34_LETHAL_MMIO_START == 0xC000100Cu,
               "Silicon invariant: 0xC000100C lethal MMIO address must stay exact");
_Static_assert(AM34_IPL_CS1_END - AM34_IPL_CS1_BASE == 0x00040000u,
               "Silicon invariant: 256 KiB IPL CS1 barrier must stay intact");

typedef enum {
    PANADIGA_TV_NTSC = 0,
    PANADIGA_TV_PAL_M = 1,
    PANADIGA_TV_PAL = 2,
    PANADIGA_TV_SECAM = 3
} panadiga_tv_std_t;

static inline bool am34_addr_is_safe(uint32_t addr, bool is_write) {
    if (addr >= AM34_LETHAL_MMIO_START && addr < AM34_LETHAL_MMIO_END) {
        return false; /* Fatal bus-hang / WDT register */
    }
    if (addr >= AM34_DAVIS_SAFE_END && addr < AM34_DAVIS_DANGER_END) {
        return false; /* Unmapped DAVIS space locks AM34 AHB bus */
    }
    if (is_write) {
        if (addr >= 0x80000000u && addr < AM34_IPL_CS0_END) {
            return false; /* Never mutate IPL via CS0 alias */
        }
        if (addr >= AM34_IPL_CS1_BASE && addr < AM34_IPL_CS1_END) {
            return false; /* Never mutate SA0..SA10 critical bootloader */
        }
    }
    return true;
}

static inline bool am34_safe_read32(uint32_t addr, uint32_t *out_val) {
    if (out_val == NULL || !am34_addr_is_safe(addr, false)) {
        return false;
    }
    volatile const uint32_t *ptr = (volatile const uint32_t *)(uintptr_t)addr;
    *out_val = *ptr;
    return true;
}

static inline bool am34_safe_write32(uint32_t addr, uint32_t val) {
    if (!am34_addr_is_safe(addr, true)) {
        return false;
    }
    volatile uint32_t *ptr = (volatile uint32_t *)(uintptr_t)addr;
    *ptr = val;
    return true;
}

static void uart0_putc(char ch) {
    volatile const uint32_t *stat = (volatile const uint32_t *)(uintptr_t)AM34_UART0_STAT_REG;
    uint32_t spin = 0;
    while (((*stat) & 0x20u) == 0u && spin < 50000u) {
        spin++;
    }
    (void)am34_safe_write32(AM34_UART0_TX_REG, (uint32_t)(uint8_t)ch);
}

static void uart0_puts(const char *s) {
    if (s == NULL) {
        return;
    }
    while (*s != '\0') {
        if (*s == '\n') {
            uart0_putc('\r');
        }
        uart0_putc(*s++);
    }
}

static void probe_tbc_aiof_baseline(panadiga_tv_std_t std_mode) {
    uint32_t aiof_id = 0;
    (void)am34_safe_read32(AM34_AIOF_TBC_BASE, &aiof_id);
    switch (std_mode) {
    case PANADIGA_TV_PAL_M:
        uart0_puts("[TBC] PAL-M experimental stub armed (AIOF safe probe)\n");
        break;
    case PANADIGA_TV_PAL:
        uart0_puts("[TBC] PAL experimental stub armed\n");
        break;
    case PANADIGA_TV_SECAM:
        uart0_puts("[TBC] SECAM experimental stub armed\n");
        break;
    case PANADIGA_TV_NTSC:
    default:
        uart0_puts("[TBC] NTSC experimental stub armed\n");
        break;
    }
}

int main(void) {
    uart0_puts("\nPANAMON2 OPEN INTEROP PROTOTYPE (AM34 SDRAM)\n");
    uart0_puts("SW_GUARDS: 0xC000100C=LOCKED DAVIS>=0x89000080=LOCKED IPL_RO=LOCKED\n");
    probe_tbc_aiof_baseline(PANADIGA_TV_PAL_M);
    uart0_puts("PANAMON2>\n");
    return 0;
}
