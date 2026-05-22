//! M8B § Reed-Solomon FEC on small reliable payloads.
//!
//! Per M8B spec § Notes: FEC is used ONLY on small reliable payloads
//! (event batches < 8 kB). Larger payloads rely on QUIC's own stream
//! retransmit, which is more efficient.
//!
//! Algorithm: systematic Reed-Solomon over GF(256) using a Vandermonde
//! generator matrix. For each (k, m) group:
//!
//! - The first k shards are the original data shards (split evenly).
//! - The next m shards are parity shards = generator_matrix × data_vec.
//! - On decode, the receiver collects any k of the (k+m) shards and
//!   recovers the original data via Gaussian elimination.
//!
//! Default: k=4 data + m=2 parity. Survives up to 2 erasures.

use serde::{Deserialize, Serialize};

/// payloads are handed off to QUIC stream retransmit per the spec.
pub const FEC_MAX_PAYLOAD_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FecShard {
    pub group_id: u64,
    pub shard_index: u8,
    pub k: u8,
    pub m: u8,
    pub data: Vec<u8>,
}

/// A full FEC group: k data + m parity shards.
#[derive(Debug, Clone)]
pub struct FecGroup {
    pub group_id: u64,
    pub k: u8,
    pub m: u8,
    pub shard_len: usize,
    /// Original payload length (so the receiver can truncate trailing
    /// padding bytes from the last data shard).
    pub original_len: usize,
    pub shards: Vec<FecShard>,
}

#[derive(Debug, thiserror::Error)]
pub enum FecError {
    #[error("payload too large for FEC: {0} bytes > {FEC_MAX_PAYLOAD_BYTES} max")]
    PayloadTooLarge(usize),
    #[error("invalid (k, m): k={k}, m={m}; both must be 1..=8 and k+m ≤ 16")]
    InvalidKM { k: u8, m: u8 },
    #[error("too few surviving shards: have {have}, need {need}")]
    TooFewShards { have: usize, need: usize },
    #[error("shard length mismatch: got {got}, expected {expected}")]
    ShardLenMismatch { got: usize, expected: usize },
    #[error("matrix is singular; cannot invert")]
    SingularMatrix,
}

// ----------------------------------------------------------------------
// GF(256) arithmetic — using the primitive polynomial 0x11d (Rijndael's).
// ----------------------------------------------------------------------

const GF_POLY: u16 = 0x11d;

struct Gf256Tables {
    exp: [u8; 512],
    log: [u8; 256],
}

fn build_tables() -> Gf256Tables {
    let mut exp = [0u8; 512];
    let mut log = [0u8; 256];
    let mut x: u16 = 1;
    for i in 0..255 {
        exp[i] = x as u8;
        log[x as usize] = i as u8;
        x <<= 1;
        if x & 0x100 != 0 {
            x ^= GF_POLY;
        }
    }
    // Wrap-around table for products.
    for i in 255..512 {
        exp[i] = exp[i - 255];
    }
    Gf256Tables { exp, log }
}

fn gf_mul(t: &Gf256Tables, a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    t.exp[(t.log[a as usize] as usize + t.log[b as usize] as usize) % 255]
}

#[allow(dead_code)]
fn gf_div(t: &Gf256Tables, a: u8, b: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let la = t.log[a as usize] as isize;
    let lb = t.log[b as usize] as isize;
    let diff = (la - lb + 255) % 255;
    t.exp[diff as usize]
}

fn gf_inv(t: &Gf256Tables, a: u8) -> u8 {
    // a^-1 = a^254
    let la = t.log[a as usize] as isize;
    t.exp[((255 - la) % 255) as usize]
}

/// Build the systematic Vandermonde matrix for `(k, m)`. The matrix is
/// `(k+m) × k` in row-major form. Top k rows are the identity (so the
/// first k output shards are the original data); bottom m rows are
/// `[i^(j) for j in 0..k]` with i starting at k+1 (avoiding division
/// by 0 in inverse below).
fn vandermonde_matrix(t: &Gf256Tables, k: u8, m: u8) -> Vec<Vec<u8>> {
    let n = (k + m) as usize;
    let mut mtx = vec![vec![0u8; k as usize]; n];
    // Top-k rows: identity.
    for i in 0..k as usize {
        mtx[i][i] = 1;
    }
    // Bottom-m rows: Vandermonde over GF(256). Row i uses base = i+1.
    for i in 0..m as usize {
        let base = (k as u8) + 1 + (i as u8);
        let mut value: u8 = 1;
        for j in 0..k as usize {
            mtx[k as usize + i][j] = value;
            value = gf_mul(t, value, base);
        }
    }
    mtx
}

