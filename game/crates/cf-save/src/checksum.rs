//! **M4B § "Save corruption is detectable"** — canonical-JSON BLAKE3 over
//! save blobs.
//!
//! The canonical representation is what `serde_json::to_string` (compact,
//! key-ordered for `BTreeMap`s, no trailing whitespace) emits. Two stable
//! properties make this hash a sound integrity check:
//!
//! 1. `BTreeMap` key ordering is fixed lex order, so the JSON byte sequence
//!    is invariant under map permutation.
//! 2. `f32` / `f64` formatting goes through Rust's `Display` which is
//!    deterministic across platforms per DR-052 ("float determinism rules").
//!
//! Callers that need a hash over a sub-shape (per-actor blob, per-chunk
//! snapshot) pass the canonical JSON bytes directly to [`blake3_hex_of`].

/// Hex BLAKE3 of an arbitrary byte slice. Zero-allocation wrapper around
/// `blake3::hash` + `hex::encode`.
pub fn blake3_hex_of(bytes: &[u8]) -> String {
    let hash = blake3::hash(bytes);
    hex::encode(hash.as_bytes())
}

/// Hex BLAKE3 of the canonical JSON form of a Serialize value. Returns
/// `serde_json::Error` rather than the M4B `SaveError` so this helper can be
/// reused from cf-replay (which doesn't depend on cf-save's SaveError).
pub fn canonical_blake3_hex<T: serde::Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let canonical = serde_json::to_string(value)?;
    Ok(blake3_hex_of(canonical.as_bytes()))
}

/// over a byte slice using a 32-byte key. Used by [`crate::ledger_chain`]
/// to bind the per-event chain to the run id + scenario seed.
pub fn blake3_keyed_hex_of(key: &[u8; 32], bytes: &[u8]) -> String {
    let hash = blake3::keyed_hash(key, bytes);
    hex::encode(hash.as_bytes())
}

/// Derive a 32-byte BLAKE3 key from an arbitrary string (typically
/// `manifest.run_id + scenario seed`). Centralizes the derivation rule so
/// the encoder and the verifier always agree.
pub fn derive_chain_key(material: &str) -> [u8; 32] {
    let hash = blake3::hash(material.as_bytes());
    *hash.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blake3_hex_is_deterministic_over_same_input() {
        let a = blake3_hex_of(b"hello world");
        let b = blake3_hex_of(b"hello world");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn canonical_form_is_invariant_under_btreemap_construction_order() {
        use std::collections::BTreeMap;
        let mut m1: BTreeMap<String, u32> = BTreeMap::new();
        m1.insert("a".to_string(), 1);
        m1.insert("b".to_string(), 2);
        m1.insert("c".to_string(), 3);
        let mut m2: BTreeMap<String, u32> = BTreeMap::new();
        m2.insert("c".to_string(), 3);
        m2.insert("a".to_string(), 1);
        m2.insert("b".to_string(), 2);
        let h1 = canonical_blake3_hex(&m1).unwrap();
        let h2 = canonical_blake3_hex(&m2).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn keyed_hash_differs_from_unkeyed_hash() {
        let key = derive_chain_key("run-abc:seed=42");
        let plain = blake3_hex_of(b"payload");
        let keyed = blake3_keyed_hex_of(&key, b"payload");
        assert_ne!(plain, keyed);
    }

    #[test]
    fn derive_chain_key_is_deterministic() {
        let a = derive_chain_key("run-abc:seed=42");
        let b = derive_chain_key("run-abc:seed=42");
        assert_eq!(a, b);
        let c = derive_chain_key("run-abc:seed=43");
        assert_ne!(a, c);
    }
}
