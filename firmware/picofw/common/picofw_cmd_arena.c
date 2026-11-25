/**
 * @file picofw_cmd_arena.c
 * @brief Single 32 KiB mutually-exclusive command capture arena definition.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/picofw_cmd_arena.h"

picofw_cmd_arena_t g_picofw_cmd_arena;
static uint8_t s_arena_owner = PICOFW_ARENA_FREE;

bool picofw_arena_try_acquire(uint8_t owner_id) {
    if ((owner_id == PICOFW_ARENA_FREE) || (s_arena_owner != PICOFW_ARENA_FREE)) {
        return false;
    }
    s_arena_owner = owner_id;
    return true;
}

void picofw_arena_release(uint8_t owner_id) {
    if (s_arena_owner == owner_id) {
        s_arena_owner = PICOFW_ARENA_FREE;
    }
}

uint8_t picofw_arena_current_owner(void) {
    return s_arena_owner;
}
