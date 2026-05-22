//! Unit tests for the per-event JSON schema validator. The validated
//! contract spans `schemas_consts.rs` (raw schema sources),
//! `schemas_lookup.rs` (`event_schema_for`), and `schemas_validate.rs`
//! (`validate_event_payload`).

use crate::schemas::{event_schema_for, validate_event_payload};
use serde_json::json;


#[test]
fn schemas_load_for_every_registered_event_type() {
    for (cat, ty) in [
        ("input", "intent_received"),
        ("equipment", "weapon_fired"),
        ("combat", "projectile_spawned"),
        ("combat", "wound_added"),
        ("actor", "inventory_dropped"),
        ("equipment", "alarm_registered"),
        ("terrain", "terrain_carved"),
        ("terrain", "terrain_penetration_threshold"),
        ("terrain", "terrain_dirty_region_batch"),
        ("terrain", "terrain_pixel_dislodged"),
        ("terrain", "hazard_contact_or_avoidance"),
        ("terrain", "anchor_material_result"),
        ("terrain", "terrain_material_probe"),
        ("terrain", "terrain_fill_or_repair"),
        // M4 schemas
        ("system", "run_started"),
        ("system", "run_finished"),
        ("system", "category_baseline"),
        ("determinism", "sim_checksum"),
        ("determinism", "first_divergence"),
        ("snapshot", "snapshot_actor"),
        ("snapshot", "snapshot_inventory"),
        ("snapshot", "snapshot_terrain_chunk"),
        ("snapshot", "snapshot_terrain_summary"),
        ("snapshot", "snapshot_chassis"),
        // M4B (2026-05-16): save subsystem + delta-chain + ledger-chain.
        ("snapshot", "baseline_emitted"),
        ("snapshot", "delta_emitted"),
        ("system", "save_completed"),
        ("system", "save_loaded"),
        ("system", "save_migrated"),
        ("system", "ledger_chain_verified"),
        // M4 § M9 firehose surface placeholders.
        ("snapshot", "snapshot_hazard_grid"),
        ("snapshot", "snapshot_affliction"),
        ("snapshot", "snapshot_armor_layer"),
        ("snapshot", "snapshot_atmospherics"),
        ("snapshot", "snapshot_environment_signal"),
        ("snapshot", "snapshot_armor"),
        ("snapshot", "snapshot_internal"),
        ("snapshot", "snapshot_concussion"),
        ("snapshot", "snapshot_fluid"),
        ("snapshot", "snapshot_origin"),
        ("snapshot", "snapshot_shield"),
        ("snapshot", "snapshot_thermal"),
        // M5 deep-damage event-surface lock — armor.* family.
        ("armor", "layer_hp_changed"),
        ("armor", "layer_critical"),
        ("armor", "layer_destroyed"),
        ("armor", "all_layers_destroyed"),
        ("armor", "chunked_off"),
        ("armor", "debris_spawned"),
        ("armor", "repaired"),
        ("armor", "angle_deflection_calculated"),
        ("armor", "ricochet"),
        ("armor", "spalling"),
        ("armor", "penetration_ray_traversed"),
        ("armor", "he_overpressure_wave"),
        ("armor", "heat_jet_penetrated"),
        ("armor", "heat_jet_pre_detonated_by_era"),
        ("armor", "apfsds_penetrated"),
        ("armor", "era_panel_detonated"),
        ("armor", "schurzen_pre_detonated"),
        ("armor", "multi_hit_degradation"),
        ("armor", "reactive_armor_consumed"),
        // M14C § per-tick HEAT + APFSDS + ERA producers.
        ("armor", "heat_jet_traversed"),
        ("armor", "apfsds_long_rod_through"),
        ("armor", "era_pre_detonated"),
        // M14D § projectile-projectile CCD pair-contact event.
        ("collision", "projectile_pair_contact"),
        // M14E § structural-integrity + tunnel-collapse events.
        ("terrain", "structural_integrity_low"),
        ("terrain", "cave_in_triggered"),
        ("terrain", "support_beam_placed"),
        ("terrain", "support_beam_destroyed"),
        ("terrain", "terrain_cascade"),
        // M14F § lateral wall collapse + brace-strut placement events.
        ("terrain", "wall_bulging"),
        ("terrain", "wall_crack_advanced"),
        ("terrain", "wall_rupture"),
        ("terrain", "brace_strut_placed"),
        // M14G § per-wound-type granularity event surface.
        ("wound", "created"),
        ("wound", "escalated"),
        ("wound", "aged"),
        ("wound", "scabbed"),
        ("wound", "scarred"),
        // M5 internal.* family.
        ("internal", "organ_damaged"),
        ("internal", "organ_destroyed"),
        ("internal", "organ_failure_cascade"),
        ("internal", "circuit_damaged"),
        ("internal", "circuit_destroyed"),
        ("internal", "circuit_failure_cascade"),
        // M5 concussion.* + internal_shock.*.
        ("concussion", "dose_changed"),
        ("concussion", "band_changed"),
        ("concussion", "ko_threshold_crossed"),
        ("concussion", "recovered"),
        ("internal_shock", "dose_changed"),
        ("internal_shock", "module_damaged"),
        // M5 fluid.*.
        ("fluid", "leak_started"),
        ("fluid", "leak_rate_changed"),
        ("fluid", "reservoir_warning"),
        ("fluid", "reservoir_critical"),
        ("fluid", "reservoir_empty"),
        ("fluid", "ignition"),
        ("fluid", "ground_splatter_spawned"),
        ("fluid", "leak_stopped"),
        ("fluid", "refilled"),
        // M5 origin.*.
        ("origin", "shot_force_feedback"),
        ("origin", "g_load_dose_changed"),
        ("origin", "helmet_breach"),
        ("origin", "oxygen_supply_changed"),
        // M5 hazard.*.
        ("hazard", "spawned"),
        ("hazard", "spread"),
        ("hazard", "actor_contact"),
        ("hazard", "tick"),
        ("hazard", "dissipated"),
        // M5 affliction.*.
        ("affliction", "applied"),
        ("affliction", "tick"),
        ("affliction", "cleared"),
        ("affliction", "escalated"),
        // M5 atmos.*.
        ("atmos", "pressure_changed"),
        ("atmos", "temperature_changed"),
        ("atmos", "gas_released"),
        ("atmos", "breach_detected"),
        ("atmos", "combustion_ignition"),
        ("atmos", "phase_transition"),
        ("atmos", "pipe_flow"),
        ("atmos", "pipe_freeze"),
        ("atmos", "pipe_rupture"),
        ("atmos", "electrolysis_started"),
        // M14B atmos.* (wind + stratification producers).
        ("atmos", "wind_force_applied"),
        ("atmos", "gas_stratified"),
        // M14B gravity.* (gravity field producer).
        ("gravity", "override_activated"),
        ("gravity", "override_deactivated"),
        // M5 shield.*.
        ("shield", "hit"),
        ("shield", "depleted"),
        ("shield", "regen_started"),
        ("shield", "regen_completed"),
        ("shield", "disrupted"),
        // M5 environment.*.
        ("environment", "signal_delta"),
        ("environment", "signal_aggregated"),
        // M5 thermal.*.
        ("thermal", "signature_changed"),
        ("thermal", "heat_exchanged"),
        ("thermal", "material_phase_change"),
        // M5 combat.projectile_hit_mo expanded payload.
        ("combat", "projectile_hit_mo"),
        // M5 audio.event_requested (M5 mandate, M5-A1).
        ("audio", "event_requested"),
        // M5-A2: combat melee + explosive hit_mo siblings.
        ("combat", "melee_hit_mo"),
        ("combat", "explosive_hit_mo"),
        // M6 actor / combat / equipment / inventory / perception / squad.
        ("actor", "action_rejected"),
        ("actor", "climb_started"),
        ("actor", "dive_started"),
        ("actor", "facing_changed"),
        ("actor", "lean_changed"),
        ("actor", "slide_started"),
        ("actor", "stamina_changed"),
        ("actor", "stance_changed"),
        ("actor", "vault_started"),
        ("combat", "knife_throw_landed"),
        ("combat", "knife_throw_started"),
        ("combat", "stealth_kill_executed"),
        ("equipment", "beacon_dropped"),
        ("equipment", "bipod_deployed"),
        ("equipment", "bipod_stowed"),
        ("equipment", "drill_overheated"),
        ("equipment", "fire_mode_cycled"),
        ("equipment", "grenade_cooked"),
        ("equipment", "grenade_detonated"),
        ("equipment", "grenade_thrown"),
        ("equipment", "item_dropped"),
        ("equipment", "item_picked_up"),
        ("equipment", "magazine_changed"),
        ("equipment", "melee_swing"),
        ("equipment", "sensor_pulse_fired"),
        ("equipment", "shell_ejected"),
        ("equipment", "suppressor_attached"),
        ("equipment", "tool_broken"),
        ("equipment", "tool_repaired"),
        ("equipment", "tool_used"),
        ("equipment", "weapon_swap_completed"),
        ("equipment", "weapon_swap_started"),
        ("inventory", "tank_slot_reserved"),
        ("inventory", "weight_changed"),
        ("equipment", "item_picked_up_with_mass"),
        ("equipment", "item_dropped_with_mass"),
        ("inventory", "encumbrance_threshold_crossed"),
        ("inventory", "container_nested"),
        ("body_armor", "degraded"),
        ("atgm", "lock_acquired"),
        ("actor", "revived"),
        ("mine", "detonated"),
        ("mortar", "crewed"),
        ("perception", "actor_signal"),
        ("perception", "footstep_emitted"),
        ("perception", "occlusion_applied"),
        ("perception", "stealth_meter_changed"),
        ("squad", "command_issued"),
        ("squad", "member_added"),
        ("squad", "waypoint_marked"),
        ("mission", "reactor_hp_changed"),
        ("mission", "reactor_destroyed"),
        ("mission", "reactor_pressure_state_changed"),
        ("mission", "timer_warning_threshold"),
        ("terrain", "material_state_changed"),
        ("terrain", "pixel_removed"),
        ("terrain", "cascade_triggered"),
        ("terrain", "debris_spawned"),
        // and per-guard path invalidation.
        ("ai", "target_scored"),
        ("ai", "path_invalidated"),
        ("net", "protocol_negotiated"),
        ("net", "rollback_window"),
        ("net", "input_resent_redundant"),
        ("net", "fec_recovered"),
        ("net", "nat_traversal_outcome"),
        ("cinematic", "started"),
        ("cinematic", "chapter_marker"),
        ("cinematic", "skipped"),
        ("cinematic", "paused"),
        ("cinematic", "resumed"),
        ("cinematic", "ended"),
        ("cinematic", "narration_word"),
    ] {
        let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
        let _parsed_value: serde_json::Value =
            serde_json::from_str(raw).unwrap_or_else(|e| panic!("schema json parse error for {cat}.{ty}: {e}"));
    }
}

