//! Panasonic Okumura LZSS (`algo == 2`, `WindowInitialIndex = 0x0FEE`) compressor and decompressor.

use crate::constants::{
    PANA_LZSS_HASH_BITS, PANA_LZSS_HASH_SIZE, PANA_LZSS_KNUTH_HASH_MULT, PANA_LZSS_MAX_CODED,
    PANA_LZSS_MAX_UNCODED, PANA_LZSS_WINDOW_INIT, PANA_LZSS_WINDOW_SIZE,
};

/// Decompresses an Okumura LZSS stream used by Panasonic DIGA firmware (`algo == 2`).
///
/// Uses a 4096-byte sliding window, 0-initialized, with initial window position `0x0FEE`.
/// Flag byte: bit 0 indicates literal (1) or 2-byte backreference (0).
pub fn decompress_pana_lzss(data: &[u8], expected_size: usize) -> Vec<u8> {
    let mut window = [0u8; 0x1000];
    let mut dst = if expected_size > 0 {
        Vec::with_capacity(expected_size)
    } else {
        Vec::new()
    };
    let mut src_i = 0;
    let mut win_pos = 0x0FEE;
    let mut flags = 0u16;

    while src_i < data.len() {
        flags >>= 1;
        if (flags & 0x100) == 0 {
            if src_i >= data.len() {
                break;
            }
            flags = (data[src_i] as u16) | 0xFF00;
            src_i += 1;
        }

        if (flags & 1) == 0 {
            // 2-byte Backreference: offset (12 bits) and length (4 bits + 3)
            if src_i + 1 >= data.len() {
                break;
            }
            let b1 = data[src_i];
            let b2 = data[src_i + 1];
            src_i += 2;

            let mut offset = (b1 as usize) | (((b2 & 0xF0) as usize) << 4);
            let length = ((b2 & 0x0F) as usize) + 3;

            for _ in 0..length {
                let c = window[offset];
                dst.push(c);
                window[win_pos] = c;
                win_pos = (win_pos + 1) & 0x0FFF;
                offset = (offset + 1) & 0x0FFF;
                if expected_size > 0 && dst.len() >= expected_size {
                    return dst;
                }
            }
        } else {
            // 1-byte Literal
            if src_i >= data.len() {
                break;
            }
            let c = data[src_i];
            src_i += 1;
            dst.push(c);
            window[win_pos] = c;
            win_pos = (win_pos + 1) & 0x0FFF;
            if expected_size > 0 && dst.len() >= expected_size {
                return dst;
            }
        }
    }

    dst
}

/// Compresses data with Panasonic's Okumura LZSS variant (`WindowInitialIndex = 0x0FEE`, `algo == 2`).
///
/// Mirrors `pana-fw/utils/lzss/compress.go` using 3-byte hash-chain matching and lazy match selection.
pub fn compress_pana_lzss(data: &[u8]) -> Vec<u8> {
    const WINDOW_SIZE: usize = PANA_LZSS_WINDOW_SIZE;
    const WINDOW_INIT: usize = PANA_LZSS_WINDOW_INIT;
    const MAX_UNCODED: usize = PANA_LZSS_MAX_UNCODED;
    const MAX_CODED: usize = PANA_LZSS_MAX_CODED;
    const HASH_BITS: usize = PANA_LZSS_HASH_BITS;
    const HASH_SIZE: usize = PANA_LZSS_HASH_SIZE;

    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    let hash3 = |pos: usize| -> usize {
        let v = (u32::from(data[pos]) << 16)
            | (u32::from(data[pos + 1]) << 8)
            | u32::from(data[pos + 2]);
        ((v.wrapping_mul(PANA_LZSS_KNUTH_HASH_MULT)) >> (32 - HASH_BITS)) as usize
    };

    let mut head = [-1i32; HASH_SIZE];
    let mut prev = [-1i32; WINDOW_SIZE];

    let insert_hash = |pos: usize, head: &mut [i32; HASH_SIZE], prev: &mut [i32; WINDOW_SIZE]| {
        if pos + MAX_UNCODED <= n {
            let h = hash3(pos);
            prev[pos & (WINDOW_SIZE - 1)] = head[h];
            head[h] = pos as i32;
        }
    };

    let find_match = |i: usize,
                      min_len: usize,
                      head: &[i32; HASH_SIZE],
                      prev: &[i32; WINDOW_SIZE]|
     -> (usize, usize) {
        let mut best_len = min_len;
        let mut best_pos = 0usize;
        if i + MAX_UNCODED > n {
            return (best_pos, best_len);
        }
        let max_len = (MAX_CODED - 1).min(n - i);
        if max_len <= best_len {
            return (best_pos, best_len);
        }
        let h = hash3(i);
        let min_pos = (i + 1).saturating_sub(WINDOW_SIZE - MAX_CODED);
        let mut cur = head[h];
        let mut chain = 64usize;
        while cur >= min_pos as i32 && chain > 0 {
            let j = cur as usize;
            if best_len >= max_len {
                break;
            }
            if !(best_len >= MAX_UNCODED && data[j + best_len] != data[i + best_len])
                && data[j] == data[i]
                && data[j + 1] == data[i + 1]
                && data[j + 2] == data[i + 2]
            {
                let mut k = 3usize;
                while k < max_len && data[j + k] == data[i + k] {
                    k += 1;
                }
                if k > best_len {
                    best_len = k;
                    best_pos = j;
                    if k >= max_len {
                        break;
                    }
                }
            }
            cur = prev[j & (WINDOW_SIZE - 1)];
            chain -= 1;
        }
        (best_pos, best_len)
    };

    let mut output = Vec::with_capacity(n / 2 + 16);
    let mut flags_offset = 0usize;
    let mut flags_bit = 0u8;
    let mut i = 0usize;

    while i < n {
        if flags_bit == 0 {
            flags_offset = output.len();
            output.push(0);
        }
        let (match_pos, match_len) = find_match(i, MAX_UNCODED - 1, &head, &prev);
        let lazy_skip = if (MAX_UNCODED..MAX_CODED - 1).contains(&match_len) && i + 1 < n {
            let (_, next_len) = find_match(i + 1, match_len, &head, &prev);
            next_len > match_len
        } else {
            false
        };

        if lazy_skip || match_len < MAX_UNCODED {
            output[flags_offset] |= 1u8 << flags_bit;
            output.push(data[i]);
            insert_hash(i, &mut head, &mut prev);
            i += 1;
        } else {
            let wp = (WINDOW_INIT + match_pos) & (WINDOW_SIZE - 1);
            output.push(wp as u8);
            output.push((((wp >> 4) & 0xF0) as u8) | (((match_len - MAX_UNCODED) & 0x0F) as u8));
            for k in 0..match_len {
                insert_hash(i + k, &mut head, &mut prev);
            }
            i += match_len;
        }
        flags_bit = (flags_bit + 1) & 7;
    }

    output
}
