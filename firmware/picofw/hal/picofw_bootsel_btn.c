/**
 * @file picofw_bootsel_btn.c
 * @brief Runtime RP2040/RP2350 RAM-resident QSPI CS BOOTSEL button sampler and stand-alone action dispatcher.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "hal/picofw_bootsel_btn.h"
#include "diag/picofw_identify.h"
#include "flash/picofw_job.h"
#include "flash/picofw_qspi_storage.h"
#include "hal/picofw_bootsel_logic.h"
#include "hal/picofw_hw.h"
#include "hal/picofw_multicore.h"

#if !defined(PICO_HOST_TEST)
#include "hardware/gpio.h"
#include "hardware/structs/ioqspi.h"
#include "hardware/structs/sio.h"
#include "hardware/sync.h"
#include "pico/stdlib.h"
#include "pico/time.h"
#endif

#include <stdio.h>

#if !defined(PICO_HOST_TEST)
static picofw_bootsel_fsm_t s_bootsel_fsm;
static bool s_bootsel_inited = false;
static uint32_t s_last_poll_ms = 0u;

bool __no_inline_not_in_flash_func(picofw_bootsel_read_raw)(void) {
    const uint cs_pin_index = 1u;
    const uint32_t flags = save_and_disable_interrupts();

    hw_write_masked(
        &ioqspi_hw->io[cs_pin_index].ctrl,
        GPIO_OVERRIDE_LOW << IO_QSPI_GPIO_QSPI_SS_CTRL_OEOVER_LSB,
        IO_QSPI_GPIO_QSPI_SS_CTRL_OEOVER_BITS
    );

    for (volatile int i = 0; i < 1000; ++i) {
    }

#if defined(PICO_RP2350) && PICO_RP2350
    const bool button_pressed = ((sio_hw->gpio_hi_in & SIO_GPIO_HI_IN_QSPI_CSN_BITS) == 0u);
#else
    const bool button_pressed = ((sio_hw->gpio_hi_in & (1u << cs_pin_index)) == 0u);
#endif

    hw_write_masked(
        &ioqspi_hw->io[cs_pin_index].ctrl,
        GPIO_OVERRIDE_NORMAL << IO_QSPI_GPIO_QSPI_SS_CTRL_OEOVER_LSB,
        IO_QSPI_GPIO_QSPI_SS_CTRL_OEOVER_BITS
    );

    restore_interrupts(flags);
    return button_pressed;
}
#else
bool picofw_bootsel_read_raw(void) {
    return false;
}
#endif

void picofw_bootsel_poll(void) {
#if !defined(PICO_HOST_TEST)
    if (!s_bootsel_inited) {
        picofw_bootsel_fsm_init(&s_bootsel_fsm);
        s_bootsel_inited = true;
    }
    const uint32_t now_ms = to_ms_since_boot(get_absolute_time());
    if ((now_ms - s_last_poll_ms) < 50u) {
        return;
    }
    s_last_poll_ms = now_ms;

    /* Core-1 steady-state loop runs 100% in SRAM (`core1_worker_loop_ram`), so 50 ms
     * BOOTSEL sampling does not need an inter-core FIFO XIP lockout stall. */
    const bool pressed = picofw_bootsel_read_raw();
    const picofw_bootsel_event_t ev = picofw_bootsel_fsm_step(&s_bootsel_fsm, pressed, now_ms);
    if (ev == PICOFW_BOOTSEL_EVT_SHORT_IDENTIFY) {
        printf("\n@EVT kind=BOOTSEL action=SHORT_IDENTIFY\n");
        picofw_ident_report_t rep;
        picofw_identify_run_hw(NULL, &rep);
        (void)picofw_blackbox_append_hw(
            rep.vdd_mv,
            picofw_hw_get_nsw5v_dropouts(),
            rep.caps_mask,
            rep.model_name,
            "BOOTSEL_SHORT_IDENTIFY"
        );
        printf("PICO> ");
        fflush(stdout);
    } else if (ev == PICOFW_BOOTSEL_EVT_LONG_VAULT_BACKUP) {
        printf("\n@EVT kind=BOOTSEL action=LONG_VAULT_BACKUP\n");
        (void)picofw_job_run_vault_backup_hw(1u);
        printf("PICO> ");
        fflush(stdout);
    }
#endif
}
