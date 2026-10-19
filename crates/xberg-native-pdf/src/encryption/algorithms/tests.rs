use super::*;

#[test]
fn test_pad_password() {
    let password = b"test";
    let padded = pad_password(password);
    assert_eq!(padded.len(), 32);
    assert_eq!(&padded[..4], b"test");
    assert_eq!(&padded[4..], &PADDING[..28]);
}

#[test]
fn test_pad_password_long() {
    let password = b"this is a very long password that exceeds 32 bytes";
    let padded = pad_password(password);
    assert_eq!(padded.len(), 32);
    assert_eq!(&padded[..], &password[..32]);
}

#[test]
fn test_pad_password_exact() {
    let password = &[0u8; 32];
    let padded = pad_password(password);
    assert_eq!(padded.len(), 32);
    assert_eq!(&padded[..], password);
}

#[test]
fn test_constant_time_compare_equal() {
    let a = b"test1234test1234";
    let b = b"test1234test1234";
    assert!(constant_time_compare(a, b));
}

#[test]
fn test_constant_time_compare_not_equal() {
    let a = b"test1234test1234";
    let b = b"test1234test1235";
    assert!(!constant_time_compare(a, b));
}

#[test]
fn test_constant_time_compare_different_length() {
    let a = b"test";
    let b = b"testing";
    assert!(!constant_time_compare(a, b));
}

#[test]
fn test_compute_encryption_key() {
    let password = b"user";
    let owner_key = &[0u8; 32];
    let permissions = -1;
    let file_id = b"test_file_id";
    let revision = 2;
    let key_length = 5;

    let key = compute_encryption_key(password, owner_key, permissions, file_id, revision, key_length, true).unwrap();

    assert_eq!(key.len(), key_length);
}

#[test]
fn test_owner_password_hash_r2() {
    let owner = b"owner";
    let user = b"user";
    let revision = 2;
    let key_length = 5;

    let owner_hash = compute_owner_password_hash(owner, user, revision, key_length).unwrap();

    assert_eq!(owner_hash.len(), 32);

    // Verify the hash can decrypt back to the user password
    // For R=2, decrypt with same RC4 key should give padded user password ~keep
}

#[test]
fn test_owner_password_hash_r3() {
    let owner = b"owner";
    let user = b"user";
    let revision = 3;
    let key_length = 16;

    let owner_hash = compute_owner_password_hash(owner, user, revision, key_length).unwrap();

    assert_eq!(owner_hash.len(), 32);
}

#[test]
fn owner_password_hash_rejects_revision_5_and_above() {
    // ISO 32000-2 Algorithm 8 needs U[0..48] and the file encryption
    // key, neither of which this function's signature carries;
    // computing O without them would silently produce a
    // spec-noncompliant value. Callers must use
    // compute_u_and_ue/compute_o_and_oe instead. ~keep
    let err = compute_owner_password_hash(b"owner", b"user", 5, 32).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("revision >= 5"), "got: {msg}");

    let err = compute_owner_password_hash(b"owner", b"user", 6, 32).unwrap_err();
    assert!(err.to_string().contains("revision >= 5"));
}

#[test]
fn test_owner_password_hash_empty_owner() {
    let user = b"user";
    let revision = 3;
    let key_length = 16;

    let hash1 = compute_owner_password_hash(b"", user, revision, key_length).unwrap();
    let hash2 = compute_owner_password_hash(user, user, revision, key_length).unwrap();

    assert_eq!(hash1, hash2);
}

#[test]
fn test_user_password_hash_r2() {
    let key = [0u8; 5];
    let file_id = b"test_file_id";
    let revision = 2;

    let user_hash = compute_user_password_hash(&key, file_id, revision).unwrap();

    assert_eq!(user_hash.len(), 32);
}

#[test]
fn test_user_password_hash_r3() {
    let key = [0u8; 16];
    let file_id = b"test_file_id";
    let revision = 3;

    let user_hash = compute_user_password_hash(&key, file_id, revision).unwrap();

    assert_eq!(user_hash.len(), 32);
}

#[test]
fn test_encryption_roundtrip_r2() {
    let owner_pass = b"owner123";
    let user_pass = b"user123";
    let file_id = b"test_file_id_123";
    let permissions = -1i32;
    let revision = 2;
    let key_length = 5;

    let owner_hash = compute_owner_password_hash(owner_pass, user_pass, revision, key_length).unwrap();

    let encryption_key =
        compute_encryption_key(user_pass, &owner_hash, permissions, file_id, revision, key_length, true).unwrap();

    let user_hash = compute_user_password_hash(&encryption_key, file_id, revision).unwrap();

    let auth_result = authenticate_user_password(
        user_pass,
        &user_hash,
        &owner_hash,
        permissions,
        file_id,
        revision,
        key_length,
        true,
        None,
    );

    assert!(auth_result.is_some());
    assert_eq!(auth_result.unwrap(), encryption_key);
}