/// each new net.* event family.
#[test]
fn m8b_net_protocol_negotiated_validates() {
    let payload = json!({
        "session_id": "sess-1",
        "accepted": true,
        "server_semver_packed": 0x0107u32,
        "client_semver_packed": 0x0104u32,
        "session_semver_packed": 0x0104u32,
        "granted_features": ["fec", "ice_lite"],
    });
    validate_event_payload("net", "protocol_negotiated", &payload).expect("valid");
}

#[test]
fn m8b_net_protocol_negotiated_rejected_validates() {
    let payload = json!({
        "session_id": "sess-2",
        "accepted": false,
        "server_semver_packed": 0x0107u32,
        "client_semver_packed": 0x0200u32,
        "session_semver_packed": 0u32,
        "granted_features": [],
        "reject_reason": "protocol_major_mismatch",
        "download_url": "https://corefall.example/update",
    });
    validate_event_payload("net", "protocol_negotiated", &payload).expect("valid reject");
}

#[test]
fn m8b_net_rollback_window_validates() {
    let payload = json!({
        "from_tick": 614u32,
        "to_tick": 620u32,
        "resim_us": 7100u32,
        "cause": "input_mismatch",
        "rollback_to_tick_elapsed_us": 800u32,
        "per_frame_resim_elapsed_us": 6300u32,
        "within_budget": true,
    });
    validate_event_payload("net", "rollback_window", &payload).expect("valid");
}

