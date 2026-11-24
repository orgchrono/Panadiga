# Panadiga Continuous Parser Fuzzing (`cargo-fuzz`)

Standalone libFuzzer harness excluded from the root stable Cargo workspace (`exclude = ["fuzz"]` in [`Cargo.toml`](../Cargo.toml)) so regular builds stay on stable Rust while parser targets can be fuzzed continuously on Linux, macOS, or WSL2.

## Prerequisites
- Rust nightly toolchain (`rustup toolchain install nightly`)
- `cargo install cargo-fuzz`

## Targets
- `fuzz_meihdfs_decode`: MEIHFS descriptor records (`decode_single_meihdfs_record`, `try_decode_single_meihdfs_record`), slice decoder, and journal blocks.
- `fuzz_extrhead_unpack`: `EXTRHEAD` header parser, bounded Okumura LZSS decompression, and Feistel auto-key unpacking.
- `fuzz_uf2_validate`: UF2 block framing validation and RP2040/RP2350 slice extraction.
- `fuzz_rsp_parse`: GDB RSP stream framing, `$m` hex decoding, `$g` MN103S register extraction, and `NorWritePolicy` packet filter.
- `fuzz_bgo_audit`: EH55 BGO FTL L2P parsing, P1 signature scan, migration bitmap evaluation, and full-image audit.

## Commands
```bash
cargo +nightly fuzz list
cargo +nightly fuzz run fuzz_extrhead_unpack -- -max_total_time=60
cargo +nightly fuzz run fuzz_meihdfs_decode -- -max_total_time=60
```
