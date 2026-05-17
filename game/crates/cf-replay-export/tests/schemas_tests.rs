//! M10B closure-feature schema gate.
//!
//! Per VAL-M10B-007 the two cf-replay-export schemas
//! (`camera_script.schema.json` + `preset_registry.schema.json`)
//! exist + parse cleanly as JSON Schema (draft-07+). This test
//! file is the closure-feature evidence the orchestrator points
//! `cargo test -p cf-replay-export schemas` at.

use std::path::PathBuf;

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

fn parse_schema(name: &str) -> serde_json::Value {
    let path = schemas_dir().join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn assert_required_keys(v: &serde_json::Value, name: &str) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{name} root must be JSON object"));
    for key in ["$id", "title", "type", "properties"] {
        assert!(obj.contains_key(key), "{name} missing required key `{key}`");
    }
}

/// VAL-M10B-007: both cf-replay-export schemas exist on disk.
#[test]
fn schemas_present_on_disk() {
    for name in ["camera_script.schema.json", "preset_registry.schema.json"] {
        let path = schemas_dir().join(name);
        assert!(
            path.is_file(),
            "schema {name} missing at {}",
            path.display()
        );
    }
}

/// VAL-M10B-007: camera_script.schema.json parses + declares required
/// keys + references the 4-variant CameraKind enum.
#[test]
fn schemas_camera_script_parses_and_declares_kind_enum() {
    let v = parse_schema("camera_script.schema.json");
    assert_required_keys(&v, "camera_script.schema.json");
    // tracks must reference the track definition via $defs.
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let tracks = props
        .get("tracks")
        .and_then(|r| r.as_object())
        .expect("tracks property exists");
    let items = tracks
        .get("items")
        .and_then(|r| r.as_object())
        .expect("tracks.items exists");
    let ref_val = items
        .get("$ref")
        .and_then(|r| r.as_str())
        .expect("tracks.items has $ref");
    assert!(
        ref_val.contains("track"),
        "tracks.items.$ref must point at the track definition; got {ref_val}"
    );
    // The track definition must declare the 4-variant kind enum.
    let defs = v
        .get("$defs")
        .and_then(|r| r.as_object())
        .expect("$defs object");
    let track = defs
        .get("track")
        .and_then(|r| r.as_object())
        .expect("$defs.track");
    let track_props = track
        .get("properties")
        .and_then(|r| r.as_object())
        .expect("track properties");
    let kind = track_props
        .get("kind")
        .and_then(|r| r.as_object())
        .expect("track.kind");
    let enum_vals = kind
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("track.kind.enum");
    let strings: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    for required in ["free_cam", "follow_player", "objective_cam", "kill_cam"] {
        assert!(
            strings.contains(&required),
            "camera_script.schema.json track.kind.enum missing `{required}`"
        );
    }
}

/// VAL-M10B-007: preset_registry.schema.json parses + declares the 6
/// required preset fields + the 5-variant preset name enum.
#[test]
fn schemas_preset_registry_declares_required_fields_and_name_enum() {
    let v = parse_schema("preset_registry.schema.json");
    assert_required_keys(&v, "preset_registry.schema.json");
    // 6 required fields (mirrors PRESET_REQUIRED_FIELDS in the
    // preset_registry crate module).
    let required = v
        .get("required")
        .and_then(|r| r.as_array())
        .expect("preset_registry required[] exists");
    let names: Vec<&str> = required.iter().filter_map(|v| v.as_str()).collect();
    for field in [
        "name",
        "resolution",
        "fps",
        "codec",
        "audio_codec",
        "target_bitrate_kbps",
        "container",
    ] {
        assert!(
            names.contains(&field),
            "preset_registry.schema.json required[] missing `{field}`; got {names:?}"
        );
    }
    // name.enum covers the 5 DECLARED_PRESETS.
    let props = v.get("properties").and_then(|p| p.as_object()).unwrap();
    let name_prop = props
        .get("name")
        .and_then(|r| r.as_object())
        .expect("name property");
    let enum_vals = name_prop
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("name.enum exists");
    let preset_names: Vec<&str> = enum_vals.iter().filter_map(|v| v.as_str()).collect();
    for required_name in cf_replay_export::preset_registry::DECLARED_PRESETS {
        assert!(
            preset_names.contains(&required_name),
            "preset_registry.schema.json name.enum missing `{required_name}`"
        );
    }
    // codec.enum covers the 4 PresetCodec variants.
    let codec = props
        .get("codec")
        .and_then(|r| r.as_object())
        .expect("codec property");
    let codec_enum = codec
        .get("enum")
        .and_then(|e| e.as_array())
        .expect("codec.enum exists");
    let codec_strings: Vec<&str> = codec_enum.iter().filter_map(|v| v.as_str()).collect();
    for required_codec in ["h264", "h265", "av1", "ffv1"] {
        assert!(
            codec_strings.contains(&required_codec),
            "preset_registry.schema.json codec.enum missing `{required_codec}`"
        );
    }
}