#[test]
fn m8b_net_input_resent_redundant_validates() {
    let payload = json!({
        "session_id": "sess-1",
        "recovered_tick": 700u32,
        "carrier_tick": 701u32,
        "window_ticks": 3u32,
    });
    validate_event_payload("net", "input_resent_redundant", &payload).expect("valid");
}

#[test]
fn m8b_net_fec_recovered_validates() {
    let payload = json!({
        "group_id": 42u32,
        "k": 4u32,
        "m": 2u32,
        "shards_lost": 1u32,
        "payload_bytes": 512u32,
    });
    validate_event_payload("net", "fec_recovered", &payload).expect("valid");
}

#[test]
fn m8b_net_nat_traversal_outcome_validates_direct() {
    let payload = json!({
        "session_id": "sess-1",
        "method": "ice_lite",
        "path": "direct",
        "elapsed_ms": 2500u32,
    });
    validate_event_payload("net", "nat_traversal_outcome", &payload).expect("valid direct");
}

#[test]
fn m8b_net_nat_traversal_outcome_validates_relay() {
    let payload = json!({
        "session_id": "sess-1",
        "method": "turn_relay",
        "path": "relay",
        "elapsed_ms": 5500u32,
    });
    validate_event_payload("net", "nat_traversal_outcome", &payload).expect("valid relay");
}

#[test]
fn m8b_net_nat_traversal_outcome_rejects_invalid_method() {
    let payload = json!({
        "session_id": "sess-1",
        "method": "magic_pony",
        "path": "direct",
        "elapsed_ms": 1000u32,
    });
    let err = validate_event_payload("net", "nat_traversal_outcome", &payload).unwrap_err();
    assert!(err.contains("method"), "expected method enum error, got: {err}");
}

#[test]
fn terrain_carved_event_validates_minimum_payload() {
    let payload = json!({
        "bbox": { "min": [0, 0], "max": [10, 10] },
        "count": 12u32,
        "removed_count": 12u32,
        "debris_count": 12u32,
        "material_ids": [1u32],
    });
    validate_event_payload("terrain", "terrain_carved", &payload).expect("valid payload");
}

#[test]
fn terrain_penetration_threshold_event_validates() {
    let payload = json!({
        "projectile_id": 7u32,
        "material_id": 1u32,
        "passed": true,
        "impulse_squared": 256.0,
        "integrity_squared": 100.0,
    });
    validate_event_payload("terrain", "terrain_penetration_threshold", &payload).expect("valid");
}

// + per-source occlusion + Doppler shift. All four schemas are
// payload-shaped (additionalProperties: true; required fields gated).

#[test]
fn m12b_audio_spatial_resolved_validates_minimum_payload() {
    #[allow(clippy::approx_constant)]
    let payload = json!({
        "canonical_name": "weapon_fired",
        "azimuth_rad": 0.78539816,
        "elevation_rad": 0.0,
        "distance_m": 10.0,
        "hrir_index": { "azimuth_bucket": 4, "elevation_bucket": 0 },
        "direction": "NE",
        "gain": 0.5,
        "source_position": [10.0, 10.0],
        "listener_position": [0.0, 0.0],
        "listener_facing_rad": 0.0
    });
    validate_event_payload("audio", "spatial_resolved", &payload).expect("valid");
}

#[test]
fn m12b_audio_spatial_resolved_rejects_unknown_direction() {
    let payload = json!({
        "canonical_name": "weapon_fired",
        "azimuth_rad": 0.0,
        "distance_m": 1.0,
        "hrir_index": { "azimuth_bucket": 0, "elevation_bucket": 0 },
        "direction": "up",
        "gain": 1.0
    });
    let err = validate_event_payload("audio", "spatial_resolved", &payload).unwrap_err();
    assert!(err.contains("direction"), "got: {err}");
}

#[test]
fn m12b_audio_reverb_applied_validates_minimum_payload() {
    let payload = json!({
        "canonical_name": "weapon_fired",
        "room_id": 7,
        "tail_seconds": 2.1,
        "decay_coefficient": 0.85,
        "decay_band": "bright",
        "wet_dry_mix": 0.57,
        "early_reflection_delay_ms": 15.0
    });
    validate_event_payload("audio", "reverb_applied", &payload).expect("valid");
}

#[test]
fn m12b_audio_reverb_applied_rejects_unknown_decay_band() {
    let payload = json!({
        "canonical_name": "weapon_fired",
        "room_id": 7,
        "tail_seconds": 2.1,
        "decay_coefficient": 0.85,
        "decay_band": "synthwave",
        "wet_dry_mix": 0.57
    });
    let err = validate_event_payload("audio", "reverb_applied", &payload).unwrap_err();
    assert!(err.contains("decay_band"), "got: {err}");
}

#[test]
fn m12b_audio_occluded_validates_minimum_payload() {
    let payload = json!({
        "canonical_name": "weapon_fired",
        "occlusion_db": -28.0,
        "low_pass_cutoff_hz": 800.0,
        "wall_count": 1,
        "clipped": false
    });
    validate_event_payload("audio", "occluded", &payload).expect("valid");
}

