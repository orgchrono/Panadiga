use super::cipher::*;
use super::keys::*;
#[test]
fn aes128_fips197_and_sp800_38a_vectors() {
    // FIPS-197 Appendix B
    let key = hex::decode("2b7e151628aed2a6abf7158809cf4f3c").unwrap();
    let pt = hex::decode("3243f6a8885a308d313198a2e0370734").unwrap();
    let ct = hex::decode("3925841d02dc09fbdc118597196a0b32").unwrap();
    let k: [u8; 16] = key.try_into().unwrap();
    let p: [u8; 16] = pt.try_into().unwrap();
    let c: [u8; 16] = ct.try_into().unwrap();
    assert_eq!(aes128_encrypt_block(&k, &p), c);
    assert_eq!(aes128_decrypt_block(&k, &c), p);

    // NIST SP 800-38A F.2.1 CBC-AES128.Encrypt (2 blocks)
    let iv: [u8; 16] = hex::decode("000102030405060708090a0b0c0d0e0f")
        .unwrap()
        .try_into()
        .unwrap();
    let cbc_pt =
        hex::decode("6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51").unwrap();
    let cbc_ct =
        hex::decode("7649abac8119b246cee98e9b12e9197d5086cb9b507219ee95db113a917678b2").unwrap();
    let mut buf = cbc_pt.clone();
    aes128_cbc_encrypt_in_place(&k, &iv, &mut buf).unwrap();
    assert_eq!(buf, cbc_ct);
    aes128_cbc_decrypt_in_place(&k, &iv, &mut buf).unwrap();
    assert_eq!(buf, cbc_pt);
}

#[test]
fn network_key_derivation_and_ukf_merge() {
    let (nkey, niv) = derive_network_key_iv();
    assert_eq!(hex::encode(nkey), "d73644e288975ed5e24117a57468d9c3");
    assert_eq!(hex::encode(niv), "a8f24304441eb907cfa1647f27f0aed1");

    let ukf = r#"
# sample keys.ukf
collection "PANA_DVD_KEYONLY": {
    "Old": {x"DF7215C7427FD010"}, # duplicate of built-in
    "custom_test": {x"1122334455667788"},
}
collection "PANA_DVD_AESPAIR": {
    "custom_2025": {x"00112233445566778899AABBCCDDEEFF", x"FFEEDDCCBBAA99887766554433221100", x"AABBCCDDEEFF0011"},
}
"#;
    let mut ring = FrmKeyring::builtin();
    let base_feistel = ring.feistel_keys.len();
    let base_aes = ring.aes_pairs.len();
    let added = ring.merge_ukf_str(ukf).unwrap();
    assert_eq!(added, 2);
    assert_eq!(ring.feistel_keys.len(), base_feistel + 2);
    assert_eq!(ring.aes_pairs.len(), base_aes + 1);
}

#[test]
fn cascading_keyring_unions_distinct_keys_and_overrides_on_name_collision() {
    let dir = std::env::temp_dir().join(format!("panadiga_ukf_cascade_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let env_p = dir.join("env.ukf");
    let local_p = dir.join("local.ukf");
    let cli_p = dir.join("cli.ukf");
    let missing_local = dir.join("absent_local.ukf");

    std::fs::write(
        &env_p,
        r#"collection "PANA_DVD_KEYONLY": {
                "env_k": {x"0102030405060708"},
                "shared_id": {x"AAAAAAAAAAAAAAAA"},
            }"#,
    )
    .unwrap();
    std::fs::write(
        &local_p,
        r#"collection "PANA_DVD_KEYONLY": {
                "local_k": {x"1112131415161718"},
            }"#,
    )
    .unwrap();
    std::fs::write(
        &cli_p,
        r#"collection "PANA_DVD_KEYONLY": {
                "shared_id": {x"BBBBBBBBBBBBBBBB"},
            }"#,
    )
    .unwrap();

    let ring = FrmKeyring::load_cascading(Some(&env_p), Some(&local_p), Some(&cli_p)).unwrap();
    assert!(ring.feistel_keys.iter().any(|k| k.name == "env_k"));
    assert!(ring.feistel_keys.iter().any(|k| k.name == "local_k"));
    let shared = ring
        .feistel_keys
        .iter()
        .find(|k| k.name == "shared_id")
        .unwrap();
    assert_eq!(shared.key_8b, [0xBB; 8]);

    let ring2 = FrmKeyring::load_cascading(Some(&env_p), Some(&missing_local), None).unwrap();
    assert!(ring2.feistel_keys.iter().any(|k| k.name == "env_k"));

    let _ = std::fs::remove_dir_all(&dir);
}
