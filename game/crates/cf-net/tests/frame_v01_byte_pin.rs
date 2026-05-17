//! M8B § Integration test — verify the v0.1 frame encoder produces
//! byte-identical output to the locked fixture vectors.
//!
//! This test is the public-API form of the inner
//! `protocol::byte_pinning_tests` module; it loads the canonical
//! fixtures file and re-runs the encoder + checks the resulting bytes.
//! The corresponding CI gate is
//! `game/tools/ci/m8b_protocol_byte_pin.sh`.

use cf_net::loss_recovery::redundant_input::{RedundantInputEntry, RedundantInputTail};
use cf_net::protocol::frame_v01::{encode_frame, encode_payload_to_vec, NetFrameV01, NetPayloadV01};
use serde::Deserialize;

const FIXTURES_JSON: &str = include_str!("../../../content/net/protocol/frame_v01_fixtures.json");

#[derive(Debug, Deserialize)]
struct FixtureFile {
    fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    variant: String,
    payload_hex: String,
    frame_hex: String,
    semver_packed: u16,
    seq: u32,
    timestamp_ms: u64,
    #[serde(default)]
    params: serde_json::Value,
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn build_payload(variant: &str, params: &serde_json::Value) -> NetPayloadV01 {
    match variant {
        "ping" => NetPayloadV01::Ping {
            send_ms: params["send_ms"].as_u64().unwrap_or(0),
        },
        "pong" => NetPayloadV01::Pong {
            send_ms: params["send_ms"].as_u64().unwrap_or(0),
            recv_ms: params["recv_ms"].as_u64().unwrap_or(0),
        },
        "disconnect" => NetPayloadV01::Disconnect {
            reason: params["reason"].as_str().unwrap_or("").to_string(),
        },
        "handshake" => NetPayloadV01::Handshake {
            semver_packed: params["semver_packed"].as_u64().unwrap_or(0) as u16,
            client_version: params["client_version"].as_str().unwrap_or("").to_string(),
            content_hash: params["content_hash"].as_str().unwrap_or("").to_string(),
            supported_features: params["supported_features"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        "handshake_ack" => NetPayloadV01::HandshakeAck {
            accepted: params["accepted"].as_bool().unwrap_or(false),
            granted_features: params["granted_features"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
                .unwrap_or_default(),
            session_id: params["session_id"].as_str().unwrap_or("").to_string(),
            server_semver_packed: params["server_semver_packed"].as_u64().unwrap_or(0) as u16,
            reject_reason: params["reject_reason"].as_str().unwrap_or("").to_string(),
            download_url: params["download_url"].as_str().unwrap_or("").to_string(),
        },
        "input_command" => NetPayloadV01::InputCommand {
            tick: params["tick"].as_u64().unwrap_or(0),
            intent_event_id: params["intent_event_id"].as_str().unwrap_or("").to_string(),
            control_command_bytes: params["control_command_bytes"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        "snapshot_delta" => NetPayloadV01::SnapshotDelta {
            from_tick: params["from_tick"].as_u64().unwrap_or(0),
            to_tick: params["to_tick"].as_u64().unwrap_or(0),
            delta_bytes: params["delta_bytes"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        "event_batch" => NetPayloadV01::EventBatch {
            tick: params["tick"].as_u64().unwrap_or(0),
            events_bytes: params["events_bytes"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        "checksum_probe" => {
            let bytes: Vec<u8> = params["checksum_bytes"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<_>>())
                .unwrap_or_default();
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            NetPayloadV01::ChecksumProbe {
                tick: params["tick"].as_u64().unwrap_or(0),
                checksum_bytes: arr,
            }
        }
        "input_command_redundant" => NetPayloadV01::InputCommandRedundant {
            head_tick: params["head_tick"].as_u64().unwrap_or(0),
            tail: RedundantInputTail {
                window_ticks: params["window_ticks"].as_u64().unwrap_or(3) as u8,
                entries: params["entries"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|v| RedundantInputEntry {
                                tick: v["tick"].as_u64().unwrap_or(0),
                                intent_bytes: v["intent_bytes"]
                                    .as_array()
                                    .map(|x| x.iter().filter_map(|y| y.as_u64().map(|n| n as u8)).collect())
                                    .unwrap_or_default(),
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            },
        },
        "fec_shard" => NetPayloadV01::FecShard {
            group_id: params["group_id"].as_u64().unwrap_or(0),
            shard_index: params["shard_index"].as_u64().unwrap_or(0) as u8,
            k: params["k"].as_u64().unwrap_or(4) as u8,
            m: params["m"].as_u64().unwrap_or(2) as u8,
            shard_bytes: params["shard_bytes"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect::<Vec<_>>())
                .unwrap_or_default(),
        },
        "nat_traversal_outcome" => NetPayloadV01::NatTraversalOutcome {
            session_id: params["session_id"].as_str().unwrap_or("").to_string(),
            method: params["method"].as_str().unwrap_or("").to_string(),
            path: params["path"].as_str().unwrap_or("").to_string(),
            elapsed_ms: params["elapsed_ms"].as_u64().unwrap_or(0) as u32,
        },
        "rollback_window" => NetPayloadV01::RollbackWindow {
            from_tick: params["from_tick"].as_u64().unwrap_or(0),
            to_tick: params["to_tick"].as_u64().unwrap_or(0),
            resim_us: params["resim_us"].as_u64().unwrap_or(0) as u32,
            cause: params["cause"].as_str().unwrap_or("").to_string(),
        },
        other => panic!("fixture variant {other} not handled"),
    }
}

#[test]
fn fixtures_byte_pin_all_payload_variants() {
    let file: FixtureFile = serde_json::from_str(FIXTURES_JSON).expect("fixtures parse");
    let required_variants = [
        "handshake",
        "handshake_ack",
        "input_command",
        "snapshot_delta",
        "event_batch",
        "checksum_probe",
        "ping",
        "pong",
        "disconnect",
        "input_command_redundant",
        "fec_shard",
        "nat_traversal_outcome",
        "rollback_window",
    ];
    for v in &required_variants {
        assert!(
            file.fixtures.iter().any(|f| f.variant == *v),
            "missing fixture for variant {v}"
        );
    }
    for f in &file.fixtures {
        let payload = build_payload(&f.variant, &f.params);
        let pb = encode_payload_to_vec(&payload);
        assert_eq!(hex(&pb), f.payload_hex, "payload_hex drift for {}", f.variant);
        let frame = NetFrameV01 {
            semver_packed: f.semver_packed,
            seq: f.seq,
            timestamp_ms: f.timestamp_ms,
            payload,
        };
        let fb = encode_frame(&frame).expect("frame fits");
        assert_eq!(hex(&fb), f.frame_hex, "frame_hex drift for {}", f.variant);
    }
}

/// **M8B § Acceptance "Unreliable datagram carries per-tick input"** —
/// minimal InputCommand payload alone is ≤ 96 bytes.
#[test]
fn input_command_payload_within_96_bytes() {
    let file: FixtureFile = serde_json::from_str(FIXTURES_JSON).expect("fixtures parse");
    let ic = file
        .fixtures
        .iter()
        .find(|f| f.variant == "input_command")
        .expect("input_command fixture present");
    let payload = build_payload(&ic.variant, &ic.params);
    let bytes = encode_payload_to_vec(&payload);
    assert!(
        bytes.len() <= 96,
        "minimal InputCommand payload {} bytes exceeds 96-byte budget",
        bytes.len()
    );
}

/// **M8B § Notes "Any change that flips a single byte in a v0.1 fixture
/// vector MUST bump PROTOCOL_SEMVER minor"**: the encoder embeds the
/// configured semver in every frame's prefix. The fixture's
/// semver_packed value pins the value the CI gate verifies.
#[test]
fn fixture_semver_matches_protocol_semver() {
    let file: FixtureFile = serde_json::from_str(FIXTURES_JSON).expect("fixtures parse");
    let ping = file.fixtures.iter().find(|f| f.variant == "ping").unwrap();
    assert_eq!(
        ping.semver_packed,
        cf_net::PROTOCOL_SEMVER.pack(),
        "fixture semver_packed must match PROTOCOL_SEMVER (currently {:#06x}); regen fixtures + bump semver",
        cf_net::PROTOCOL_SEMVER.pack()
    );
}