#[test]
fn m12b_audio_occluded_rejects_positive_db() {
    let payload = json!({
        "canonical_name": "weapon_fired",
        "occlusion_db": 5.0,
        "low_pass_cutoff_hz": 800.0,
        "wall_count": 1
    });
    let err = validate_event_payload("audio", "occluded", &payload).unwrap_err();
    assert!(err.contains("occlusion_db"), "got: {err}");
}

#[test]
fn m12b_audio_doppler_shifted_validates_minimum_payload() {
    let payload = json!({
        "canonical_name": "projectile_fly",
        "doppler_factor": 0.30,
        "clamped": false,
        "speed_of_sound_m_per_s": 343.0,
        "medium": "air"
    });
    validate_event_payload("audio", "doppler_shifted", &payload).expect("valid");
}

#[test]
fn m12b_audio_doppler_shifted_rejects_factor_outside_safe_range() {
    let payload = json!({
        "canonical_name": "projectile_fly",
        "doppler_factor": 10.0,
        "clamped": false,
        "speed_of_sound_m_per_s": 343.0
    });
    let err = validate_event_payload("audio", "doppler_shifted", &payload).unwrap_err();
    assert!(err.contains("doppler_factor"), "got: {err}");
}

#[test]
fn m12b_audio_doppler_shifted_rejects_unknown_medium() {
    let payload = json!({
        "canonical_name": "projectile_fly",
        "doppler_factor": 0.5,
        "clamped": false,
        "speed_of_sound_m_per_s": 343.0,
        "medium": "plasma"
    });
    let err = validate_event_payload("audio", "doppler_shifted", &payload).unwrap_err();
    assert!(err.contains("medium"), "got: {err}");
}

// payload-shaped (additionalProperties: true; required fields gated).

#[test]
fn m12c_cinematic_started_validates_minimum_payload() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "source": "opening",
        "replay": false,
    });
    validate_event_payload("cinematic", "started", &payload).expect("valid");
}

#[test]
fn m12c_cinematic_started_rejects_unknown_source() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "source": "title_card",
    });
    let err = validate_event_payload("cinematic", "started", &payload).unwrap_err();
    assert!(err.contains("source"), "got: {err}");
}

#[test]
fn m12c_cinematic_chapter_marker_validates() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "chapter_id": "dropship_door_opens",
        "ms": 8000,
    });
    validate_event_payload("cinematic", "chapter_marker", &payload).expect("valid");
}

#[test]
fn m12c_cinematic_skipped_validates_user_input() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "skipped_at_ms": 3500,
        "reason": "user_input",
    });
    validate_event_payload("cinematic", "skipped", &payload).expect("valid");
}

#[test]
fn m12c_cinematic_skipped_validates_sandbox_suppressed() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "skipped_at_ms": 0,
        "reason": "sandbox_suppressed",
    });
    validate_event_payload("cinematic", "skipped", &payload).expect("valid");
}

#[test]
fn m12c_cinematic_skipped_rejects_unknown_reason() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "skipped_at_ms": 0,
        "reason": "rage_quit",
    });
    let err = validate_event_payload("cinematic", "skipped", &payload).unwrap_err();
    assert!(err.contains("reason"), "got: {err}");
}

#[test]
fn m12c_cinematic_paused_resumed_validate() {
    let p = json!({"id": "cin_intro", "ms": 4200});
    validate_event_payload("cinematic", "paused", &p).expect("paused valid");
    validate_event_payload("cinematic", "resumed", &p).expect("resumed valid");
}

#[test]
fn m12c_cinematic_ended_validates() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "duration_ms": 30000,
        "was_skipped": false,
    });
    validate_event_payload("cinematic", "ended", &payload).expect("valid");
}

#[test]
fn m12c_cinematic_narration_word_validates() {
    let payload = json!({
        "id": "cin_intro_reactor_defense",
        "word_index": 7,
        "text": "dropship",
        "ms": 2100,
    });
    validate_event_payload("cinematic", "narration_word", &payload).expect("valid");
}

#[test]
fn unknown_event_type_is_ok_by_default() {
    let payload = json!({});
    assert!(validate_event_payload("not", "registered", &payload).is_ok());
}

#[test]
fn validates_input_intent_received_required_fields() {
    let mut payload = json!({
        "actor": 1,
        "source": "cfctl",
        "move_x": 0.0,
        "aim_x": 1.0,
        "aim_y": 0.0,
        "jump": false,
        "fire": false,
        "reload": false,
    });
    assert!(validate_event_payload("input", "intent_received", &payload).is_ok());
    payload.as_object_mut().unwrap().remove("actor");
    let err = validate_event_payload("input", "intent_received", &payload).unwrap_err();
    assert!(err.contains("`actor` missing"), "got: {err}");
}

#[test]
fn validates_projectile_spawned_array_arity() {
    let bad = json!({
        "id": 1,
        "owner": 2,
        "origin": [0.0],
        "velocity": [1.0, 0.0],
        "damage": 12.0,
    });
    let err = validate_event_payload("combat", "projectile_spawned", &bad).unwrap_err();
    assert!(err.contains("origin"), "got: {err}");
}

/// the envelope-shaped schema (the validator must walk into
/// `properties.payload` to find required/properties).
#[test]
fn m5_armor_layer_destroyed_payload_validates() {
    let payload = json!({
        "item_id": 12,
        "zone": "torso",
        "layer": "External",
        "breach_kind": "punctured",
    });
    validate_event_payload("armor", "layer_destroyed", &payload).expect("valid payload");
}

