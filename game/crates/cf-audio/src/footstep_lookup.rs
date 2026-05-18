//! **M14A** § "cf-audio (EXTEND): per-material + per-origin footstep cue".
//!
//! `lookup_footstep_cue(material_id, origin_id) → cue_id` returns the audio
//! cue id for the planted-foot stride. Falls back to `footstep_generic` when
//! no per-material cue is authored.

/// **M14A** § "Per-material footstep cue lookup".
///
/// Resolves the per-material + per-origin footstep cue id. The cue id format
/// is `footstep_<material>_<origin_class>` where:
/// - `<material>` is one of `concrete` / `dirt` / `metal` / `loose_fill` /
///   `lava` / `acid` / `ice` / `snow` / `oil` / `mud` / `water` / `generic`.
/// - `<origin_class>` is `_organic` / `_synthetic` / `_hybrid`.
///
/// Callers that want the fallback only use [`fallback_footstep_cue`].
pub fn lookup_footstep_cue(material_id: u8, origin_id: &str) -> String {
    let material = match material_id {
        1 => "dirt",
        2 => "concrete",
        3 => "metal",
        4 => "hazard",
        5 => "loose_fill",
        6 => "repair_fill",
        7 => "anchor",
        12 => "lava",
        13 => "acid",
        14 => "ice",
        15 => "snow",
        16 => "oil",
        17 => "mud",
        18 => "water",
        _ => "generic",
    };
    let origin = match origin_id {
        "robot" | "synth" => "synthetic",
        "android" | "hybrid" => "hybrid",
        _ => "organic",
    };
    format!("footstep_{}_{}", material, origin)
}

/// Fallback cue id for missing per-material entries.
pub fn fallback_footstep_cue() -> &'static str {
    "footstep_generic"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_on_concrete_uses_organic_cue() {
        assert_eq!(lookup_footstep_cue(2, "human"), "footstep_concrete_organic");
    }

    #[test]
    fn robot_on_metal_uses_synthetic_cue() {
        assert_eq!(lookup_footstep_cue(3, "robot"), "footstep_metal_synthetic");
    }

    #[test]
    fn unknown_material_falls_back_to_generic() {
        assert_eq!(lookup_footstep_cue(99, "human"), "footstep_generic_organic");
    }
}
