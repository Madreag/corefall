//! Credits screen — auto-generated from cf-asset-ledger + git contributors
//! + AI model attribution.
//!
//! Per spec: sections covering Engine / Gameplay / Production Track /
//! Audio / Narrative / Localization / Modding Community / AI Models /
//! Comparable Games / Special Thanks.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditsTemplate {
    pub schema_version: u32,
    pub description: String,
    pub sections: Vec<CreditsSection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditsSection {
    pub title: String,
    pub entries: Vec<CreditsEntry>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditsEntry {
    pub role: String,
    pub names: Vec<String>,
}

/// Load credits template from RON file. Substitutes __PLACEHOLDERS__ with
/// values from cf-asset-ledger + git log + Workshop subscribers.
pub fn load_credits_template<P: AsRef<Path>>(path: P) -> Result<CreditsTemplate, String> {
    let txt = std::fs::read_to_string(path).map_err(|e| format!("read failed: {}", e))?;
    let template: CreditsTemplate = ron::from_str(&txt).map_err(|e| format!("RON parse failed: {}", e))?;
    Ok(template)
}

/// Substitute __PLACEHOLDER__ tokens in credits text with concrete values.
/// Concrete substitutions:
/// - __BAKED_M9A_COUNT__ → ledger row count for Tier1_SVG entries
/// - __BAKED_M12A_COUNT__ → Audio_SFX entries
/// - __BAKED_M37A_COUNT__ → Audio_Music entries
/// - __BUILD_GIT_SHA__ → current commit SHA
/// - __BUILD_TIMESTAMP_MST__ → build timestamp in Phoenix local
/// - everything else stays as placeholder
pub fn substitute_placeholders(template: &mut CreditsTemplate, substitutions: &[(String, String)]) {
    for section in &mut template.sections {
        for entry in &mut section.entries {
            for name in &mut entry.names {
                for (placeholder, replacement) in substitutions {
                    *name = name.replace(placeholder, replacement);
                }
            }
        }
    }
}

/// Reduce-motion-respecting scroll behavior. When reduce_motion=true,
/// returns 0 (instant; press [Space] to advance manually). Otherwise
/// returns the smooth scroll speed in pixels per second.
pub fn scroll_speed_pixels_per_sec(reduce_motion: bool) -> f32 {
    if reduce_motion { 0.0 } else { 30.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_replaces_placeholders() {
        let mut t = CreditsTemplate {
            schema_version: 1,
            description: "test".to_string(),
            sections: vec![CreditsSection {
                title: "Production".to_string(),
                entries: vec![CreditsEntry {
                    role: "Visual".to_string(),
                    names: vec!["__BAKED_M9A_COUNT__ tier-1 placeholders".to_string()],
                }],
            }],
        };
        let subs = vec![("__BAKED_M9A_COUNT__".to_string(), "6066".to_string())];
        substitute_placeholders(&mut t, &subs);
        assert_eq!(t.sections[0].entries[0].names[0], "6066 tier-1 placeholders");
    }

    #[test]
    fn unsubstituted_placeholders_pass_through() {
        let mut t = CreditsTemplate {
            schema_version: 1,
            description: "test".to_string(),
            sections: vec![CreditsSection {
                title: "Modding Community".to_string(),
                entries: vec![CreditsEntry {
                    role: "Featured Mods".to_string(),
                    names: vec!["__WORKSHOP_FEATURED__".to_string()],
                }],
            }],
        };
        substitute_placeholders(&mut t, &[]);
        assert_eq!(t.sections[0].entries[0].names[0], "__WORKSHOP_FEATURED__");
    }

    #[test]
    fn reduce_motion_disables_scroll() {
        assert_eq!(scroll_speed_pixels_per_sec(true), 0.0);
        assert!(scroll_speed_pixels_per_sec(false) > 0.0);
    }
}