/// declares `additionalProperties: true` at the payload level so producer
/// extensions don't bump the envelope).
#[test]
fn m5_armor_layer_destroyed_accepts_additive_payload_extension() {
    let payload = json!({
        "item_id": 12,
        "zone": "torso",
        "layer": "External",
        "breach_kind": "punctured",
        "bound_zone": "torso_front",
    });
    validate_event_payload("armor", "layer_destroyed", &payload).expect("additive ok");
}

#[test]
fn m5_armor_layer_destroyed_rejects_missing_breach_kind() {
    let payload = json!({
        "item_id": 12,
        "zone": "torso",
        "layer": "External",
    });
    let err = validate_event_payload("armor", "layer_destroyed", &payload).unwrap_err();
    assert!(err.contains("breach_kind"), "got: {err}");
}

/// validates with a representative payload. Closes the test-coverage gap
/// flagged by the validator audit.
#[test]
fn m5_per_family_happy_path() {
    validate_event_payload(
        "armor",
        "layer_hp_changed",
        &json!({
            "actor_id": 1,
            "item_id": 10,
            "zone": "torso",
            "layer": "External",
            "from": 50.0,
            "to": 30.0,
            "cause": "kinetic_round",
            "ap_factor": 0.7,
        }),
    )
    .expect("armor.layer_hp_changed valid");
    validate_event_payload(
        "internal",
        "organ_damaged",
        &json!({
            "actor_id": 1,
            "organ_id": "heart",
            "organ_kind": "vital",
            "from_hp": 100.0,
            "to_hp": 60.0,
            "cause": "kinetic_pierce",
            "source_hit_event_id": "run:42:7",
        }),
    )
    .expect("internal.organ_damaged valid");
    validate_event_payload(
        "concussion",
        "band_changed",
        &json!({
            "actor_id": 1,
            "from_band": "Mild",
            "to_band": "Moderate",
            "dose": 45.0,
        }),
    )
    .expect("concussion.band_changed valid");
    validate_event_payload(
        "internal_shock",
        "dose_changed",
        &json!({
            "actor_id": 2,
            "from_dose": 10.0,
            "to_dose": 40.0,
            "source_event_id": "run:42:9",
        }),
    )
    .expect("internal_shock.dose_changed valid");
    validate_event_payload(
        "fluid",
        "leak_started",
        &json!({
            "actor_id": 1,
            "fluid_kind": "oil",
            "source_module_id": "oil_reservoir",
            "leak_rate": 0.5,
            "position": [10.0, 20.0],
        }),
    )
    .expect("fluid.leak_started valid");
    validate_event_payload(
        "origin",
        "g_load_dose_changed",
        &json!({
            "actor_id": 1,
            "from_dose": 0.0,
            "to_dose": 4.5,
            "source": "fall",
        }),
    )
    .expect("origin.g_load_dose_changed valid");
    validate_event_payload(
        "hazard",
        "spawned",
        &json!({
            "hazard_id": "h-1",
            "kind": "fire",
            "position": [5.0, 5.0],
            "intensity": 0.8,
            "source_event_id": "run:42:11",
        }),
    )
    .expect("hazard.spawned valid");
    validate_event_payload(
        "affliction",
        "applied",
        &json!({
            "actor_id": 1,
            "kind": "blinded",
            "source_event_id": "run:42:13",
            "expected_duration_ticks": 90,
            "severity_0_1": 0.6,
        }),
    )
    .expect("affliction.applied (with new `blinded` kind) valid");
    validate_event_payload(
        "atmos",
        "gas_released",
        &json!({
            "atm_id": "atm-1",
            "gas": "H2",
            "moles": 3.5,
            "source": "electrolysis",
            "ignition_risk": 0.4,
        }),
    )
    .expect("atmos.gas_released valid");
    validate_event_payload(
        "shield",
        "hit",
        &json!({
            "actor_id": 1,
            "hp_before": 100.0,
            "hp_after": 75.0,
            "cause": "kinetic_round",
        }),
    )
    .expect("shield.hit valid");
    validate_event_payload(
        "environment",
        "signal_delta",
        &json!({
            "actor_id": 1,
            "slice": "thermal",
            "from": 295.0,
            "to": 310.0,
            "tick": 100,
        }),
    )
    .expect("environment.signal_delta valid");
    validate_event_payload(
        "thermal",
        "material_phase_change",
        &json!({
            "material_id": 7,
            "from_phase": "solid",
            "to_phase": "liquid",
            "position": [1.0, 2.0],
            "latent_heat_consumed_j": 12345.0,
        }),
    )
    .expect("thermal.material_phase_change valid");
    validate_event_payload(
        "combat",
        "projectile_hit_mo",
        &json!({
            "shooter_id": 1,
            "weapon_id": 5,
            "projectile_id": 99,
            "target_id": 2,
            "hit_zone": "torso",
            "impact_point": [10.0, 20.0],
            "impact_normal": [0.0, 1.0],
            "impact_impulse": 50.0,
            "impact_energy_j": 1200.0,
            "ap_factor": 0.5,
            "ap_round_tier": "standard",
            "material_at_impact": 1,
            "surface_kind": "armor_external",
            "armor_effective_hardness": 0.8,
            "armor_absorbed_dmg": 30.0,
            "passthrough_dmg": 20.0,
            "damage_kind": "kinetic",
            "hp_before": 100.0,
            "hp_after": 80.0,
            "damage_amount": 20.0,
            "pierced_armor": false,
            "parent_hit_event_id": "run:42:5",
        }),
    )
    .expect("combat.projectile_hit_mo valid");
    validate_event_payload(
        "audio",
        "event_requested",
        &json!({
            "kind": "material_state",
            "material": "metal",
            "impact_state": "pristine_hit",
            "surface_kind": "armor_external",
            "damage_kind": "kinetic",
            "source_event_id": "run:42:5",
        }),
    )
    .expect("audio.event_requested valid");
    validate_event_payload(
        "combat",
        "melee_hit_mo",
        &json!({
            "attacker_id": 1,
            "melee_weapon_id": 8,
            "melee_kind": "knife_stab",
            "target_id": 2,
            "hit_zone": "torso",
            "impact_point": [3.0, 4.0],
            "impact_normal": [0.0, 1.0],
            "impact_impulse": 5.0,
            "impact_energy_j": 80.0,
            "material_at_impact": 1,
            "surface_kind": "flesh",
            "armor_effective_hardness": 0.0,
            "armor_absorbed_dmg": 0.0,
            "passthrough_dmg": 25.0,
            "damage_kind": "kinetic",
            "hp_before": 100.0,
            "hp_after": 75.0,
            "damage_amount": 25.0,
            "pierced_armor": false,
            "parent_hit_event_id": "run:42:20",
        }),
    )
    .expect("combat.melee_hit_mo valid");
    validate_event_payload(
        "combat",
        "explosive_hit_mo",
        &json!({
            "attacker_id": 1,
            "weapon_id": 12,
            "explosive_kind": "frag",
            "target_id": 2,
            "hit_zone": "torso",
            "epicenter": [10.0, 10.0],
            "range_to_epicenter_m": 1.5,
            "blast_radius_m": 4.0,
            "impact_impulse": 200.0,
            "impact_energy_j": 5000.0,
            "overpressure_pa": 150000.0,
            "material_at_impact": 1,
            "surface_kind": "armor_external",
            "armor_effective_hardness": 0.6,
            "armor_absorbed_dmg": 20.0,
            "passthrough_dmg": 60.0,
            "damage_kind": "thermal",
            "hp_before": 100.0,
            "hp_after": 40.0,
            "damage_amount": 60.0,
            "pierced_armor": false,
            "parent_hit_event_id": "run:42:30",
        }),
    )
    .expect("combat.explosive_hit_mo valid");
}