/// Encode a payload into a `(k, m)` FEC group. Payload is split into k
/// equal-length data shards (zero-padded if needed); m parity shards
/// are computed via the Vandermonde generator matrix.
pub fn encode_fec_group(payload: &[u8], k: u8, m: u8, group_id: u64) -> Result<FecGroup, FecError> {
    if payload.len() > FEC_MAX_PAYLOAD_BYTES {
        return Err(FecError::PayloadTooLarge(payload.len()));
    }
    if k == 0 || k > 8 || m == 0 || m > 8 || (k + m) > 16 {
        return Err(FecError::InvalidKM { k, m });
    }
    let tables = build_tables();
    let shard_len = payload.len().div_ceil(k as usize).max(1);
    let mut data_shards = Vec::with_capacity(k as usize);
    for i in 0..k as usize {
        let start = i * shard_len;
        let end = (start + shard_len).min(payload.len());
        let mut shard = vec![0u8; shard_len];
        if start < end {
            shard[..end - start].copy_from_slice(&payload[start..end]);
        }
        data_shards.push(shard);
    }
    // Parity shards: parity[i][j] = SUM(generator[k+i][col] * data_shards[col][j])
    let mtx = vandermonde_matrix(&tables, k, m);
    let mut parity_shards = vec![vec![0u8; shard_len]; m as usize];
    for i in 0..m as usize {
        for col in 0..k as usize {
            let coeff = mtx[k as usize + i][col];
            for j in 0..shard_len {
                parity_shards[i][j] ^= gf_mul(&tables, coeff, data_shards[col][j]);
            }
        }
    }
    let mut shards = Vec::with_capacity((k + m) as usize);
    for (idx, sh) in data_shards.into_iter().enumerate() {
        shards.push(FecShard {
            group_id,
            shard_index: idx as u8,
            k,
            m,
            data: sh,
        });
    }
    for (idx, sh) in parity_shards.into_iter().enumerate() {
        shards.push(FecShard {
            group_id,
            shard_index: (k as usize + idx) as u8,
            k,
            m,
            data: sh,
        });
    }
    Ok(FecGroup {
        group_id,
        k,
        m,
        shard_len,
        original_len: payload.len(),
        shards,
    })
}

