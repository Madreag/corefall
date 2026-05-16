//! **M12A** § Caption bridge — audio event → caption surface per ACC-A.
//!
//! Per spec acceptance criterion:
//!
//! ```text
//! Scenario: Captions auto-show on audio event
//!   Given Settings.caption_mode=Standard
//!   When a gunshot event fires
//!   Then cf-audio plays the SFX
//!   And cf-audio::caption_bridge fires ux.captions_shown with the caption template
//!   And HUD caption strip shows "GUNSHOT — north (iron_rifle)"
//! ```
//!
//! This module owns the per-SFX caption template registry + the
//! template-resolution helper that turns `{direction}` / `{weapon_kind}`
//! placeholders into final caption text. Templates are loaded from
//! `tools/audio_gen/caption_templates.ron` at startup; cf-control's
//! engine fires the `ux.captions_shown` event with the resolved text
//! whenever a registered audio cue plays.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::positional::AudioDirection;

/// Caption severity matching `cf-control::settings::CaptionMode` band.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum CaptionSeverity {
    Critical,
    Warning,
    Info,
}

impl CaptionSeverity {
    /// Snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            CaptionSeverity::Critical => "critical",
            CaptionSeverity::Warning => "warning",
            CaptionSeverity::Info => "info",
        }
    }

    /// Parse from a (case-insensitive) wire string. Unknown → `Info`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "critical" => CaptionSeverity::Critical,
            "warning" => CaptionSeverity::Warning,
            _ => CaptionSeverity::Info,
        }
    }
}

/// One caption template entry — matches `tools/audio_gen/caption_templates.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptionTemplate {
    /// SFX id (matches `cf-asset-ledger` canonical_name).
    pub sfx_id: String,
    /// Format string with `{var}` placeholders. cf-audio::caption_bridge
    /// resolves them from the runtime caption-context dict.
    pub template: String,
    /// ACC-A severity band — drives the caption strip color + the
    /// `CaptionMode::CriticalOnly` filter.
    pub severity: CaptionSeverity,
    /// ACC-A category filter — `combat | ai | terrain | mission | system | accessibility`.
    pub categories: Vec<String>,
}

/// **M12A** § Caption registry — `sfx_id → CaptionTemplate`. Hydrated
/// from `tools/audio_gen/caption_templates.ron` at startup.
#[derive(Debug, Default, Clone)]
pub struct CaptionRegistry {
    templates: BTreeMap<String, CaptionTemplate>,
}

impl CaptionRegistry {
    /// Insert a template.
    pub fn insert(&mut self, t: CaptionTemplate) {
        self.templates.insert(t.sfx_id.clone(), t);
    }

    /// Look up a template by SFX id.
    pub fn get(&self, sfx_id: &str) -> Option<&CaptionTemplate> {
        self.templates.get(sfx_id)
    }

    /// Total registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Iterate templates in deterministic order.
    pub fn iter(&self) -> impl Iterator<Item = &CaptionTemplate> {
        self.templates.values()
    }
}

/// Resolve `{var}` placeholders in `template` from `ctx`. Unknown
/// placeholders pass through unchanged (so an `iron_rifle` weapon_kind
/// fallback is graceful when the engine forgot to provide it).
#[must_use]
pub fn resolve_template(template: &str, ctx: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len() + 32);
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        out.push_str(&remaining[..start]);
        let after_brace = &remaining[start + 1..];
        if let Some(end) = after_brace.find('}') {
            let key = &after_brace[..end];
            if let Some(value) = ctx.get(key) {
                out.push_str(value);
            } else {
                out.push('{');
                out.push_str(key);
                out.push('}');
            }
            remaining = &after_brace[end + 1..];
        } else {
            // No closing brace — pass through the whole remaining string.
            out.push('{');
            remaining = after_brace;
        }
    }
    out.push_str(remaining);
    out
}

/// **M12A** § Render the final caption line for an SFX firing event.
/// Returns `None` if no template exists for the SFX id (every M12A SFX
/// MUST have a caption per the spec; missing templates are bugs at
/// authoring time).
#[must_use]
pub fn render_caption_for_sfx(
    registry: &CaptionRegistry,
    sfx_id: &str,
    direction: AudioDirection,
    extra_vars: &BTreeMap<String, String>,
) -> Option<String> {
    let t = registry.get(sfx_id)?;
    let mut ctx = extra_vars.clone();
    ctx.insert("direction".to_string(), direction.label().to_string());
    ctx.insert("sfx_id".to_string(), sfx_id.to_string());
    Some(resolve_template(&t.template, &ctx))
}