#[test]
fn test_encryption_roundtrip_r3() {
    let owner_pass = b"owner456";
    let user_pass = b"user456";
    let file_id = b"test_file_id_456";
    let permissions = -1i32;
    let revision = 3;
    let key_length = 16;

    let owner_hash = compute_owner_password_hash(owner_pass, user_pass, revision, key_length).unwrap();
    let encryption_key =
        compute_encryption_key(user_pass, &owner_hash, permissions, file_id, revision, key_length, true).unwrap();
    let user_hash = compute_user_password_hash(&encryption_key, file_id, revision).unwrap();

    let auth_result = authenticate_user_password(
        user_pass,
        &user_hash,
        &owner_hash,
        permissions,
        file_id,
        revision,
        key_length,
        true,
        None,
    );

    assert!(auth_result.is_some());
    assert_eq!(auth_result.unwrap(), encryption_key);
}

#[test]
fn test_authenticate_user_password_short_user_key() {
    // /U value shorter than 16 bytes should return None, not panic ~keep
    let short_user_key = vec![0u8; 10];
    let owner_key = vec![0u8; 32];
    let result = authenticate_user_password(b"", &short_user_key, &owner_key, -1, b"file_id", 2, 5, true, None);
    assert!(result.is_none());
}

#[test]
fn test_saslprep_ascii_passthrough() {
    let password = b"hello123";
    let result = saslprep_password(password);
    assert_eq!(result, b"hello123");
}

#[test]
fn test_saslprep_unicode_normalization() {
    // NFKC normalization: fullwidth 'A' (U+FF21) should map to 'A' (U+0041) ~keep
    let password = "\u{FF21}".as_bytes();
    let result = saslprep_password(password);
    assert_eq!(result, b"A");
}

#[test]
fn test_authenticate_user_r5_correct_password() {
    let password = b"test";
    let validation_salt = [0x01u8; 8];
    let key_salt = [0x02u8; 8];

    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(validation_salt);
    let hash = hasher.finalize();

    let mut user_key = hash.to_vec();
    user_key.extend_from_slice(&validation_salt);
    user_key.extend_from_slice(&key_salt);
    assert_eq!(user_key.len(), 48);

    // Wrap a known file key into /UE with the spec algorithm:
    // UE = AES-256-CBC(key = SHA-256(password || key_salt), iv = 0, file key). ~keep
    let file_key = [0x5Au8; 32];
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(key_salt);
    let intermediate = hasher.finalize();
    let ue = crate::encryption::aes::aes256_encrypt_no_padding(&intermediate, &[0u8; 16], &file_key).unwrap();

    let result = authenticate_user_password(password, &user_key, &[0u8; 48], -1, b"", 5, 32, true, Some(&ue));
    // The file key is the unwrapped /UE — never the intermediate hash. ~keep
    assert_eq!(result.unwrap(), file_key.to_vec());

    // Without /UE the file key is unobtainable; authentication must fail
    // loudly rather than hand back the intermediate hash as a "key". ~keep
    let no_ue = authenticate_user_password(password, &user_key, &[0u8; 48], -1, b"", 5, 32, true, None);
    assert!(no_ue.is_none());
}

#[test]
fn test_authenticate_user_r5_wrong_password() {
    let password = b"test";
    let validation_salt = [0x01u8; 8];
    let key_salt = [0x02u8; 8];

    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(validation_salt);
    let hash = hasher.finalize();

    let mut user_key = hash.to_vec();
    user_key.extend_from_slice(&validation_salt);
    user_key.extend_from_slice(&key_salt);

    let result = authenticate_user_password(b"wrong", &user_key, &[0u8; 48], -1, b"", 5, 32, true, None);
    assert!(result.is_none());
}

#[test]
fn test_authenticate_user_r5_short_u_value() {
    // U value shorter than 48 bytes should return None ~keep
    let result = authenticate_user_password(b"test", &[0u8; 40], &[0u8; 48], -1, b"", 5, 32, true, None);
    assert!(result.is_none());
}