/// 23-affliction enum.
#[test]
fn m5_internal_failure_cascade_rejects_unknown_affliction() {
    let payload = json!({
        "actor_id": 1,
        "organ_id": "heart",
        "applied_afflictions": ["bleeding", "not_a_real_affliction"],
        "hp_drain_per_s": 1.0,
    });
    let err = validate_event_payload("internal", "organ_failure_cascade", &payload).unwrap_err();
    assert!(
        err.contains("applied_afflictions"),
        "expected error about applied_afflictions, got: {err}"
    );
}

#[test]
fn m5_internal_failure_cascade_accepts_known_affliction() {
    let payload = json!({
        "actor_id": 1,
        "organ_id": "lungs_left",
        "applied_afflictions": ["bleeding", "hypoxic", "concussed"],
        "hp_drain_per_s": 1.5,
    });
    validate_event_payload("internal", "organ_failure_cascade", &payload).expect("valid applied_afflictions array");
}

/// environment.signal_aggregated.signal.{schema_version, active_hazards}.
/// A missing required nested field is rejected.
#[test]
fn m5_environment_signal_aggregated_rejects_missing_signal_field() {
    let payload = json!({
        "actor_id": 1,
        "tick": 100,
        "signal": {
            "active_hazards": ["Hypoxic"]
        }
    });
    let err = validate_event_payload("environment", "signal_aggregated", &payload).unwrap_err();
    assert!(err.contains("schema_version"), "got: {err}");
}

/// values in active_hazards array.
#[test]
fn m5_environment_signal_aggregated_rejects_bad_hazard_class() {
    let payload = json!({
        "actor_id": 1,
        "tick": 100,
        "signal": {
            "schema_version": 1,
            "active_hazards": ["Hypoxic", "NotARealHazard"]
        }
    });
    let err = validate_event_payload("environment", "signal_aggregated", &payload).unwrap_err();
    assert!(
        err.contains("active_hazards") || err.contains("NotARealHazard"),
        "got: {err}"
    );
}

#[test]
fn m5_environment_signal_aggregated_accepts_valid_signal() {
    let payload = json!({
        "actor_id": 1,
        "tick": 100,
        "signal": {
            "schema_version": 1,
            "active_hazards": ["Hypoxic", "Hyperthermic", "Radiation"]
        }
    });
    validate_event_payload("environment", "signal_aggregated", &payload).expect("valid signal");
}

/// new locked maximum and should pass.
#[test]
fn m5_concussion_dose_changed_accepts_max_dose() {
    let payload = json!({
        "actor_id": 1,
        "from_dose": 99.0,
        "to_dose": 100.0,
        "source_event_id": "run:42:7",
        "origin_id": "Human",
    });
    validate_event_payload("concussion", "dose_changed", &payload).expect("dose=100 accepted");
}

/// maximum and is rejected.
#[test]
fn m5_concussion_dose_changed_rejects_over_max_dose() {
    let payload = json!({
        "actor_id": 1,
        "from_dose": 50.0,
        "to_dose": 100.1,
        "source_event_id": "run:42:7",
        "origin_id": "Human",
    });
    let err = validate_event_payload("concussion", "dose_changed", &payload).unwrap_err();
    assert!(err.contains("100"), "expected error about maximum 100, got: {err}");
}

/// maximum 1.0 and is rejected.
#[test]
fn m5_armor_ricochet_rejects_probability_over_one() {
    let payload = json!({
        "impact_angle": 0.5,
        "ricochet_probability": 1.5,
        "was_ricocheted": false,
        "deflection_vector": [0.0, 1.0],
    });
    let err = validate_event_payload("armor", "ricochet", &payload).unwrap_err();
    assert!(err.contains("ricochet_probability"), "got: {err}");
}