/// **M12A** § Whether the caption should surface given the live caption
/// mode + category filter. Maps `cf-control::settings::CaptionMode` to
/// severity gates:
///
/// - `Off`      → nothing surfaces.
/// - `CriticalOnly` → only `Critical` band.
/// - `Standard` → `Critical` + `Warning`.
/// - `Expanded` → `Critical` + `Warning` + `Info`.
#[must_use]
pub fn caption_visible(
    template_severity: CaptionSeverity,
    template_categories: &[String],
    caption_mode: &str,
    enabled_categories: &[String],
) -> bool {
    let severity_ok = match (caption_mode, template_severity) {
        ("off", _) => false,
        ("critical_only", CaptionSeverity::Critical) => true,
        ("critical_only", _) => false,
        ("standard", CaptionSeverity::Critical | CaptionSeverity::Warning) => true,
        ("standard", CaptionSeverity::Info) => false,
        ("expanded", _) => true,
        _ => true,
    };
    if !severity_ok {
        return false;
    }
    if enabled_categories.is_empty() {
        return true;
    }
    template_categories
        .iter()
        .any(|c| enabled_categories.iter().any(|e| e == c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(id: &str, body: &str, sev: CaptionSeverity, cats: &[&str]) -> CaptionTemplate {
        CaptionTemplate {
            sfx_id: id.to_string(),
            template: body.to_string(),
            severity: sev,
            categories: cats.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn caption_severity_round_trip() {
        for s in [CaptionSeverity::Critical, CaptionSeverity::Warning, CaptionSeverity::Info] {
            assert_eq!(CaptionSeverity::from_str(s.as_str()), s);
        }
        assert_eq!(CaptionSeverity::from_str("nonsense"), CaptionSeverity::Info);
    }

    #[test]
    fn resolve_template_substitutes_placeholders() {
        let mut ctx = BTreeMap::new();
        ctx.insert("direction".to_string(), "north".to_string());
        ctx.insert("weapon_kind".to_string(), "iron_rifle".to_string());
        let out = resolve_template("GUNSHOT — {direction} ({weapon_kind})", &ctx);
        assert_eq!(out, "GUNSHOT — north (iron_rifle)");
    }

    #[test]
    fn resolve_template_passes_through_unknown_placeholders() {
        let ctx = BTreeMap::new();
        let out = resolve_template("BREACH — {direction}", &ctx);
        assert_eq!(out, "BREACH — {direction}");
    }

    #[test]
    fn resolve_template_handles_unmatched_brace() {
        let ctx = BTreeMap::new();
        let out = resolve_template("LITERAL { no close", &ctx);
        assert_eq!(out, "LITERAL { no close");
    }

    #[test]
    fn render_caption_for_sfx_uses_registered_template() {
        let mut reg = CaptionRegistry::default();
        reg.insert(template(
            "sfx_pistol_fire",
            "GUNSHOT — {direction} ({weapon_kind})",
            CaptionSeverity::Info,
            &["combat"],
        ));
        let mut extra = BTreeMap::new();
        extra.insert("weapon_kind".to_string(), "iron_rifle".to_string());
        let line = render_caption_for_sfx(&reg, "sfx_pistol_fire", AudioDirection::North, &extra);
        assert_eq!(line.as_deref(), Some("GUNSHOT — north (iron_rifle)"));
    }

    #[test]
    fn render_caption_returns_none_for_unknown_sfx() {
        let reg = CaptionRegistry::default();
        let line = render_caption_for_sfx(&reg, "missing_sfx", AudioDirection::Here, &BTreeMap::new());
        assert!(line.is_none());
    }

    #[test]
    fn caption_visible_respects_caption_mode_off() {
        assert!(!caption_visible(CaptionSeverity::Critical, &[], "off", &[]));
    }

    #[test]
    fn caption_visible_critical_only_filters_warning_and_info() {
        let cats = vec!["combat".to_string()];
        let enabled = vec!["combat".to_string()];
        assert!(caption_visible(CaptionSeverity::Critical, &cats, "critical_only", &enabled));
        assert!(!caption_visible(CaptionSeverity::Warning, &cats, "critical_only", &enabled));
        assert!(!caption_visible(CaptionSeverity::Info, &cats, "critical_only", &enabled));
    }

    #[test]
    fn caption_visible_standard_includes_warning() {
        let cats = vec!["combat".to_string()];
        let enabled = vec!["combat".to_string()];
        assert!(caption_visible(CaptionSeverity::Critical, &cats, "standard", &enabled));
        assert!(caption_visible(CaptionSeverity::Warning, &cats, "standard", &enabled));
        assert!(!caption_visible(CaptionSeverity::Info, &cats, "standard", &enabled));
    }

    #[test]
    fn caption_visible_expanded_includes_info() {
        let cats = vec!["combat".to_string()];
        let enabled = vec!["combat".to_string()];
        assert!(caption_visible(CaptionSeverity::Info, &cats, "expanded", &enabled));
    }

    #[test]
    fn caption_visible_respects_category_filter() {
        let cats = vec!["combat".to_string()];
        let enabled = vec!["ai".to_string()];
        // Combat-tagged template, but the player only enabled AI captions.
        assert!(!caption_visible(CaptionSeverity::Warning, &cats, "standard", &enabled));
    }

    #[test]
    fn caption_visible_empty_enabled_means_no_category_filter() {
        let cats = vec!["combat".to_string()];
        // Empty enabled list = "show all categories"; verbose but matches the
        // caption_categories empty-set semantics.
        assert!(caption_visible(CaptionSeverity::Warning, &cats, "standard", &[]));
    }
}
