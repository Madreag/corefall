//! M8B § Byte-pinning tests — verify the v0.1 frame encoder produces the
//! same byte stream as the canonical fixture vectors. Any byte that
//! changes here MUST bump `PROTOCOL_SEMVER` minor and add a new fixture;
//! the byte-pin CI gate (game/tools/ci/m8b_protocol_byte_pin.sh) fails
//! any patch that flips a single byte without that bump.
//!
//! Fixtures live under
//! `game/content/net/protocol/frame_v01_fixtures.json` and are loaded at
//! test time via `include_str!`. Each entry maps a payload variant name
//! (snake_case) → `{ payload_hex, frame_hex, semver_packed, seq,
//! timestamp_ms }`.

#![cfg(test)]

use serde::Deserialize;

use crate::protocol::frame_v01::{encode_frame, encode_payload_to_vec, NetFrameV01, NetPayloadV01};

const FIXTURES_JSON: &str = include_str!("../../../../content/net/protocol/frame_v01_fixtures.json");

#[derive(Debug, Deserialize)]
struct FixtureFile {
    pub fixtures: Vec<Fixture>,
}

#[derive(Debug, Deserialize)]
struct Fixture {
    pub variant: String,
    pub payload_hex: String,
    pub frame_hex: String,
    pub semver_packed: u16,
    pub seq: u32,
    pub timestamp_ms: u64,
    #[serde(default)]
    pub params: serde_json::Value,
}

fn build_payload(variant: &str, params: &serde_json::Value) -> NetPayloadV01 {
    use crate::loss_recovery::redundant_input::{RedundantInputEntry, RedundantInputTail};
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
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        },
        "handshake_ack" => NetPayloadV01::HandshakeAck {
            accepted: params["accepted"].as_bool().unwrap_or(false),
            granted_features: params["granted_features"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
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

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

mod tests {
    use super::*;

    #[test]
    fn every_payload_variant_has_a_fixture() {
        let file: FixtureFile = serde_json::from_str(FIXTURES_JSON).expect("fixtures parse");
        let need = [
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
        for v in &need {
            assert!(
                file.fixtures.iter().any(|f| f.variant == *v),
                "missing fixture for variant {v}"
            );
        }
    }

    #[test]
    fn each_fixture_byte_pinned_against_encoder_output() {
        let file: FixtureFile = serde_json::from_str(FIXTURES_JSON).expect("fixtures parse");
        for f in &file.fixtures {
            let payload = build_payload(&f.variant, &f.params);
            let payload_bytes = encode_payload_to_vec(&payload);
            assert_eq!(
                hex_encode(&payload_bytes),
                f.payload_hex,
                "payload_hex byte-pin drift for variant {}: encoder output diverged from fixture",
                f.variant
            );
            let frame = NetFrameV01 {
                semver_packed: f.semver_packed,
                seq: f.seq,
                timestamp_ms: f.timestamp_ms,
                payload,
            };
            let frame_bytes = encode_frame(&frame).expect("frame fits within max size");
            assert_eq!(
                hex_encode(&frame_bytes),
                f.frame_hex,
                "frame_hex byte-pin drift for variant {}: encoder output diverged from fixture",
                f.variant
            );
        }
    }

    /// the minimal InputCommand payload alone MUST be ≤ 96 bytes.
    #[test]
    fn input_command_minimal_payload_is_within_96_byte_budget() {
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
            "minimal input_command payload {} bytes exceeds 96-byte budget",
            bytes.len()
        );
    }
}
