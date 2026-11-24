use super::{U32RelocSpec, WordPatchSite};
use crate::constants::AM34_MAIN_BASE_VA;

pub(super) const fn va_off(va: u32) -> usize {
    (va - AM34_MAIN_BASE_VA) as usize
}

pub(super) const NFD_4MB_TO_8MB_RODATA_WORDS: &[WordPatchSite] = &[
    // BGO DID group logical/physical sector counts (`28/35 -> 40/45`, `P1=3 -> 1`).
    WordPatchSite {
        offset: va_off(0x579E_D2CC),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D0),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D4),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2D8),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E0),
        expect: 0x0000_0005,
        replace: 0x0000_0003,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E4),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2E8),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    WordPatchSite {
        offset: va_off(0x579E_D2EC),
        expect: 0x0000_0023,
        replace: 0x0000_002D,
    },
    // Primary BGO descriptor (`0x579E_D32C..=0x579E_D36C`).
    WordPatchSite {
        offset: va_off(0x579E_D32C),
        expect: 0x0000_0003,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D330),
        expect: 0x0000_001C,
        replace: 0x0000_0028,
    },
    WordPatchSite {
        offset: va_off(0x579E_D340),
        expect: 0x0000_0A1A,
        replace: 0x0000_0F24,
    },
    WordPatchSite {
        offset: va_off(0x579E_D344),
        expect: 0x0000_0002,
        replace: 0x0000_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D350),
        expect: 0x0000_0020,
        replace: 0x0000_002E,
    },
    WordPatchSite {
        offset: va_off(0x579E_D354),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D358),
        expect: 0x843F_0000,
        replace: 0x8440_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D35C),
        expect: 0x0000_2000,
        replace: 0x0001_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D360),
        expect: 0x0000_0020,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D364),
        expect: 0x0000_0028,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D368),
        expect: 0x843F_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D36C),
        expect: 0x8440_0000,
        replace: 0xFFFF_FFFF,
    },
    // BGO RAM pointer rows (`0x579E_D398..=0x579E_D400`) relocated into `0x531B_B000..0x531B_BC00`.
    WordPatchSite {
        offset: va_off(0x579E_D398),
        expect: 0x52FE_9230,
        replace: 0x531B_B000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D39C),
        expect: 0x52FE_93C0,
        replace: 0x531B_B620,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A0),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A4),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3A8),
        expect: 0x52FE_9466,
        replace: 0x531B_B830,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3AC),
        expect: 0x52FE_937C,
        replace: 0x531B_B510,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3B4),
        expect: 0x52FE_9374,
        replace: 0x531B_B500,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3B8),
        expect: 0x52FE_9462,
        replace: 0x531B_B820,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3BC),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C0),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C4),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3C8),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D0),
        expect: 0x52FE_9376,
        replace: 0x531B_B502,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D4),
        expect: 0x52FE_9463,
        replace: 0x531B_B821,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3D8),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3DC),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3E0),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3E4),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3EC),
        expect: 0x52FE_9378,
        replace: 0x531B_B504,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F0),
        expect: 0x52FE_9464,
        replace: 0x531B_B822,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F4),
        expect: 0x52FE_93BE,
        replace: 0x531B_B612,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3F8),
        expect: 0x52FE_9469,
        replace: 0x531B_B833,
    },
    WordPatchSite {
        offset: va_off(0x579E_D3FC),
        expect: 0x52FE_9468,
        replace: 0x531B_B832,
    },
    WordPatchSite {
        offset: va_off(0x579E_D400),
        expect: 0x52FE_93BC,
        replace: 0x531B_B610,
    },
    // Flash vendor bank base tables (`0x579E_D408..=0x579E_D434`).
    WordPatchSite {
        offset: va_off(0x579E_D408),
        expect: 0x0000_0003,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D40C),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D410),
        expect: 0x8430_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D414),
        expect: 0x8438_0000,
        replace: 0xFFFF_FFFF,
    },
    WordPatchSite {
        offset: va_off(0x579E_D42C),
        expect: 0x0000_0002,
        replace: 0x0000_0001,
    },
    WordPatchSite {
        offset: va_off(0x579E_D430),
        expect: 0x8420_0000,
        replace: 0x8413_0000,
    },
    WordPatchSite {
        offset: va_off(0x579E_D434),
        expect: 0x8438_0000,
        replace: 0xFFFF_FFFF,
    },
];

pub(super) const NFD_4MB_TO_8MB_RAM_RELOCS: &[U32RelocSpec] = &[
    U32RelocSpec {
        from: 0x52FE_6CE0,
        to: 0x531B_8000,
        expected_count: 11,
    },
    U32RelocSpec {
        from: 0x52FE_6CE8,
        to: 0x531B_8008,
        expected_count: 7,
    },
    U32RelocSpec {
        from: 0x52FE_6CEA,
        to: 0x531B_800A,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_6EE0,
        to: 0x531B_8200,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_74E0,
        to: 0x531B_8800,
        expected_count: 16,
    },
    U32RelocSpec {
        from: 0x52FE_88E0,
        to: 0x531B_A600,
        expected_count: 31,
    },
    U32RelocSpec {
        from: 0x52FE_89A0,
        to: 0x531B_A6C0,
        expected_count: 3,
    },
    U32RelocSpec {
        from: 0x52FE_89A2,
        to: 0x531B_A6C2,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x52FE_89B0,
        to: 0x531B_A6D0,
        expected_count: 4,
    },
    U32RelocSpec {
        from: 0x52FE_8BB0,
        to: 0x531B_A8D0,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_8BC0,
        to: 0x531B_A8E0,
        expected_count: 17,
    },
    U32RelocSpec {
        from: 0x52FE_8DC0,
        to: 0x531B_AAE0,
        expected_count: 1,
    },
    // BGO DID table (`40 * 12 = 480 B` in 8 MiB vs 332 B in 4 MiB; prevents flash fntable smash).
    U32RelocSpec {
        from: 0x52FE_8FB4,
        to: 0x531B_B8F4,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x52FE_8FC0,
        to: 0x531B_B900,
        expected_count: 8,
    },
    U32RelocSpec {
        from: 0x52FE_8FC4,
        to: 0x531B_B904,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x52FE_8FC8,
        to: 0x531B_B908,
        expected_count: 1,
    },
    // BGO sector map (`45 * 2 = 90 B` in 8 MiB vs 78 B in 4 MiB; prevents partition-state smash).
    U32RelocSpec {
        from: 0x52FE_9176,
        to: 0x531B_BB00,
        expected_count: 7,
    },
    U32RelocSpec {
        from: 0x52FE_9178,
        to: 0x531B_BB02,
        expected_count: 6,
    },
    U32RelocSpec {
        from: 0x52FE_917A,
        to: 0x531B_BB04,
        expected_count: 1,
    },
    // Spansion NOR JEDEC command addresses aligned with native 8 MiB Loader ver0.61 NOR IPL.
    U32RelocSpec {
        from: 0x8420_0000,
        to: 0x8413_0000,
        expected_count: 2,
    },
    U32RelocSpec {
        from: 0x8420_0002,
        to: 0x8413_0002,
        expected_count: 3,
    },
    U32RelocSpec {
        from: 0x8420_001C,
        to: 0x8413_001C,
        expected_count: 1,
    },
    U32RelocSpec {
        from: 0x8420_0554,
        to: 0x8413_0554,
        expected_count: 10,
    },
    U32RelocSpec {
        from: 0x8420_0AAA,
        to: 0x8413_0AAA,
        expected_count: 11,
    },
];
