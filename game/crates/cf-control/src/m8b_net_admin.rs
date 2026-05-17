//! M8B § cf-control net admin/observe surface.
//!
//! Per M8B spec § Crates / modules touched — `cf-control` MODIFY:
//! "New cfctl: `observe.net.session_transport`, `observe.net.rollback_stats`,
//! `observe.net.loss_recovery`, `admin.net.force_relay`."
//!
//! M8B ships the WIRE CONTRACT for these methods: param types + return
//! shape + schema dump entries. Production wiring of the live engine
//! state lives in cf-control's engine + server.rs at M9+. The internal
//! tests in this module verify the param shape + default-value behavior
//! the schema dump pinning depends on.

use cf_net::nat::{NatTraversalMethod, NatTraversalPath};
use cf_net::transport_select::TransportMode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// `observe.net.session_transport` — projection of the currently
/// negotiated transport mode + NAT traversal outcome. Empty when no
/// session is active.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObserveNetSessionTransportParams {
    pub schema_version: u32,
}

/// `observe.net.session_transport` — return shape.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetSessionTransportView {
    pub schema_version: u32,
    pub session_id: String,
    pub transport_mode: String,
    pub traversal_method: String,
    pub traversal_path: String,
    pub elapsed_ms: u32,
    pub session_semver_packed: u16,
}

impl NetSessionTransportView {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            session_id: String::new(),
            transport_mode: String::new(),
            traversal_method: String::new(),
            traversal_path: String::new(),
            elapsed_ms: 0,
            session_semver_packed: 0,
        }
    }

    pub fn from_parts(
        session_id: &str,
        transport: TransportMode,
        method: NatTraversalMethod,
        path: NatTraversalPath,
        elapsed_ms: u32,
        session_semver_packed: u16,
    ) -> Self {
        Self {
            schema_version: 1,
            session_id: session_id.to_string(),
            transport_mode: transport.as_str().to_string(),
            traversal_method: method.as_str().to_string(),
            traversal_path: path.as_str().to_string(),
            elapsed_ms,
            session_semver_packed,
        }
    }
}

/// `observe.net.rollback_stats` — projection of the most-recent
/// rollback window's perf telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObserveNetRollbackStatsParams {
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetRollbackStatsView {
    pub schema_version: u32,
    /// Last 64 rollback windows recorded; ordered oldest-first.
    pub recent_windows: Vec<NetRollbackWindowSample>,
    pub windows_within_budget: u64,
    pub windows_over_budget: u64,
    pub p99_resim_us: u32,
    pub max_resim_us: u32,
}

impl NetRollbackStatsView {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            recent_windows: Vec::new(),
            windows_within_budget: 0,
            windows_over_budget: 0,
            p99_resim_us: 0,
            max_resim_us: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetRollbackWindowSample {
    pub from_tick: u64,
    pub to_tick: u64,
    pub resim_us: u32,
    pub within_budget: bool,
}

/// `observe.net.loss_recovery` — projection of the loss-recovery
/// telemetry (redundant input recoveries + FEC recoveries).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObserveNetLossRecoveryParams {
    pub schema_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NetLossRecoveryView {
    pub schema_version: u32,
    pub redundant_input_window_ticks: u8,
    pub recovered_input_ticks_total: u64,
    pub fec_shards_recovered_total: u64,
    pub fec_groups_total: u64,
    pub fec_groups_within_budget: u64,
}

impl NetLossRecoveryView {
    pub fn empty() -> Self {
        Self {
            schema_version: 1,
            redundant_input_window_ticks: cf_net::loss_recovery::redundant_input::REDUNDANT_INPUT_DEFAULT_WINDOW,
            recovered_input_ticks_total: 0,
            fec_shards_recovered_total: 0,
            fec_groups_total: 0,
            fec_groups_within_budget: 0,
        }
    }
}

/// `admin.net.force_relay` — force the next-join NAT traversal flow to
/// engage TURN relay regardless of ICE-lite outcome. Useful for staff /
/// ops debugging.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AdminNetForceRelayParams {
    pub schema_version: u32,
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_session_transport_view_has_blank_fields() {
        let v = NetSessionTransportView::empty();
        assert_eq!(v.schema_version, 1);
        assert!(v.session_id.is_empty());
        assert!(v.transport_mode.is_empty());
        assert!(v.traversal_method.is_empty());
        assert!(v.traversal_path.is_empty());
        assert_eq!(v.elapsed_ms, 0);
        assert_eq!(v.session_semver_packed, 0);
    }

    #[test]
    fn session_transport_view_from_parts_round_trips() {
        let v = NetSessionTransportView::from_parts(
            "sess-1",
            TransportMode::DedicatedServerAuth,
            NatTraversalMethod::IceLite,
            NatTraversalPath::Direct,
            1234,
            0x0104,
        );
        assert_eq!(v.session_id, "sess-1");
        assert_eq!(v.transport_mode, "dedicated_server_auth");
        assert_eq!(v.traversal_method, "ice_lite");
        assert_eq!(v.traversal_path, "direct");
        assert_eq!(v.elapsed_ms, 1234);
        assert_eq!(v.session_semver_packed, 0x0104);
    }

    #[test]
    fn loss_recovery_view_defaults_to_three_tick_window() {
        let v = NetLossRecoveryView::empty();
        assert_eq!(
            v.redundant_input_window_ticks,
            cf_net::loss_recovery::redundant_input::REDUNDANT_INPUT_DEFAULT_WINDOW
        );
    }

    /// **M8B § Notes "All new schemas MUST be added to `dump_schemas --check`"**:
    /// param shapes are schemars-derivable so the cf-control dump_schemas
    /// example can re-emit them.
    #[test]
    fn admin_param_shapes_emit_schemas() {
        let _ = schemars::schema_for!(ObserveNetSessionTransportParams);
        let _ = schemars::schema_for!(ObserveNetRollbackStatsParams);
        let _ = schemars::schema_for!(ObserveNetLossRecoveryParams);
        let _ = schemars::schema_for!(AdminNetForceRelayParams);
    }
}
