//! M8B § Protocol semver — major / minor / patch negotiation handshake.
//!
//! Wire-shape: PROTOCOL_SEMVER is packed into a u16 as
//! `(minor << 8) | patch`. Major is implicit (`0` during the v0.x phase);
//! a major bump moves the high byte's interpretation. This matches the
//! spec's literal values: `0.1.7` → `0x0107`, `0.2.0` → `0x0200`.
//!
//! Compatibility rules (cf-net's v0.x semantics):
//!
//! - `client.major != server.major` OR `client.minor != server.minor`
//!   → reject with `protocol_major_mismatch`. The client surfaces a
//!   download-update prompt with the server's exposed download URL.
//!   (During v0.x the `minor` field is the major-compat axis per spec
//!   scenario "Semver negotiation rejects a major-mismatched client":
//!   v0.1.7 vs v0.2.0 mismatch.)
//! - Equal `major` + `minor`, differing `patch` → accept; session uses
//!   the LOWER patch's feature subset (server downscopes for older
//!   clients; this matches the spec's "minor-newer server" scenario where
//!   v0.1.4 client + v0.1.7 server settle at v0.1.4 features).
//!
//! Server attempts to honor the LOWER of the two semvers' feature set —
//! the intersection of features both sides advertise.

use serde::{Deserialize, Serialize};

