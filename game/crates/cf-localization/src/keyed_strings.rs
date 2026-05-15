//! Keyed strings table — language code + key → string map.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::icu_format::format_with_args;

/// Canonical English language code per ISO 639-1.
pub const EN_LANGUAGE_CODE: &str = "en";

/// Compile-time bundled English strings table. The bake reads the JSON file
/// at `game/content/localization/en.json` so cf-app can render a correct
/// HUD on a fresh checkout without I/O.
pub const EN_TABLE_BYTES: &[u8] = include_bytes!("../../../content/localization/en.json");

/// One JSON-formatted localization table for a single language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalizationTable {
    /// ISO-639-1 (or country-suffixed) language code.
    pub language: String,
    /// Key → string mapping. Keys are stable cf-localization identifiers
    /// like `hud.compass.cardinal_n` or `settings.controls.reload_label`.
    pub entries: BTreeMap<String, String>,
}

/// Load failure modes.
#[derive(Debug, Error)]
pub enum LocalizationLoadError {
    /// JSON parse failure.
    #[error("localization json parse failed: {0}")]
    Parse(#[from] serde_json::Error),
    /// Required `entries` map is empty.
    #[error("localization table has no entries")]
    Empty,
}

impl LocalizationTable {
    /// Construct an empty table for the given language.
    pub fn new(language: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            entries: BTreeMap::new(),
        }
    }

    /// Load a JSON-encoded table. The JSON shape mirrors the struct
    /// (`{"language": "en", "entries": {...}}`) and rejects an empty
    /// `entries` map (the launch baseline must hit ≥ 500 keys per spec).
    pub fn load_from_json(input: &str) -> Result<Self, LocalizationLoadError> {
        let parsed: LocalizationTable = serde_json::from_str(input)?;
        if parsed.entries.is_empty() {
            return Err(LocalizationLoadError::Empty);
        }
        Ok(parsed)
    }

    /// Bundled English baseline (loaded from `EN_TABLE_BYTES`).
    pub fn english_baseline() -> Result<Self, LocalizationLoadError> {
        let txt = std::str::from_utf8(EN_TABLE_BYTES)
            .map_err(|e| LocalizationLoadError::Parse(serde_json::from_str::<serde_json::Value>(&e.to_string()).unwrap_err()))?;
        Self::load_from_json(txt)
    }

    /// Number of entries in the table.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Strict lookup — returns None when the key is missing.
    pub fn lookup(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    /// Lenient lookup — falls back to the key itself when missing so the
    /// HUD never shows blank strings on a new key.
    pub fn lookup_or_key<'a>(&'a self, key: &'a str) -> &'a str {
        self.lookup(key).unwrap_or(key)
    }

    /// Look up + substitute placeholders. Supports `{placeholder}`
    /// substitution (no escaping; brace literals are not honored at M8).
    /// Plural handling is delegated to `format_with_args` which detects
    /// the ICU `{count, plural, one {...} other {...}}` form.
    pub fn format(&self, key: &str, args: &[(&str, &str)]) -> String {
        let template = self.lookup_or_key(key);
        format_with_args(template, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_table_round_trips() {
        let mut t = LocalizationTable::new("en");
        assert!(t.is_empty());
        t.entries.insert("hello".into(), "Hello".into());
        assert_eq!(t.len(), 1);
        assert_eq!(t.lookup("hello"), Some("Hello"));
        assert_eq!(t.lookup("missing"), None);
    }

    #[test]
    fn lookup_or_key_falls_back() {
        let t = LocalizationTable::new("en");
        assert_eq!(t.lookup_or_key("hud.banners.alert"), "hud.banners.alert");
    }

    #[test]
    fn load_from_json_rejects_empty() {
        let err = LocalizationTable::load_from_json("{\"language\":\"en\",\"entries\":{}}");
        assert!(matches!(err, Err(LocalizationLoadError::Empty)));
    }

    #[test]
    fn load_from_json_round_trips() {
        let json = "{\"language\":\"en\",\"entries\":{\"a\":\"A\",\"b\":\"B\"}}";
        let t = LocalizationTable::load_from_json(json).unwrap();
        assert_eq!(t.language, "en");
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn format_substitutes_placeholders() {
        let mut t = LocalizationTable::new("en");
        t.entries.insert("hud.greeting".into(), "Hello, {name}".into());
        let s = t.format("hud.greeting", &[("name", "World")]);
        assert_eq!(s, "Hello, World");
    }

    #[test]
    fn english_baseline_loads_with_at_least_500_keys() {
        let t = LocalizationTable::english_baseline().expect("english baseline must load");
        assert_eq!(t.language, EN_LANGUAGE_CODE);
        assert!(
            t.len() >= 500,
            "english baseline must hit ~500 keys per M8 spec; got {}",
            t.len()
        );
    }
}