#[test]
fn test_authenticate_owner_password_r2_roundtrip() {
    let owner_pass = b"owner123";
    let user_pass = b"user123";
    let file_id = b"test_file_id_123";
    let permissions = -1i32;
    let revision = 2;
    let key_length = 5;

    let owner_hash = compute_owner_password_hash(owner_pass, user_pass, revision, key_length).unwrap();
    let encryption_key =
        compute_encryption_key(user_pass, &owner_hash, permissions, file_id, revision, key_length, true).unwrap();
    let user_hash = compute_user_password_hash(&encryption_key, file_id, revision).unwrap();

    let result = authenticate_owner_password(
        owner_pass,
        &user_hash,
        &owner_hash,
        permissions,
        file_id,
        revision,
        key_length,
        true,
        None,
    )
    .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), encryption_key);
}

#[test]
fn test_authenticate_owner_password_r3_roundtrip() {
    let owner_pass = b"owner456";
    let user_pass = b"user456";
    let file_id = b"test_file_id_456";
    let permissions = -1i32;
    let revision = 3;
    let key_length = 16;

    let owner_hash = compute_owner_password_hash(owner_pass, user_pass, revision, key_length).unwrap();
    let encryption_key =
        compute_encryption_key(user_pass, &owner_hash, permissions, file_id, revision, key_length, true).unwrap();
    let user_hash = compute_user_password_hash(&encryption_key, file_id, revision).unwrap();

    let result = authenticate_owner_password(
        owner_pass,
        &user_hash,
        &owner_hash,
        permissions,
        file_id,
        revision,
        key_length,
        true,
        None,
    )
    .unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap(), encryption_key);
}

#[test]
fn test_authenticate_owner_password_wrong_password() {
    let owner_pass = b"owner123";
    let user_pass = b"user123";
    let file_id = b"test_file_id_123";
    let permissions = -1i32;
    let revision = 3;
    let key_length = 16;

    let owner_hash = compute_owner_password_hash(owner_pass, user_pass, revision, key_length).unwrap();
    let encryption_key =
        compute_encryption_key(user_pass, &owner_hash, permissions, file_id, revision, key_length, true).unwrap();
    let user_hash = compute_user_password_hash(&encryption_key, file_id, revision).unwrap();

    let result = authenticate_owner_password(
        b"wrong",
        &user_hash,
        &owner_hash,
        permissions,
        file_id,
        revision,
        key_length,
        true,
        None,
    )
    .unwrap();
    assert!(result.is_none());
}

#[test]
fn test_authenticate_owner_password_r5_roundtrip() {
    let password = b"ownerpass";
    let owner_validation_salt = [0x11u8; 8];
    let owner_key_salt = [0x22u8; 8];

    let user_key = [0xAAu8; 48];

    let mut hasher = Sha256::new();
    hasher.update(password.as_slice());
    hasher.update(owner_validation_salt);
    hasher.update(&user_key[..48]);
    let hash = hasher.finalize();

    let mut owner_key = hash.to_vec();
    owner_key.extend_from_slice(&owner_validation_salt);
    owner_key.extend_from_slice(&owner_key_salt);

    // Wrap a known file key into /OE with the spec algorithm:
    // OE = AES-256-CBC(key = SHA-256(password || owner_key_salt || U), iv = 0, file key). ~keep
    let file_key = [0xC3u8; 32];
    let mut hasher = Sha256::new();
    hasher.update(password.as_slice());
    hasher.update(owner_key_salt);
    hasher.update(&user_key[..48]);
    let intermediate = hasher.finalize();
    let oe = crate::encryption::aes::aes256_encrypt_no_padding(&intermediate, &[0u8; 16], &file_key).unwrap();

    let result = authenticate_owner_password(password, &user_key, &owner_key, -1, b"", 5, 32, true, Some(&oe)).unwrap();
    // The file key is the unwrapped /OE — never the intermediate hash. ~keep
    assert_eq!(result.unwrap(), file_key.to_vec());

    // Without /OE the file key is unobtainable; authentication must fail. ~keep
    let no_oe = authenticate_owner_password(password, &user_key, &owner_key, -1, b"", 5, 32, true, None).unwrap();
    assert!(no_oe.is_none());
}

#[test]
fn test_authenticate_owner_password_r5_wrong_password() {
    let password = b"ownerpass";
    let owner_validation_salt = [0x11u8; 8];
    let owner_key_salt = [0x22u8; 8];
    let user_key = [0xAAu8; 48];

    let mut hasher = Sha256::new();
    hasher.update(password.as_slice());
    hasher.update(owner_validation_salt);
    hasher.update(&user_key[..48]);
    let hash = hasher.finalize();

    let mut owner_key = hash.to_vec();
    owner_key.extend_from_slice(&owner_validation_salt);
    owner_key.extend_from_slice(&owner_key_salt);

    let result = authenticate_owner_password(b"wrong", &user_key, &owner_key, -1, b"", 5, 32, true, None).unwrap();
    assert!(result.is_none());
}
