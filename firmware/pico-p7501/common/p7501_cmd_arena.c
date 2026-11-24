/**
 * @file p7501_cmd_arena.c
 * @brief Single 32 KiB mutually-exclusive command capture arena definition.
 *
 * @standard ISO/IEC 9899:2011 (C11)
 */

#include "common/p7501_cmd_arena.h"

p7501_cmd_arena_t g_p7501_cmd_arena;
static uint8_t s_arena_owner = P7501_ARENA_FREE;

bool p7501_arena_try_acquire(uint8_t owner_id) {
    if ((owner_id == P7501_ARENA_FREE) || (s_arena_owner != P7501_ARENA_FREE)) {
        return false;
    }
    s_arena_owner = owner_id;
    return true;
}

void p7501_arena_release(uint8_t owner_id) {
    if (s_arena_owner == owner_id) {
        s_arena_owner = P7501_ARENA_FREE;
    }
}

uint8_t p7501_arena_current_owner(void) {
    return s_arena_owner;
}