#[test]
fn m5_armor_spalling_rejects_fragment_count_out_of_range() {
    let payload_high = json!({
        "item_id": 1,
        "zone": "torso",
        "layer": "External",
        "fragment_count": 99,
        "damage_per_fragment": 10.0,
        "cause_event_id": "run:42:1",
    });
    let err = validate_event_payload("armor", "spalling", &payload_high).unwrap_err();
    assert!(err.contains("fragment_count"), "got: {err}");
    let payload_low = json!({
        "item_id": 1,
        "zone": "torso",
        "layer": "External",
        "fragment_count": 0,
        "damage_per_fragment": 10.0,
        "cause_event_id": "run:42:1",
    });
    let err = validate_event_payload("armor", "spalling", &payload_low).unwrap_err();
    assert!(err.contains("fragment_count"), "got: {err}");
}

/// integrity.
#[test]
fn m5_hazard_spread_requires_hazard_id() {
    let payload = json!({
        "from_pos": [0.0, 0.0],
        "to_pos": [1.0, 0.0],
        "kind": "fire",
        "intensity": 0.5,
        "rate": 0.1,
    });
    let err = validate_event_payload("hazard", "spread", &payload).unwrap_err();
    assert!(err.contains("hazard_id"), "got: {err}");
}

/// locked to 5..10 per spec ("full blackout 5-10s").
#[test]
fn m5_concussion_ko_threshold_crossed_rejects_out_of_range_duration() {
    let payload_short = json!({
        "actor_id": 1,
        "ko_duration_s": 2.0,
    });
    let err = validate_event_payload("concussion", "ko_threshold_crossed", &payload_short).unwrap_err();
    assert!(err.contains("ko_duration_s"), "got: {err}");
    let payload_long = json!({
        "actor_id": 1,
        "ko_duration_s": 30.0,
    });
    let err = validate_event_payload("concussion", "ko_threshold_crossed", &payload_long).unwrap_err();
    assert!(err.contains("ko_duration_s"), "got: {err}");
}

/// `parent_hit_event_id` instead of the envelope-colliding
/// `parent_event_id`.
#[test]
fn m5_combat_projectile_hit_mo_rejects_envelope_named_parent() {
    let payload = json!({
        "shooter_id": 1,
        "weapon_id": 5,
        "projectile_id": 99,
        "target_id": 2,
        "hit_zone": "torso",
        "impact_point": [10.0, 20.0],
        "impact_normal": [0.0, 1.0],
        "impact_impulse": 50.0,
        "impact_energy_j": 1200.0,
        "ap_factor": 0.5,
        "ap_round_tier": "standard",
        "material_at_impact": 1,
        "surface_kind": "armor_external",
        "armor_effective_hardness": 0.8,
        "armor_absorbed_dmg": 30.0,
        "passthrough_dmg": 20.0,
        "damage_kind": "kinetic",
        "hp_before": 100.0,
        "hp_after": 80.0,
        "damage_amount": 20.0,
        "pierced_armor": false,
        "parent_event_id": "run:42:5",
    });
    let err = validate_event_payload("combat", "projectile_hit_mo", &payload).unwrap_err();
    assert!(
        err.contains("parent_hit_event_id"),
        "expected error about parent_hit_event_id, got: {err}"
    );
}

/// rejected on concussion.dose_changed.
#[test]
fn m5_concussion_dose_changed_rejects_bad_origin() {
    let payload = json!({
        "actor_id": 1,
        "from_dose": 10.0,
        "to_dose": 20.0,
        "source_event_id": "run:42:7",
        "origin_id": "Construct",
    });
    let result = validate_event_payload("concussion", "dose_changed", &payload);
    assert!(result.is_err(), "expected rejection of non-canonical Origin");
}

#[test]
fn m5_armor_layer_destroyed_rejects_bad_zone_enum() {
    let payload = json!({
        "item_id": 12,
        "zone": "not_a_zone",
        "layer": "External",
        "breach_kind": "punctured",
    });
    let err = validate_event_payload("armor", "layer_destroyed", &payload).unwrap_err();
    assert!(err.contains("zone"), "got: {err}");
}

/// representative utility-scored target payload (one player candidate +
/// one reactor candidate, player chosen, full weights breakdown).
#[test]
fn m9_ai_target_scored_validates_happy_path() {
    let payload = json!({
        "actor": 11,
        "target_actor": 7,
        "chosen_id": "7",
        "score": 1.42,
        "candidates": [
            {
                "id": "7",
                "kind": "player",
                "actor_id": 7,
                "distance": 30.0,
                "has_los": true,
                "is_player": true,
                "is_high_value_static": false,
                "score": 1.42,
                "reason": "player_aggressive_los",
            },
            {
                "id": "reactor_alpha",
                "kind": "reactor",
                "reactor_id": "reactor_alpha",
                "distance": 80.0,
                "has_los": true,
                "is_player": false,
                "is_high_value_static": true,
                "score": 1.10,
                "reason": "defensive_value",
            },
        ],
        "rationale": "player_aggressive: player_aggressive_los",
        "weights": {
            "proximity": 1.0,
            "los": 1.0,
            "threat": 0.7,
            "value": 0.5,
        },
    });
    validate_event_payload("ai", "target_scored", &payload).expect("ai.target_scored valid");
}

#[test]
fn m9_ai_target_scored_requires_chosen_id() {
    let payload = json!({
        "actor": 11,
        "target_actor": 7,
        "candidates": [],
        "rationale": "no_candidates",
    });
    let err = validate_event_payload("ai", "target_scored", &payload).unwrap_err();
    assert!(err.contains("chosen_id"), "got: {err}");
}

/// representative payload (guard's planned pursuit line crosses a
/// freshly-carved bbox).
#[test]
fn m9_ai_path_invalidated_validates_happy_path() {
    let payload = json!({
        "actor": 9,
        "actor_id": 9,
        "bbox": { "min": [10.0, 20.0], "max": [40.0, 50.0] },
        "old_path": [[5.0, 5.0], [60.0, 60.0]],
        "reason": "terrain_dirty",
        "fraction_of_path_dirty": 0.375,
    });
    validate_event_payload("ai", "path_invalidated", &payload).expect("ai.path_invalidated valid");
}

