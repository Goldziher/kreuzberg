//! Deterministic element identifier for [`InternalElement`](super::InternalElement).

use std::fmt;

use serde::{Deserialize, Serialize};

#[cfg_attr(alef, alef(skip))]
/// Deterministic element identifier, generated via blake3 hashing.
///
/// Format: `"ie-{12 hex chars}"` (48 bits from blake3, ~281 trillion address space).
/// Same input always produces the same ID, enabling diffing and caching.
///
/// Serializes as a plain string (`"ie-aabbccddeeff"`).
#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InternalElementId([u8; 15]);

impl Serialize for InternalElementId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InternalElementId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        if s.len() != 15 {
            return Err(serde::de::Error::custom(format!(
                "InternalElementId must be 15 bytes, got {}",
                s.len()
            )));
        }
        let mut buf = [0u8; 15];
        buf.copy_from_slice(s.as_bytes());
        Ok(Self(buf))
    }
}

impl InternalElementId {
    /// Generate a deterministic ID from element content.
    ///
    /// Hashes the element kind discriminant, text content, page number, and
    /// positional index using blake3. Takes 48 bits (6 bytes) of the hash.
    pub(crate) fn generate(kind_discriminant: &str, text: &str, page: Option<u32>, index: u32) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(kind_discriminant.as_bytes());
        hasher.update(text.as_bytes());
        hasher.update(&page.unwrap_or(u32::MAX).to_le_bytes());
        hasher.update(&index.to_le_bytes());
        let hash = hasher.finalize();
        let bytes = &hash.as_bytes()[..6];
        let mut buf = [0u8; 15];
        buf[0] = b'i';
        buf[1] = b'e';
        buf[2] = b'-';
        hex::encode_to_slice(bytes, &mut buf[3..]).expect("fixed size");
        Self(buf)
    }

    /// Get the ID as a string slice.
    pub(crate) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap()
    }
}

impl fmt::Display for InternalElementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for InternalElementId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