/// Decode a FEC group from a set of surviving shards. Any k of the
/// (k+m) shards must be present. Returns the original payload bytes.
pub fn decode_fec_group(
    surviving: &[FecShard],
    k: u8,
    m: u8,
    original_len: usize,
) -> Result<Vec<u8>, FecError> {
    if k == 0 || k > 8 || m == 0 || m > 8 || (k + m) > 16 {
        return Err(FecError::InvalidKM { k, m });
    }
    if surviving.len() < k as usize {
        return Err(FecError::TooFewShards {
            have: surviving.len(),
            need: k as usize,
        });
    }
    let shard_len = surviving[0].data.len();
    for s in surviving.iter().take(k as usize) {
        if s.data.len() != shard_len {
            return Err(FecError::ShardLenMismatch {
                got: s.data.len(),
                expected: shard_len,
            });
        }
    }
    let tables = build_tables();
    let mtx = vandermonde_matrix(&tables, k, m);
    // Pick the first k surviving shards. Build a k×k submatrix from
    // their generator-matrix rows, plus the corresponding right-hand-side
    // (the shard data).
    let mut sub: Vec<Vec<u8>> = Vec::with_capacity(k as usize);
    let mut rhs: Vec<Vec<u8>> = Vec::with_capacity(k as usize);
    for s in surviving.iter().take(k as usize) {
        sub.push(mtx[s.shard_index as usize].clone());
        rhs.push(s.data.clone());
    }
    // Gauss-Jordan eliminate `sub` to identity; apply same row ops to rhs.
    let n = k as usize;
    for col in 0..n {
        // Find pivot.
        let mut pivot_row = None;
        for row in col..n {
            if sub[row][col] != 0 {
                pivot_row = Some(row);
                break;
            }
        }
        let pivot_row = pivot_row.ok_or(FecError::SingularMatrix)?;
        if pivot_row != col {
            sub.swap(pivot_row, col);
            rhs.swap(pivot_row, col);
        }
        // Normalize pivot row.
        let pivot_val = sub[col][col];
        let inv = gf_inv(&tables, pivot_val);
        for j in 0..n {
            sub[col][j] = gf_mul(&tables, sub[col][j], inv);
        }
        for j in 0..shard_len {
            rhs[col][j] = gf_mul(&tables, rhs[col][j], inv);
        }
        // Eliminate column from all other rows.
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = sub[row][col];
            if factor == 0 {
                continue;
            }
            for j in 0..n {
                sub[row][j] ^= gf_mul(&tables, factor, sub[col][j]);
            }
            for j in 0..shard_len {
                rhs[row][j] ^= gf_mul(&tables, factor, rhs[col][j]);
            }
        }
    }
    // rhs now contains the k data shards in order 0..k.
    let mut out = Vec::with_capacity(shard_len * n);
    for row in rhs.iter().take(n) {
        out.extend_from_slice(row);
    }
    out.truncate(original_len);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_no_loss() {
        let payload = b"M8B FEC test payload!".to_vec();
        let group = encode_fec_group(&payload, 4, 2, 99).unwrap();
        assert_eq!(group.shards.len(), 6);
        // Decode using only the first 4 (data) shards.
        let surviving = group.shards.iter().take(4).cloned().collect::<Vec<_>>();
        let decoded = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn round_trip_one_shard_lost() {
        let payload = b"M8B Acceptance Reed-Solomon FEC k=4 m=2 single corruption test".to_vec();
        let group = encode_fec_group(&payload, 4, 2, 100).unwrap();
        // Erase data shard 1 (so we have shards 0, 2, 3, parity_0, parity_1 = 5 surviving).
        let surviving: Vec<FecShard> = group
            .shards
            .iter()
            .filter(|s| s.shard_index != 1)
            .cloned()
            .collect();
        let decoded = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn round_trip_two_shards_lost() {
        let payload = vec![0xABu8; 64];
        let group = encode_fec_group(&payload, 4, 2, 101).unwrap();
        // Erase data shard 0 AND data shard 2.
        let surviving: Vec<FecShard> = group
            .shards
            .iter()
            .filter(|s| s.shard_index != 0 && s.shard_index != 2)
            .cloned()
            .collect();
        assert_eq!(surviving.len(), 4);
        let decoded = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn round_trip_parity_lost() {
        let payload = vec![0x77u8; 128];
        let group = encode_fec_group(&payload, 4, 2, 102).unwrap();
        // Erase both parity shards.
        let surviving: Vec<FecShard> = group
            .shards
            .iter()
            .filter(|s| s.shard_index < 4)
            .cloned()
            .collect();
        let decoded = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn too_few_surviving_shards_errors() {
        let payload = vec![0u8; 32];
        let group = encode_fec_group(&payload, 4, 2, 103).unwrap();
        let surviving: Vec<FecShard> = group.shards.iter().take(3).cloned().collect();
        let err = decode_fec_group(&surviving, 4, 2, payload.len()).unwrap_err();
        assert!(matches!(err, FecError::TooFewShards { .. }));
    }

    #[test]
    fn payload_too_large_errors() {
        let payload = vec![0u8; FEC_MAX_PAYLOAD_BYTES + 1];
        let err = encode_fec_group(&payload, 4, 2, 104).unwrap_err();
        assert!(matches!(err, FecError::PayloadTooLarge(_)));
    }

    #[test]
    fn invalid_km_rejected() {
        let err = encode_fec_group(&[], 0, 2, 0).unwrap_err();
        assert!(matches!(err, FecError::InvalidKM { .. }));
        let err = encode_fec_group(&[], 4, 9, 0).unwrap_err();
        assert!(matches!(err, FecError::InvalidKM { .. }));
        let err = encode_fec_group(&[], 10, 8, 0).unwrap_err();
        assert!(matches!(err, FecError::InvalidKM { .. }));
    }
}