#[test]
fn m9_ai_path_invalidated_requires_bbox() {
    let payload = json!({
        "actor": 9,
        "actor_id": 9,
        "old_path": [[5.0, 5.0]],
        "reason": "terrain_dirty",
        "fraction_of_path_dirty": 0.1,
    });
    let err = validate_event_payload("ai", "path_invalidated", &payload).unwrap_err();
    assert!(err.contains("bbox"), "got: {err}");
}

/// a const property — proves the M5 conformance contract per the spec's
/// "each schema declares schema_version=\"0.1\" matching the M4 locked
/// envelope" scenario.
#[test]
fn m5_schemas_declare_schema_version_v0_1() {
    let pairs: &[(&str, &str)] = &[
        ("armor", "layer_hp_changed"),
        ("armor", "layer_critical"),
        ("armor", "layer_destroyed"),
        ("armor", "all_layers_destroyed"),
        ("armor", "chunked_off"),
        ("armor", "debris_spawned"),
        ("armor", "repaired"),
        ("armor", "angle_deflection_calculated"),
        ("armor", "ricochet"),
        ("armor", "spalling"),
        ("armor", "penetration_ray_traversed"),
        ("armor", "he_overpressure_wave"),
        ("armor", "heat_jet_penetrated"),
        ("armor", "heat_jet_pre_detonated_by_era"),
        ("armor", "apfsds_penetrated"),
        ("armor", "era_panel_detonated"),
        ("armor", "schurzen_pre_detonated"),
        ("armor", "multi_hit_degradation"),
        ("armor", "reactive_armor_consumed"),
        // M14C per-tick producers.
        ("armor", "heat_jet_traversed"),
        ("armor", "apfsds_long_rod_through"),
        ("armor", "era_pre_detonated"),
        // M14D projectile-projectile CCD pair contact.
        ("collision", "projectile_pair_contact"),
        // M14G per-wound-type granularity event surface.
        ("wound", "created"),
        ("wound", "escalated"),
        ("wound", "aged"),
        ("wound", "scabbed"),
        ("wound", "scarred"),
        ("internal", "organ_damaged"),
        ("internal", "organ_destroyed"),
        ("internal", "organ_failure_cascade"),
        ("internal", "circuit_damaged"),
        ("internal", "circuit_destroyed"),
        ("internal", "circuit_failure_cascade"),
        ("concussion", "dose_changed"),
        ("concussion", "band_changed"),
        ("concussion", "ko_threshold_crossed"),
        ("concussion", "recovered"),
        ("internal_shock", "dose_changed"),
        ("internal_shock", "module_damaged"),
        ("fluid", "leak_started"),
        ("fluid", "leak_rate_changed"),
        ("fluid", "reservoir_warning"),
        ("fluid", "reservoir_critical"),
        ("fluid", "reservoir_empty"),
        ("fluid", "ignition"),
        ("fluid", "ground_splatter_spawned"),
        ("fluid", "leak_stopped"),
        ("fluid", "refilled"),
        ("origin", "shot_force_feedback"),
        ("origin", "g_load_dose_changed"),
        ("origin", "helmet_breach"),
        ("origin", "oxygen_supply_changed"),
        ("hazard", "spawned"),
        ("hazard", "spread"),
        ("hazard", "actor_contact"),
        ("hazard", "tick"),
        ("hazard", "dissipated"),
        ("affliction", "applied"),
        ("affliction", "tick"),
        ("affliction", "cleared"),
        ("affliction", "escalated"),
        ("atmos", "pressure_changed"),
        ("atmos", "temperature_changed"),
        ("atmos", "gas_released"),
        ("atmos", "breach_detected"),
        ("atmos", "combustion_ignition"),
        ("atmos", "phase_transition"),
        ("atmos", "pipe_flow"),
        ("atmos", "pipe_freeze"),
        ("atmos", "pipe_rupture"),
        ("atmos", "electrolysis_started"),
        ("atmos", "wind_force_applied"),
        ("atmos", "gas_stratified"),
        ("gravity", "override_activated"),
        ("gravity", "override_deactivated"),
        ("shield", "hit"),
        ("shield", "depleted"),
        ("shield", "regen_started"),
        ("shield", "regen_completed"),
        ("shield", "disrupted"),
        ("environment", "signal_delta"),
        ("environment", "signal_aggregated"),
        ("thermal", "signature_changed"),
        ("thermal", "heat_exchanged"),
        ("thermal", "material_phase_change"),
        ("combat", "projectile_hit_mo"),
        ("audio", "event_requested"),
        ("combat", "melee_hit_mo"),
        ("combat", "explosive_hit_mo"),
        // canonical M4 envelope shape per the M5-locked contract.
        ("ai", "target_scored"),
        ("ai", "path_invalidated"),
    ];
    for (cat, ty) in pairs {
        let raw = event_schema_for(cat, ty).unwrap_or_else(|| panic!("no schema for {cat}.{ty}"));
        let v: serde_json::Value = serde_json::from_str(raw).expect("schema is json");
        let sv = v
            .pointer("/properties/schema_version/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.schema_version.const"));
        assert_eq!(
            sv, "prototype-recorder-event.v0.1",
            "{cat}.{ty} schema_version must be canonical M4 envelope literal (got {sv})"
        );
        let cat_const = v
            .pointer("/properties/category/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.category.const"));
        assert_eq!(cat_const, *cat, "{cat}.{ty} category const mismatch");
        let ty_const = v
            .pointer("/properties/event_type/const")
            .and_then(|x| x.as_str())
            .unwrap_or_else(|| panic!("{cat}.{ty} missing properties.event_type.const"));
        assert_eq!(ty_const, *ty, "{cat}.{ty} event_type const mismatch");
    }
}
