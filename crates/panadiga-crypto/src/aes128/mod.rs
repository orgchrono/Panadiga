#![deny(unsafe_code)]

//! Pure-Rust FIPS-197 AES-128-CBC (nopad), Panasonic Blu-ray Keyring (`2016`/`2018` + `keys.ukf`),
//! and `libdownloadlib.so` Network Key Derivation.

pub mod cipher;
pub mod keys;
#[cfg(test)]
mod tests;

pub use cipher::{
    aes128_cbc_decrypt_in_place, aes128_cbc_encrypt_in_place, aes128_decrypt_block,
    aes128_encrypt_block,
};
pub use keys::{
    AesCatalogEntry, FrmKeyring, PANA_DVD_AES_CATALOG, PanaAesKeyPair, PanaFeistelKey,
    derive_network_key_iv,
};
