//! M8B § Integration tests for semver negotiation + downgrade-attack
//! detection.
//!
//! Maps to spec § Acceptance criteria scenarios:
//! - "Semver negotiation accepts a minor-newer server"
//! - "Semver negotiation rejects a major-mismatched client"
//! - "Protocol downgrade attack is rejected"

use cf_net::protocol::semver::{detect_downgrade_attack, negotiate, NegotiationOutcome, Semver};

#[test]
fn minor_newer_server_accepted_at_client_subset() {
    // Client 0.1.4, Server 0.1.7 → session at v0.1.4 features.
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
            assert_eq!(session_semver.patch, 4, "session settles at lower patch");
            assert!(granted_features.contains(&"redundant_input".to_string()));
            assert!(granted_features.contains(&"fec".to_string()));
            assert!(granted_features.contains(&"ice_lite".to_string()));
            assert!(
                !granted_features.contains(&"spectator_v2".to_string()),
                "feature exclusive to the server is not granted"
            );
        }
        other => panic!("expected Accepted, got {other:?}"),
    }
}

#[test]
fn major_mismatched_client_rejected() {
    // Server 0.1.7, Client 0.2.0 → reject with protocol_major_mismatch.
    let server = Semver::new(0, 1, 7);
    let client = Semver::new(0, 2, 0);
    let outcome = negotiate(
        server,
        &["fec"],
        client,
        &["fec"],
        "https://corefall.example/update",
    );
    match outcome.clone() {
        NegotiationOutcome::RejectedMajorMismatch {
            server: s,
            client: c,
            download_url,
        } => {
            // Per spec scenario: server: 0x0107, client: 0x0200.
            assert_eq!(s.pack(), 0x0107);
            assert_eq!(c.pack(), 0x0200);
            assert!(download_url.contains("corefall.example/update"));
        }
        other => panic!("expected RejectedMajorMismatch, got {other:?}"),
    }

    // **M8B § spec literal**: "Then the server responds with
    // NetError::ProtocolVersionMismatch { server: 0x0107, client: 0x0200 }".
    let net_err = outcome.to_net_error().expect("major mismatch yields NetError");
    match net_err {
        cf_net::NetError::ProtocolVersionMismatch { server: s, client: c } => {
            assert_eq!(s, 0x0107);
            assert_eq!(c, 0x0200);
        }
        other => panic!("expected ProtocolVersionMismatch, got {other:?}"),
    }
}

#[test]
fn downgrade_attack_detected_on_handshake_mismatch() {
    // **M8B § Acceptance "Protocol downgrade attack is rejected"**:
    // TLS-bound layer says 0.1.4; application Handshake says 0.0.1
    // (an attempted downgrade). detect_downgrade_attack returns an
    // error; the From<DowngradeAttackError> for NetError implementation
    // converts it to the spec-literal NetError::Transport("tls
    // handshake mismatch").
    let tls = Semver::new(0, 1, 4).pack();
    let app = Semver::new(0, 0, 1).pack();
    let err = detect_downgrade_attack(tls, app).unwrap_err();
    assert_eq!(err.tls_advertised, Semver::new(0, 1, 4));
    assert_eq!(err.application_advertised, Semver::new(0, 0, 1));

    // Per spec: "And the session is closed with NetError::Transport(\"tls
    // handshake mismatch\")"
    let net_err: cf_net::NetError = err.into();
    match net_err {
        cf_net::NetError::Transport(reason) => assert_eq!(reason, "tls handshake mismatch"),
        other => panic!("expected NetError::Transport(\"tls handshake mismatch\"), got {other:?}"),
    }
}

#[test]
fn tls_handshake_mismatch_helper_returns_spec_literal() {
    let err = cf_net::tls_handshake_mismatch_error();
    match err {
        cf_net::NetError::Transport(reason) => assert_eq!(reason, "tls handshake mismatch"),
        other => panic!("expected Transport variant, got {other:?}"),
    }
}

#[test]
fn matching_semvers_accepted() {
    let v = Semver::new(0, 1, 4);
    let outcome = negotiate(
        v,
        &["fec", "ice_lite"],
        v,
        &["fec", "ice_lite"],
        "https://corefall.example/update",
    );
    match outcome {
        NegotiationOutcome::Accepted {
            session_semver,
            granted_features,
        } => {
            assert_eq!(session_semver, v);
            assert_eq!(granted_features, vec!["fec".to_string(), "ice_lite".to_string()]);
        }
        other => panic!("expected Accepted, got {other:?}"),
    }
}
