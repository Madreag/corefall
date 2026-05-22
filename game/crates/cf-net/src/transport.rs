//! M8A § Network transport — QUIC + WebSocket fallback scaffolds.
//!
//! M8A ships the protocol shape + per-frame max size enforcement. The
//! actual QUIC transport (`quinn`) is wired at M9+ when the cf-net
//! server binary is provisioned on the reference platform; until then,
//! cf-headless serve uses an in-process loopback transport for local
//! determinism testing.

use crate::protocol::{NetFrame, NET_FRAME_MAX_SIZE_BYTES};

/// size is within the locked 1450-byte per-frame max.
pub fn encode_frame(frame: &NetFrame) -> crate::NetResult<Vec<u8>> {
    let bytes = serde_json::to_vec(frame).map_err(|e| crate::NetError::Transport(format!("encode: {e}")))?;
    if bytes.len() > NET_FRAME_MAX_SIZE_BYTES {
        return Err(crate::NetError::FrameTooLarge(bytes.len()));
    }
    Ok(bytes)
}

/// protocol-version mismatch before any sim frame.
pub fn decode_frame(bytes: &[u8]) -> crate::NetResult<NetFrame> {
    if bytes.len() > NET_FRAME_MAX_SIZE_BYTES {
        return Err(crate::NetError::FrameTooLarge(bytes.len()));
    }
    let frame: NetFrame =
        serde_json::from_slice(bytes).map_err(|e| crate::NetError::Deserialize(format!("decode: {e}")))?;
    crate::protocol::verify_handshake_version(frame.version)?;
    Ok(frame)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::NetPayload;

    #[test]
    fn small_frame_round_trips() {
        let frame = NetFrame::new(1, 0, NetPayload::Ping { send_ms: 7 });
        let bytes = encode_frame(&frame).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(frame, back);
    }

    #[test]
    fn decode_rejects_oversized_frame() {
        let oversized = vec![0u8; NET_FRAME_MAX_SIZE_BYTES + 1];
        let err = decode_frame(&oversized).unwrap_err();
        assert!(matches!(err, crate::NetError::FrameTooLarge(_)));
    }

    #[test]
    fn decode_rejects_mismatched_version() {
        let mut frame = NetFrame::new(1, 0, NetPayload::Ping { send_ms: 7 });
        frame.version = 99;
        let bytes = serde_json::to_vec(&frame).unwrap();
        let err = decode_frame(&bytes).unwrap_err();
        assert!(matches!(err, crate::NetError::ProtocolVersionMismatch { .. }));
    }
}
