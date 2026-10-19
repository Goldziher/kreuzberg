//! PDF encryption algorithms.
//!
//! This module implements the cryptographic algorithms specified in the PDF specification
//! for key derivation and password validation.
//!
//! PDF Spec: Section 7.6.3 - Standard Security Handler
//! PDF 2.0 Spec (ISO 32000-2:2020): Section 7.6.4.3.3 - Algorithm 8-11 for R>=5

use sha2::{Digest, Sha256, Sha384, Sha512};

/// Padding string used in PDF encryption (32 bytes).
///
/// PDF Spec: Algorithm 2, step 1
const PADDING: &[u8; 32] = b"\x28\xBF\x4E\x5E\x4E\x75\x8A\x41\
                              \x64\x00\x4E\x56\xFF\xFA\x01\x08\
                              \x2E\x2E\x00\xB6\xD0\x68\x3E\x80\
                              \x2F\x0C\xA9\xFE\x64\x53\x69\x7A";

/// Compute the encryption key from a password (Algorithm 2).
///
/// PDF Spec: Section 7.6.3.3 - Algorithm 2: Computing an encryption key
///
/// # Arguments
///
/// * `password` - User or owner password (up to 32 bytes)
/// * `owner_key` - 32-byte owner password hash from encryption dictionary
/// * `permissions` - User access permissions (P field)
/// * `file_id` - First element of file identifier array
/// * `revision` - Encryption revision number (R field)
/// * `key_length` - Key length in bytes
/// * `encrypt_metadata` - Whether to encrypt metadata
///
/// # Returns
///
/// The derived encryption key
pub fn compute_encryption_key(
    password: &[u8],
    owner_key: &[u8],
    permissions: i32,
    file_id: &[u8],
    revision: u32,
    key_length: usize,
    encrypt_metadata: bool,
) -> crate::Result<Vec<u8>> {
    // For R>=5, the encryption key is randomly generated, not derived from password
    // PDF 2.0 Spec: Algorithm 8 generates a random 32-byte file encryption key ~keep
    if revision >= 5 {
        return generate_random_encryption_key(key_length);
    }

    // #230 Phase C: route the primitive through the governed
    // provider (byte-identical under the default `compat` policy). ~keep
    let mut hasher = super::md5_kdf_hasher()?;

    // Step a: Pad or truncate password to 32 bytes ~keep
    let mut padded_password = [0u8; 32];
    let pass_len = password.len().min(32);
    padded_password[..pass_len].copy_from_slice(&password[..pass_len]);
    if pass_len < 32 {
        padded_password[pass_len..].copy_from_slice(&PADDING[..(32 - pass_len)]);
    }

    // Step b: Pass the password to MD5 ~keep
    hasher.update(&padded_password);

    // Step c: Pass the owner password hash ~keep
    hasher.update(owner_key);

    // Step d: Pass permissions as 32-bit little-endian ~keep
    hasher.update(&permissions.to_le_bytes());

    // Step e: Pass the file identifier ~keep
    hasher.update(file_id);

    // Step f: For R >= 4, if EncryptMetadata is false, pass 0xFFFFFFFF ~keep
    if revision >= 4 && !encrypt_metadata {
        hasher.update(&[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    // Step g: Finish MD5 hash ~keep
    let mut hash = hasher.finalize();

    // Step h: For R >= 3, do 50 additional MD5 iterations on first key_length bytes ~keep
    if revision >= 3 {
        for _ in 0..50 {
            let mut h = super::md5_kdf_hasher()?;
            h.update(&hash[..key_length.min(16)]);
            hash = h.finalize();
        }
    }

    // Step i: Return first key_length bytes (max 16 for MD5) ~keep
    Ok(hash[..key_length.min(16)].to_vec())
}

/// Generate a random encryption key for R>=5.
///
/// PDF 2.0 Spec: For AES-256, the file encryption key is randomly
/// generated (ISO 32000-2 §7.6.4.4). Routes through the active
/// [`crate::crypto::CryptoProvider`]'s `random_bytes` so the FIPS
/// provider can supply OS RNG via `aws_lc_rs::rand::SystemRandom`
/// instead of the previous `SHA-256(uuid_v4 || uuid_v4 ||
/// timestamp_ns)` cascade — the latter is not cryptographically
/// suitable as a key generator and is rejected by FIPS auditors.
/// Issue #236.
fn generate_random_encryption_key(key_length: usize) -> crate::Result<Vec<u8>> {
    generate_random_bytes(key_length)
}

/// Pad or truncate a password to 32 bytes using the standard padding.
///
/// PDF Spec: Algorithm 2, step 1
pub fn pad_password(password: &[u8]) -> Vec<u8> {
    let mut padded = Vec::with_capacity(32);
    let pass_len = password.len().min(32);
    padded.extend_from_slice(&password[..pass_len]);
    if pass_len < 32 {
        padded.extend_from_slice(&PADDING[..(32 - pass_len)]);
    }
    padded
}

/// Authenticate the user password (Algorithm 4/5 for R<=4, Algorithm 11 for R>=5).
///
/// PDF Spec: Section 7.6.3.4 - Algorithm 4/5: User password authentication
/// PDF 2.0 Spec: Algorithm 11 - Authenticating user password for R>=5
///
/// Returns the encryption key if authentication succeeds.
pub fn authenticate_user_password(
    password: &[u8],
    user_key: &[u8],
    owner_key: &[u8],
    permissions: i32,
    file_id: &[u8],
    revision: u32,
    key_length: usize,
    encrypt_metadata: bool,
    user_encryption: Option<&[u8]>,
) -> Option<Vec<u8>> {
    // R>=5 uses SHA-256 based verification (Algorithm 11 for R5, Algorithm 2.A for R6) ~keep
    if revision >= 5 {
        return authenticate_user_password_r5_r6(password, user_key, revision, user_encryption);
    }

    let key = compute_encryption_key(
        password,
        owner_key,
        permissions,
        file_id,
        revision,
        key_length,
        encrypt_metadata,
    )
    .ok()?;

    let expected_user_key = if revision >= 3 {
        compute_user_key_r3(&key, file_id).ok()?
    } else {
        compute_user_key_r2(&key).ok()?
    };

    // Compare first 16 bytes (constant-time comparison) ~keep
    if user_key.len() < 16 || expected_user_key.len() < 16 {
        return None;
    }
    let matches = constant_time_compare(&user_key[..16], &expected_user_key[..16]);

    if matches { Some(key) } else { None }
}

/// Verify user password for R>=5 (PDF 2.0 Algorithm 11 for R5, Algorithm 2.A for R6).
///
/// R5: Simple SHA-256 hash comparison.
/// R6: Uses Algorithm 2.B (iterative hash with SHA-256/384/512 and AES-CBC).
fn authenticate_user_password_r5_r6(
    password: &[u8],
    user_key: &[u8],
    revision: u32,
    user_encryption: Option<&[u8]>,
) -> Option<Vec<u8>> {
    if user_key.len() < 48 {
        return None;
    }

    let password = saslprep_password(password);
    let password = truncate_password_utf8(&password);

    let validation_salt = &user_key[32..40];
    let key_salt = &user_key[40..48];

    let hash = if revision >= 6 {
        // R6: Algorithm 2.B (ISO 32000-2:2020 S7.6.4.3.4) ~keep
        algorithm_2b(&password, validation_salt, &[])
    } else {
        // R5: Simple SHA-256(password || validation_salt) ~keep
        let mut hasher = Sha256::new();
        hasher.update(&password);
        hasher.update(validation_salt);
        hasher.finalize().to_vec()
    };

    if !constant_time_compare(&hash[..32], &user_key[..32]) {
        return None;
    }

    // The intermediate key differs between revisions (plain SHA-256 for R5,
    // Algorithm 2.B for R6), but for both the file key is the AES-256
    // unwrap of /UE with that intermediate key (ISO 32000-2 §7.6.4.4.8;
    // Adobe Supplement ExtensionLevel 3 for R5). The intermediate key is
    // never the file key itself. ~keep
    let ue = user_encryption?;
    if ue.len() < 32 {
        return None;
    }
    let intermediate_key = if revision >= 6 {
        algorithm_2b(&password, key_salt, &[])
    } else {
        let mut hasher = Sha256::new();
        hasher.update(&password);
        hasher.update(key_salt);
        hasher.finalize().to_vec()
    };
    let iv = [0u8; 16];
    super::aes::aes256_decrypt_no_padding(&intermediate_key[..32], &iv, &ue[..32]).ok()
}

/// Apply SASLprep (RFC 4013) normalization to a password.
///
/// PDF 2.0 Spec requires SASLprep for Unicode passwords in R>=5.
/// Falls back to raw bytes if the input is not valid UTF-8 or normalization fails.
fn saslprep_password(password: &[u8]) -> Vec<u8> {
    let Ok(password_str) = std::str::from_utf8(password) else {
        return password.to_vec();
    };
    match stringprep::saslprep(password_str) {
        Ok(normalized) => normalized.as_bytes().to_vec(),
        Err(_) => password.to_vec(),
    }
}

/// ISO 32000-2:2020 Algorithm 2.B — Computing a hash (revision 6).
///
/// This iterative hash algorithm uses SHA-256, SHA-384, and SHA-512 combined
/// with AES-128-CBC encryption. It replaces simple SHA-256 hashing used in R5.
///
/// # Arguments
/// * `password` - The preprocessed password (SASLprep'd and truncated)
/// * `salt` - 8-byte salt (validation_salt or key_salt)
/// * `user_key` - Additional data: empty for user auth, U[0..48] for owner auth
fn algorithm_2b(password: &[u8], salt: &[u8], user_key: &[u8]) -> Vec<u8> {
    // Step 1: Initial hash = SHA-256(password || salt || user_key) ~keep
    let mut hasher = Sha256::new();
    hasher.update(password);
    hasher.update(salt);
    hasher.update(user_key);
    let mut k = hasher.finalize().to_vec();

    let mut round: usize = 0;
    loop {
        // Step a: Build K1 = (password || K || user_key) repeated 64 times ~keep
        let k1_unit_len = password.len() + k.len() + user_key.len();
        let mut k1 = Vec::with_capacity(k1_unit_len * 64);
        for _ in 0..64 {
            k1.extend_from_slice(password);
            k1.extend_from_slice(&k);
            k1.extend_from_slice(user_key);
        }

        // Pad K1 to multiple of 16 for AES-CBC ~keep
        let remainder = k1.len() % 16;
        if remainder != 0 {
            k1.extend(std::iter::repeat_n(0u8, 16 - remainder));
        }

        // Step b: E = AES-128-CBC-encrypt(key=K[0..16], iv=K[16..32], data=K1) ~keep
        let aes_key = &k[..16];
        let aes_iv = &k[16..32];
        let e = match super::aes::aes128_encrypt_no_padding(aes_key, aes_iv, &k1) {
            Ok(encrypted) => encrypted,
            Err(_) => return k,
        };

        // Step c: Determine next hash algorithm.
        // Sum of first 16 bytes of E, mod 3 ~keep
        let sum: u32 = e.iter().take(16).map(|&b| b as u32).sum();
        let remainder = sum % 3;

        // Step d: Hash E with selected algorithm ~keep
        k = match remainder {
            0 => {
                let mut h = Sha256::new();
                h.update(&e);
                h.finalize().to_vec()
            }
            1 => {
                let mut h = Sha384::new();
                h.update(&e);
                h.finalize().to_vec()
            }
            _ => {
                let mut h = Sha512::new();
                h.update(&e);
                h.finalize().to_vec()
            }
        };

        // Step e: per ISO 32000-2:2020 Algorithm 2.B step f, the round
        // counter increments before the termination check. Stop once at
        // least 64 rounds have run AND the last byte of E is ≤ round - 32. ~keep
        round += 1;
        let last_byte = *e.last().unwrap_or(&0) as usize;
        if round >= 64 && last_byte <= round.saturating_sub(32) {
            break;
        }
    }

    k.truncate(32);
    k
}

/// Compute the user password hash for R=2 (Algorithm 4).
///
/// PDF Spec: Section 7.6.3.4 - Algorithm 4
fn compute_user_key_r2(key: &[u8]) -> crate::Result<Vec<u8>> {
    super::rc4::rc4_crypt(key, PADDING)
}

/// Compute the user password hash for R>=3 (Algorithm 5).
///
/// PDF Spec: Section 7.6.3.4 - Algorithm 5
fn compute_user_key_r3(key: &[u8], file_id: &[u8]) -> crate::Result<Vec<u8>> {
    // Step a: Create MD5 hash of padding + file ID (#230 Phase C:
    // governed provider; byte-identical under the default policy). ~keep
    let mut hasher = super::md5_kdf_hasher()?;
    hasher.update(PADDING);
    hasher.update(file_id);
    let mut hash = hasher.finalize();

    // Step b: Encrypt the hash 20 times with modified keys ~keep
    for i in 0..20 {
        let mut modified_key = key.to_vec();
        for byte in &mut modified_key {
            *byte ^= i as u8;
        }
        hash = super::rc4::rc4_crypt(&modified_key, &hash)?;
    }

    // Step c: Append 16 arbitrary bytes (we use zeros) ~keep
    hash.extend_from_slice(&[0u8; 16]);
    Ok(hash)
}

/// Compute the owner password hash (Algorithm 3 for R<=4, Algorithm 8 for R>=5).
///
/// PDF Spec: Section 7.6.3.3 - Algorithm 3: Computing the O value (R<=4)
/// PDF 2.0 Spec: Algorithm 8: Computing O and U for R>=5
///
/// This generates the /O value for the encryption dictionary.
///
/// # Arguments
///
/// * `owner_password` - Owner password (if empty, uses user_password)
/// * `user_password` - User password
/// * `revision` - Encryption revision (R value: 2, 3, 4, 5, or 6)
/// * `key_length` - Key length in bytes (5 for 40-bit, 16 for 128-bit, 32 for 256-bit)
///
/// # Returns
///
/// 32-byte owner password hash for /O entry.
///
/// # R>=5 is not supported here
///
/// ISO 32000-2 Algorithm 8 requires `U[0..48]` and the random file
/// encryption key as inputs to the O/OE computation for revision 5/6 —
/// neither of which this function's signature carries. Returning a
/// hash computed without them would silently produce a
/// spec-noncompliant /O value that no conformant reader (and not even
/// this crate's own [`authenticate_owner_password`]) can authenticate
/// against — exactly the "confidently wrong output" this crate refuses
/// to produce. Call [`compute_u_and_ue`] to get `U` and the file key,
/// then [`compute_o_and_oe`] to get a spec-correct `O`/`OE` pair; the
/// write path ([`super::EncryptDictBuilder::build`]) already does
/// this.
pub fn compute_owner_password_hash(
    owner_password: &[u8],
    user_password: &[u8],
    revision: u32,
    key_length: usize,
) -> crate::Result<Vec<u8>> {
    if revision >= 5 {
        return Err(crate::Error::InvalidPdf(
            "compute_owner_password_hash does not support revision >= 5: ISO 32000-2 \
             Algorithm 8 requires U[0..48] and the file encryption key, which this \
             function's signature cannot supply. Use compute_u_and_ue to obtain U and \
             the file key, then compute_o_and_oe to obtain a spec-correct O/OE pair."
                .to_string(),
        ));
    }

    // Algorithm 3 for R<=4 ~keep
    // Step a: Use owner password, or user password if owner is empty ~keep
    let password = if owner_password.is_empty() {
        user_password
    } else {
        owner_password
    };

    // Step b: Pad the password to 32 bytes ~keep
    let padded_password = pad_password(password);

    // Step c: Initialize MD5 and pass the padded password (#230
    // Phase C: governed provider; byte-identical default policy). ~keep
    let mut hasher = super::md5_kdf_hasher()?;
    hasher.update(&padded_password);
    let mut hash = hasher.finalize();

    // Step d: For R >= 3, do 50 additional MD5 iterations ~keep
    if revision >= 3 {
        for _ in 0..50 {
            let mut h = super::md5_kdf_hasher()?;
            h.update(&hash[..key_length.min(16)]);
            hash = h.finalize();
        }
    }

    // Step e: Use first key_length bytes as RC4 key (max 16) ~keep
    let rc4_key_len = key_length.min(16);
    let rc4_key = &hash[..rc4_key_len];

    // Step f: Pad the user password ~keep
    let padded_user = pad_password(user_password);

    // Step g: RC4 encrypt the padded user password ~keep
    let mut result = super::rc4::rc4_crypt(rc4_key, &padded_user)?;

    // Step h: For R >= 3, do 19 more RC4 encryptions with XOR'd keys ~keep
    if revision >= 3 {
        for i in 1..=19 {
            let mut modified_key = rc4_key.to_vec();
            for byte in &mut modified_key {
                *byte ^= i as u8;
            }
            result = super::rc4::rc4_crypt(&modified_key, &result)?;
        }
    }

    Ok(result)
}

/// Compute the user password hash for the encryption dictionary (Algorithm 4/5/8).
///
/// PDF Spec: Section 7.6.3.4 - Algorithm 4 (R=2) and Algorithm 5 (R>=3)
/// PDF 2.0 Spec: Algorithm 8 - Computing U for R>=5
///
/// This generates the /U value for the encryption dictionary.
///
/// # Arguments
///
/// * `encryption_key` - The computed encryption key from Algorithm 2
/// * `file_id` - First element of file identifier array (only used for R>=3)
/// * `revision` - Encryption revision (R value)
///
/// # Returns
///
/// 32-byte user password hash for /U entry (48 bytes for R>=5)
pub fn compute_user_password_hash(encryption_key: &[u8], file_id: &[u8], revision: u32) -> crate::Result<Vec<u8>> {
    if revision >= 5 {
        // For R>=5, use the encryption key directly as user password indicator
        // This creates the U value with validation/key salts ~keep
        return compute_user_hash_r5(encryption_key);
    }
    if revision >= 3 {
        compute_user_key_r3(encryption_key, file_id)
    } else {
        compute_user_key_r2(encryption_key)
    }
}

/// Compute user password hash for R>=5 (Algorithm 8 part).
///
/// PDF 2.0 Spec: Algorithm 8 - Computing U value for R=5/6
///
/// For R>=5, the U value is 48 bytes:
/// - Bytes 0-31: SHA-256(password || user_validation_salt)
/// - Bytes 32-39: user_validation_salt (random 8 bytes)
/// - Bytes 40-47: user_key_salt (random 8 bytes)
fn compute_user_hash_r5(user_password: &[u8]) -> crate::Result<Vec<u8>> {
    let validation_salt = generate_random_bytes(8)?;
    let key_salt = generate_random_bytes(8)?;

    let password = truncate_password_utf8(user_password);

    let mut hasher = Sha256::new();
    hasher.update(&password);
    hasher.update(&validation_salt);
    let hash = hasher.finalize();

    let mut result = hash.to_vec();
    result.extend_from_slice(&validation_salt);
    result.extend_from_slice(&key_salt);

    Ok(result)
}

/// PDF 2.0 Algorithm 8: compute U, UE, and the random file encryption key for R>=5.
///
/// Returns `(U, UE, file_encryption_key)` where:
/// - `U` is 48 bytes: hash(password, validation_salt) || validation_salt || key_salt
/// - `UE` is 32 bytes: AES256-CBC(key=hash(password, key_salt), IV=0, data=file_key)
/// - `file_encryption_key` is the 32-byte random key used to encrypt streams
pub fn compute_u_and_ue(
    user_password: &[u8],
    key_length: usize,
    revision: u32,
) -> crate::Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    let file_key = generate_random_encryption_key(key_length)?;
    let validation_salt = generate_random_bytes(8)?;
    let key_salt = generate_random_bytes(8)?;

    let password = saslprep_password(user_password);
    let password = truncate_password_utf8(&password);

    let hash = if revision >= 6 {
        algorithm_2b(&password, &validation_salt, &[])
    } else {
        let mut h = Sha256::new();
        h.update(&password);
        h.update(&validation_salt);
        h.finalize().to_vec()
    };

    let mut u = hash;
    u.extend_from_slice(&validation_salt);
    u.extend_from_slice(&key_salt);

    let intermediate_key = if revision >= 6 {
        algorithm_2b(&password, &key_salt, &[])
    } else {
        let mut h = Sha256::new();
        h.update(&password);
        h.update(&key_salt);
        h.finalize().to_vec()
    };

    // PDF 2.0 spec (ISO 32000-2 Algorithm 8) mandates a zero IV for AES-256-CBC key wrapping.
    // Security comes from the random file_key and key_salt, not the IV. ~keep
    let iv: [u8; 16] = std::array::from_fn(|_| 0);
    let ue = super::aes::aes256_encrypt_no_padding(&intermediate_key[..32], &iv, &file_key).unwrap_or_default();

    Ok((u, ue, file_key))
}

/// PDF 2.0 Algorithm 9: compute O and OE for R>=5.
///
/// Returns `(O, OE)` where:
/// - `O` is 48 bytes: hash(owner_password, validation_salt, U) || validation_salt || key_salt
/// - `OE` is 32 bytes: AES256-CBC(key=hash(owner_password, key_salt, U), IV=0, data=file_key)
pub fn compute_o_and_oe(
    owner_password: &[u8],
    _user_password: &[u8],
    file_key: &[u8],
    u: &[u8],
    revision: u32,
) -> crate::Result<(Vec<u8>, Vec<u8>)> {
    let validation_salt = generate_random_bytes(8)?;
    let key_salt = generate_random_bytes(8)?;

    let password = saslprep_password(owner_password);
    let password = truncate_password_utf8(&password);
    let u48 = &u[..u.len().min(48)];

    let hash = if revision >= 6 {
        algorithm_2b(&password, &validation_salt, u48)
    } else {
        let mut h = Sha256::new();
        h.update(&password);
        h.update(&validation_salt);
        h.update(u48);
        h.finalize().to_vec()
    };

    let mut o = hash;
    o.extend_from_slice(&validation_salt);
    o.extend_from_slice(&key_salt);

    let intermediate_key = if revision >= 6 {
        algorithm_2b(&password, &key_salt, u48)
    } else {
        let mut h = Sha256::new();
        h.update(&password);
        h.update(&key_salt);
        h.update(u48);
        h.finalize().to_vec()
    };

    // PDF 2.0 spec (ISO 32000-2 Algorithm 9) mandates a zero IV for AES-256-CBC key wrapping.
    // Security comes from the random file_key and key_salt, not the IV. ~keep
    let iv: [u8; 16] = std::array::from_fn(|_| 0);
    let oe = super::aes::aes256_encrypt_no_padding(&intermediate_key[..32], &iv, file_key).unwrap_or_default();

    Ok((o, oe))
}

/// Generate cryptographically strong random bytes from the active
/// [`crypto::CryptoProvider`]. Both shipped providers source this
/// from the OS entropy pool — `getrandom::fill()` for the default
/// `RustCryptoProvider` and `aws_lc_rs::rand::SystemRandom` for the
/// FIPS provider. Issue #236.
///
/// Returns [`crate::Error::InvalidPdf`] if the OS RNG fails. Modern
/// Linux (`getrandom(2)` since 3.17) and BSDs / macOS / Windows all
/// guarantee `getrandom`-equivalent never blocks once the entropy
/// pool is initialized, so this should be unreachable in practice —
/// but propagating the error keeps `xberg-native-pdf` from crashing the
/// host process if it ever fires.
///
/// [`crypto::CryptoProvider`]: crate::crypto::CryptoProvider
fn generate_random_bytes(len: usize) -> crate::Result<Vec<u8>> {
    let mut buf = vec![0u8; len];
    crate::crypto::active().random_bytes(&mut buf).map_err(|e| {
        crate::Error::InvalidPdf(format!(
            "OS RNG failure from CryptoProvider '{}': {}",
            crate::crypto::active().name(),
            e
        ))
    })?;
    Ok(buf)
}

/// Truncate password to 127 bytes for UTF-8 (R>=5 requirement).
///
/// PDF 2.0 Spec: For R>=5, passwords are UTF-8 encoded and
/// limited to 127 bytes.
fn truncate_password_utf8(password: &[u8]) -> Vec<u8> {
    let mut result = password.to_vec();
    if result.len() > 127 {
        // Find UTF-8 boundary for truncation ~keep
        let mut end = 127;
        while end > 0 && (result[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        result.truncate(end);
    }
    result
}

/// Authenticate the owner password (Algorithm 7 for R≤4, Algorithm 12 for R≥5).
///
/// PDF Spec: Section 7.6.3.4 - Algorithm 7: Owner password authentication
/// PDF 2.0 Spec: Algorithm 12 - Authenticating owner password for R>=5
///
/// Returns the encryption key if authentication succeeds.
pub fn authenticate_owner_password(
    owner_password: &[u8],
    user_key: &[u8],
    owner_key: &[u8],
    permissions: i32,
    file_id: &[u8],
    revision: u32,
    key_length: usize,
    encrypt_metadata: bool,
    owner_encryption: Option<&[u8]>,
) -> crate::Result<Option<Vec<u8>>> {
    if revision >= 5 {
        return Ok(authenticate_owner_password_r5_r6(
            owner_password,
            owner_key,
            user_key,
            revision,
            owner_encryption,
        ));
    }

    // Algorithm 7: Authenticate owner password for R≤4 ~keep
    // Steps a-d: Compute RC4 key from owner password (same as Algorithm 3 steps a-d) ~keep
    if owner_password.is_empty() {
        return Ok(None);
    }
    let padded_password = pad_password(owner_password);

    // #230 Phase C: governed provider; byte-identical default policy. ~keep
    let mut hasher = super::md5_kdf_hasher()?;
    hasher.update(&padded_password);
    let mut hash = hasher.finalize();

    if revision >= 3 {
        for _ in 0..50 {
            let mut h = super::md5_kdf_hasher()?;
            h.update(&hash[..key_length.min(16)]);
            hash = h.finalize();
        }
    }

    let rc4_key_len = key_length.min(16);
    let rc4_key = &hash[..rc4_key_len];

    // Step e: Decrypt the /O value to recover the padded user password ~keep
    let user_password_padded = if revision == 2 {
        // R=2: Single RC4 decryption ~keep
        super::rc4::rc4_crypt(rc4_key, owner_key)?
    } else {
        // R≥3: 20 RC4 decryptions with XOR'd keys (19 down to 0) ~keep
        let mut result = owner_key.to_vec();
        for i in (0..=19).rev() {
            let mut modified_key = rc4_key.to_vec();
            for byte in &mut modified_key {
                *byte ^= i as u8;
            }
            result = super::rc4::rc4_crypt(&modified_key, &result)?;
        }
        result
    };

    // Step f: Use recovered user password to authenticate via Algorithm 6 ~keep
    Ok(authenticate_user_password(
        &user_password_padded,
        user_key,
        owner_key,
        permissions,
        file_id,
        revision,
        key_length,
        encrypt_metadata,
        None,
    ))
}

/// Verify owner password for R>=5 (PDF 2.0 Algorithm 12 for R5, Algorithm 2.A for R6).
///
/// R5: Simple SHA-256 hash comparison.
/// R6: Uses Algorithm 2.B (iterative hash with SHA-256/384/512 and AES-CBC).
fn authenticate_owner_password_r5_r6(
    password: &[u8],
    owner_key: &[u8],
    user_key: &[u8],
    revision: u32,
    owner_encryption: Option<&[u8]>,
) -> Option<Vec<u8>> {
    if owner_key.len() < 48 || user_key.len() < 48 {
        return None;
    }

    let password = saslprep_password(password);
    let password = truncate_password_utf8(&password);

    let owner_validation_salt = &owner_key[32..40];
    let owner_key_salt = &owner_key[40..48];
    let u_value = &user_key[..48];

    let hash = if revision >= 6 {
        // R6: Algorithm 2.B with U[0..48] as additional data ~keep
        algorithm_2b(&password, owner_validation_salt, u_value)
    } else {
        // R5: SHA-256(password || owner_validation_salt || U[0..48]) ~keep
        let mut hasher = Sha256::new();
        hasher.update(&password);
        hasher.update(owner_validation_salt);
        hasher.update(u_value);
        hasher.finalize().to_vec()
    };

    if !constant_time_compare(&hash[..32], &owner_key[..32]) {
        return None;
    }

    // Same shape as the user path: the revision decides the intermediate
    // hash, and the file key is always the AES-256 unwrap of /OE with it
    // (ISO 32000-2 §7.6.4.4.8; Adobe Supplement ExtensionLevel 3 for R5). ~keep
    let oe = owner_encryption?;
    if oe.len() < 32 {
        return None;
    }
    let intermediate_key = if revision >= 6 {
        algorithm_2b(&password, owner_key_salt, u_value)
    } else {
        let mut hasher = Sha256::new();
        hasher.update(&password);
        hasher.update(owner_key_salt);
        hasher.update(u_value);
        hasher.finalize().to_vec()
    };
    let iv = [0u8; 16];
    super::aes::aes256_decrypt_no_padding(&intermediate_key[..32], &iv, &oe[..32]).ok()
}

/// Constant-time comparison to prevent timing attacks.
///
/// Returns true if the slices are equal.
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests;