/// **M8B § locked**: cf-net protocol semver as a 3-tuple. Bumped per
/// the byte-pin CI gate's rules.
///
/// - **0.1.0** (M8A baseline): NetFrame + NetPayload locked, JSON wire.
/// - **0.1.4** (M8B): byte-pinned v0.1 layout + semver gate + redundant
///   input + FEC + NAT traversal + rollback window event surface.
pub const PROTOCOL_SEMVER: Semver = Semver {
    major: 0,
    minor: 1,
    patch: 4,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Semver {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl Semver {
    pub const fn new(major: u8, minor: u8, patch: u8) -> Self {
        Self { major, minor, patch }
    }

    /// Pack into a u16 for wire encoding: `(minor << 8) | patch`.
    /// Mirrors the spec's `0x0107` representation for 0.1.7 + `0x0200`
    /// for 0.2.0. During the v0.x phase the `major` field is implicit
    /// (always 0) and is not folded into the wire bytes; a major bump
    /// would re-encode via a future protocol-version field outside this
    /// u16.
    pub const fn pack(self) -> u16 {
        pack(self.major, self.minor, self.patch)
    }

    /// Inverse of `pack`. `major` is reconstructed as 0 during the v0.x
    /// phase — the wire packed form does not encode it.
    pub const fn unpack(packed: u16) -> Self {
        Self {
            major: 0,
            minor: ((packed >> 8) & 0xFF) as u8,
            patch: (packed & 0xFF) as u8,
        }
    }

    pub fn as_string(self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Standalone `pack` helper for use in const contexts. The packed
/// form is `(minor << 8) | patch`. `major` is consumed for symmetry
/// but doesn't enter the u16 during v0.x.
pub const fn pack(major: u8, minor: u8, patch: u8) -> u16 {
    let _ = major;
    ((minor as u16) << 8) | (patch as u16)
}

/// Outcome of `negotiate`. The "granted features" list is the intersection
/// of the two sides' advertised features; the session is constrained to
/// that intersection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NegotiationOutcome {
    /// Accepted. `session_semver` is the lower of the two minor versions.
    /// Patch is taken from the server (servers typically run newer
    /// patch).
    Accepted {
        session_semver: Semver,
        granted_features: Vec<String>,
    },
    /// Major version mismatch — non-recoverable. The client should be
    /// prompted to download an update.
    RejectedMajorMismatch {
        server: Semver,
        client: Semver,
        download_url: String,
    },
}

/// Run the M8B semver-handshake on the SERVER side. The server is at
/// `server_semver` + advertises `server_features`; the client sent
/// `client_semver` + `client_features`.
///
/// Returns [`NegotiationOutcome`] without panicking. Callers wrap this
/// in a [`crate::protocol::frame_v01::NetPayloadV01::HandshakeAck`].
///
/// **v0.x compatibility rule**: during the v0.x phase, the `minor`
/// field is the major-compat axis (per spec scenario "v0.1.7 vs v0.2.0
/// = major-mismatched"). Patch-level differences are accepted; the
/// session settles at the LOWER patch's feature subset (so a v0.1.4
/// client + v0.1.7 server runs at v0.1.4 features per the spec).
pub fn negotiate(
    server_semver: Semver,
    server_features: &[&str],
    client_semver: Semver,
    client_features: &[&str],
    download_url: &str,
) -> NegotiationOutcome {
    if server_semver.major != client_semver.major || server_semver.minor != client_semver.minor {
        return NegotiationOutcome::RejectedMajorMismatch {
            server: server_semver,
            client: client_semver,
            download_url: download_url.to_string(),
        };
    }
    // Same major + same minor → patch differs. Session settles at the
    // LOWER patch (client downscopes if server is newer).
    let session_patch = server_semver.patch.min(client_semver.patch);
    let session_semver = Semver {
        major: server_semver.major,
        minor: server_semver.minor,
        patch: session_patch,
    };
    let mut granted: Vec<String> = server_features
        .iter()
        .filter(|f| client_features.contains(f))
        .map(|s| (*s).to_string())
        .collect();
    granted.sort();
    granted.dedup();
    NegotiationOutcome::Accepted {
        session_semver,
        granted_features: granted,
    }
}

/// **M8B § Acceptance "Protocol downgrade attack is rejected"** — when
/// the client's advertised semver in the application-layer Handshake
/// differs from the value embedded in the QUIC TLS-bound transport
/// parameters, the session MUST close with a TLS handshake mismatch
/// error. This helper exposes that check as a pure function so the
/// transport layer + integration tests can both call it.
pub fn detect_downgrade_attack(
    tls_advertised_semver_packed: u16,
    application_advertised_semver_packed: u16,
) -> Result<(), DowngradeAttackError> {
    if tls_advertised_semver_packed != application_advertised_semver_packed {
        return Err(DowngradeAttackError {
            tls_advertised: Semver::unpack(tls_advertised_semver_packed),
            application_advertised: Semver::unpack(application_advertised_semver_packed),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("tls handshake mismatch: tls={} app={}", tls_advertised.as_string(), application_advertised.as_string())]
pub struct DowngradeAttackError {
    pub tls_advertised: Semver,
    pub application_advertised: Semver,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unpack_round_trips_for_v0_x() {
        // During v0.x the `major` field is implicit 0 and does not
        // enter the packed wire form. The pack/unpack pair preserves
        // (minor, patch) byte-for-byte.
        for minor in 0..=0xFFu8 {
            for patch in 0..=0xFFu8 {
                let v = Semver { major: 0, minor, patch };
                assert_eq!(Semver::unpack(v.pack()), v);
            }
        }
    }

    #[test]
    fn locked_protocol_semver_is_0_1_4() {
        assert_eq!(PROTOCOL_SEMVER, Semver::new(0, 1, 4));
        assert_eq!(PROTOCOL_SEMVER.pack(), 0x0104);
    }

    #[test]
    fn pack_0_1_7_is_0x0107() {
        assert_eq!(pack(0, 1, 7), 0x0107);
    }

    #[test]
    fn pack_0_2_0_is_0x0200() {
        assert_eq!(pack(0, 2, 0), 0x0200);
    }

    #[test]
    fn minor_newer_server_accepted_at_client_minor() {
        let server = Semver::new(0, 1, 7);
        let client = Semver::new(0, 1, 4);
        let outcome = negotiate(
            server,
            &["redundant_input", "fec", "ice_lite", "spectator_v2"],
            client,
            &["redundant_input", "fec", "ice_lite"],
            "https://corefall.example/update",
        );
        match outcome {
            NegotiationOutcome::Accepted {
                session_semver,
                granted_features,
            } => {
                assert_eq!(session_semver.patch, 4);
                assert!(granted_features.contains(&"redundant_input".to_string()));
                assert!(granted_features.contains(&"fec".to_string()));
                assert!(granted_features.contains(&"ice_lite".to_string()));
                assert!(!granted_features.contains(&"spectator_v2".to_string()));
            }
            other => panic!("expected Accepted, got {other:?}"),
        }
    }

    #[test]
    fn major_mismatch_rejected() {
        let server = Semver::new(0, 1, 7);
        let client = Semver::new(0, 2, 0);
        let outcome = negotiate(
            server,
            &["fec"],
            client,
            &["fec"],
            "https://corefall.example/update",
        );
        match outcome {
            NegotiationOutcome::RejectedMajorMismatch {
                server: s,
                client: c,
                download_url,
            } => {
                assert_eq!(s, server);
                assert_eq!(c, client);
                assert!(download_url.contains("corefall.example"));
            }
            other => panic!("expected RejectedMajorMismatch, got {other:?}"),
        }
    }

    #[test]
    fn downgrade_attack_detected() {
        // TLS-bound semver claims 0.1.4; application Handshake claims 0.0.1
        // (an attempted downgrade). detect_downgrade_attack returns the
        // mismatch + both semvers for the error path.
        let err = detect_downgrade_attack(pack(0, 1, 4), pack(0, 0, 1)).unwrap_err();
        assert_eq!(err.tls_advertised, Semver::new(0, 1, 4));
        assert_eq!(err.application_advertised, Semver::new(0, 0, 1));
    }

    #[test]
    fn matching_semvers_pass_downgrade_check() {
        assert!(detect_downgrade_attack(pack(0, 1, 4), pack(0, 1, 4)).is_ok());
    }
}
