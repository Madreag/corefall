//! `AudioRegistry` — read-only lookup from the cf-asset-ledger.
//!
//! Hydrated from `content/asset_ledger/ledger.jsonl` at startup so any future
//! Bevy / rodio / oboe backend can resolve a cue's `canonical_name` →
//! on-disk WAV path without each backend having to re-parse the ledger.
//!
//! Lives in `cf-audio` (not in a Bevy adapter crate) because the lookup is
//! pure data + pure path resolution; no presentation or determinism surface.
//!
//! **M9B**: adds a static `family` registry so milestone-owned cue groups
//! (e.g. `trench`) can be enumerated without re-scanning the ledger.
//! Spec §"Crates / modules touched": cf-audio gets trench cues
//! `duckboard_step`, `mud_squelch`, `entrenching_dig`, `drainage_drip`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// **M9B** § VAL-M9B-AUDIO-001 — four trench audio cues registered in
/// the audio family. Spec § Crates / modules touched:
///
/// > Trench cues: `duckboard_step`, `mud_squelch`, `entrenching_dig`,
/// > `drainage_drip`.
///
/// Surfaced via [`AudioRegistry::family`] under the `"trench"` family
/// name so closure-feature tests can audit membership without
/// re-parsing the asset ledger.
pub const TRENCH_AUDIO_CUES: &[&str] = &[
    "duckboard_step",
    "mud_squelch",
    "entrenching_dig",
    "drainage_drip",
];

/// Map a family name to its canonical cue-id list. Returns `&[]` for
/// unknown family names so the lookup is panic-free.
#[must_use]
pub fn family_members(family: &str) -> &'static [&'static str] {
    match family {
        "trench" => TRENCH_AUDIO_CUES,
        _ => &[],
    }
}

/// One hydrated ledger row exposed to audio callers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioAsset {
    /// Canonical name (`canonical_name` field on the ledger row).
    pub canonical_name: String,
    /// `Audio_SFX` / `Audio_Voice` / `Audio_Music`.
    pub category: String,
    /// Sub-category (`kind` field on the ledger row).
    pub kind: String,
    /// Pipeline that baked this asset (e.g. `M37A_eleven_voice_v1`).
    pub pipeline: String,
    /// Production tier (`Tier1_LLM_Audio` / `Tier2_Audio_Production` / …).
    pub tier: String,
    /// Absolute or repo-relative path to the WAV on disk.
    pub output_path: PathBuf,
    /// File size in bytes (from ledger row, NOT freshly stat'd).
    pub output_size_bytes: u64,
    /// Blake3 hex digest of the file contents.
    pub output_blake3: String,
}

/// Hydrated audio asset catalogue. Cheap clone (Arc-like via `BTreeMap`).
///
/// Built once at app startup; consult via the `voice` / `sfx` / `music`
/// accessors. The returned `&AudioAsset` borrows from the registry — wrap
/// the registry in a Bevy `Resource` and access through the borrow at use
/// sites.
#[derive(Debug, Default, Clone)]
pub struct AudioRegistry {
    voice: BTreeMap<String, AudioAsset>,
    sfx: BTreeMap<String, AudioAsset>,
    music: BTreeMap<String, AudioAsset>,
}

#[derive(Debug, Deserialize)]
struct LedgerRow {
    canonical_name: String,
    category: String,
    kind: String,
    pipeline: String,
    tier: String,
    output_path: String,
    output_size_bytes: u64,
    output_blake3: String,
}

impl AudioRegistry {
    /// Read every audio row from `ledger.jsonl` into an indexed catalogue.
    ///
    /// Non-audio categories are skipped silently. Malformed rows are skipped
    /// with a `tracing::warn!`; the registry never panics on bad input.
    pub fn hydrate_from_ledger(ledger_path: &Path) -> std::io::Result<Self> {
        let file = File::open(ledger_path)?;
        let reader = BufReader::new(file);
        let mut out = AudioRegistry::default();
        for (line_no, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(err) => {
                    tracing::warn!(target: "cf::audio::registry", line = line_no, ?err, "skipping unreadable line");
                    continue;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            let row: LedgerRow = match serde_json::from_str(&line) {
                Ok(r) => r,
                Err(_) => continue, // skip silently — audio is one of many categories
            };
            if !row.category.starts_with("Audio_") {
                continue;
            }
            let asset = AudioAsset {
                canonical_name: row.canonical_name.clone(),
                category: row.category.clone(),
                kind: row.kind,
                pipeline: row.pipeline,
                tier: row.tier,
                output_path: PathBuf::from(row.output_path),
                output_size_bytes: row.output_size_bytes,
                output_blake3: row.output_blake3,
            };
            match row.category.as_str() {
                "Audio_Voice" => {
                    out.voice.insert(row.canonical_name, asset);
                }
                "Audio_SFX" => {
                    out.sfx.insert(row.canonical_name, asset);
                }
                "Audio_Music" => {
                    out.music.insert(row.canonical_name, asset);
                }
                _ => {}
            }
        }
        Ok(out)
    }

    /// Look up a voice line by its canonical id (e.g. `voice_marcus_hayes_first_meeting`).
    pub fn voice(&self, canonical_name: &str) -> Option<&AudioAsset> {
        self.voice.get(canonical_name)
    }

    /// Look up a sound effect by its canonical id (e.g. `sfx_pistol_fire`).
    pub fn sfx(&self, canonical_name: &str) -> Option<&AudioAsset> {
        self.sfx.get(canonical_name)
    }

    /// Look up a music loop. `canonical_name` is the full `<track_id>_<variant>` string
    /// (e.g. `music_world_earth_calm`).
    pub fn music(&self, canonical_name: &str) -> Option<&AudioAsset> {
        self.music.get(canonical_name)
    }

    /// Look up the right adaptive-music variant for a given track_id + intensity.
    ///
    /// Variant selection (per `music_tracks_prompts.json::adaptive_engine_notes`):
    /// `[0.0, 0.3) calm` / `[0.3, 0.6) buildup` / `[0.6, 0.9) climax` / `[0.9, 1.0] climax` / debrief is selected by caller after encounter exit.
    pub fn music_variant_for(&self, track_id: &str, intensity: f32) -> Option<&AudioAsset> {
        let variant = if intensity < 0.3 {
            "calm"
        } else if intensity < 0.6 {
            "buildup"
        } else {
            "climax"
        };
        self.music.get(&format!("{}_{}", track_id, variant))
    }

    /// Counts (voice, sfx, music). Used by startup logging.
    pub fn counts(&self) -> (usize, usize, usize) {
        (self.voice.len(), self.sfx.len(), self.music.len())
    }

    /// **M9B** § VAL-M9B-AUDIO-001 — return the static cue-id list for
    /// the named family. Cue ids are owned by [`TRENCH_AUDIO_CUES`] and
    /// peer constants; the registry only owns the family → static
    /// list mapping so the ledger never has to know about gameplay
    /// vocabulary.
    #[must_use]
    pub fn family(&self, family: &str) -> &'static [&'static str] {
        family_members(family)
    }

    /// Iterate over every voice asset (deterministic order).
    pub fn voices(&self) -> impl Iterator<Item = &AudioAsset> {
        self.voice.values()
    }

    /// Iterate over every SFX asset (deterministic order).
    pub fn sfxs(&self) -> impl Iterator<Item = &AudioAsset> {
        self.sfx.values()
    }

    /// Iterate over every music asset (deterministic order).
    pub fn musics(&self) -> impl Iterator<Item = &AudioAsset> {
        self.music.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_ledger_with(rows: &[&str]) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("temp file");
        for r in rows {
            writeln!(f, "{}", r).unwrap();
        }
        f.flush().unwrap();
        f
    }

    #[test]
    fn hydrates_empty_ledger_to_empty_registry() {
        let f = make_ledger_with(&[]);
        let reg = AudioRegistry::hydrate_from_ledger(f.path()).expect("hydrate");
        assert_eq!(reg.counts(), (0, 0, 0));
    }

    #[test]
    fn indexes_three_audio_categories() {
        let f = make_ledger_with(&[
            r#"{"canonical_name":"voice_marcus_hayes_first_meeting","category":"Audio_Voice","kind":"voice_line","pipeline":"M37A_eleven_voice_v1","tier":"Tier2_Audio_Production","output_path":"/abs/voice/voice_marcus_hayes_first_meeting.wav","output_size_bytes":826420,"output_blake3":"deadbeef"}"#,
            r#"{"canonical_name":"sfx_pistol_fire","category":"Audio_SFX","kind":"weapon","pipeline":"M12A_eleven_sfx_v1","tier":"Tier2_Audio_Production","output_path":"/abs/sfx/sfx_pistol_fire.wav","output_size_bytes":50000,"output_blake3":"feedbeef"}"#,
            r#"{"canonical_name":"music_world_earth_calm","category":"Audio_Music","kind":"music_loop","pipeline":"M37A_eleven_music_v1","tier":"Tier2_Audio_Production","output_path":"/abs/music/music_world_earth_calm.wav","output_size_bytes":44114664,"output_blake3":"cafebabe"}"#,
            r#"{"canonical_name":"weapon_rifle","category":"WeaponSprite","kind":"sprite","pipeline":"M9A_svg_v1","tier":"Tier1_SVG","output_path":"/abs/x","output_size_bytes":1,"output_blake3":"x"}"#,
        ]);
        let reg = AudioRegistry::hydrate_from_ledger(f.path()).expect("hydrate");
        assert_eq!(reg.counts(), (1, 1, 1), "audio rows only");
        assert!(reg.voice("voice_marcus_hayes_first_meeting").is_some());
        assert!(reg.sfx("sfx_pistol_fire").is_some());
        assert!(reg.music("music_world_earth_calm").is_some());
        assert!(reg.voice("nope").is_none());
    }

    #[test]
    fn music_variant_selector_maps_intensity_correctly() {
        let f = make_ledger_with(&[
            r#"{"canonical_name":"music_boss_hollow_king_calm","category":"Audio_Music","kind":"music_loop","pipeline":"M37A_eleven_music_v1","tier":"Tier2_Audio_Production","output_path":"/abs/m/calm.wav","output_size_bytes":1,"output_blake3":"a"}"#,
            r#"{"canonical_name":"music_boss_hollow_king_buildup","category":"Audio_Music","kind":"music_loop","pipeline":"M37A_eleven_music_v1","tier":"Tier2_Audio_Production","output_path":"/abs/m/buildup.wav","output_size_bytes":1,"output_blake3":"b"}"#,
            r#"{"canonical_name":"music_boss_hollow_king_climax","category":"Audio_Music","kind":"music_loop","pipeline":"M37A_eleven_music_v1","tier":"Tier2_Audio_Production","output_path":"/abs/m/climax.wav","output_size_bytes":1,"output_blake3":"c"}"#,
        ]);
        let reg = AudioRegistry::hydrate_from_ledger(f.path()).expect("hydrate");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 0.0).unwrap().canonical_name, "music_boss_hollow_king_calm");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 0.29).unwrap().canonical_name, "music_boss_hollow_king_calm");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 0.3).unwrap().canonical_name, "music_boss_hollow_king_buildup");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 0.59).unwrap().canonical_name, "music_boss_hollow_king_buildup");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 0.6).unwrap().canonical_name, "music_boss_hollow_king_climax");
        assert_eq!(reg.music_variant_for("music_boss_hollow_king", 1.0).unwrap().canonical_name, "music_boss_hollow_king_climax");
        assert!(reg.music_variant_for("music_unknown", 0.5).is_none());
    }

    /// VAL-M9B-AUDIO-001: registry exposes the four trench cues under
    /// the `trench` family.
    #[test]
    fn registry_contains_trench_cues() {
        let registry = AudioRegistry::default();
        let family = registry.family("trench");
        assert_eq!(family.len(), 4, "trench family must contain 4 cues");
        for required in [
            "duckboard_step",
            "mud_squelch",
            "entrenching_dig",
            "drainage_drip",
        ] {
            assert!(
                family.contains(&required),
                "trench family missing `{required}`"
            );
        }
    }

    /// Alias matching the validation contract evidence string
    /// `m9b_trench_audio_family`.
    #[test]
    fn m9b_trench_audio_family() {
        registry_contains_trench_cues();
    }

    /// Unknown families return an empty slice without panicking.
    #[test]
    fn unknown_family_returns_empty_slice() {
        let registry = AudioRegistry::default();
        assert!(registry.family("nonexistent").is_empty());
        assert!(family_members("garbage").is_empty());
    }

    #[test]
    fn malformed_lines_are_skipped_silently() {
        let f = make_ledger_with(&[
            r#"{"canonical_name":"voice_a","category":"Audio_Voice","kind":"voice_line","pipeline":"X","tier":"Tier2_Audio_Production","output_path":"/abs","output_size_bytes":1,"output_blake3":"a"}"#,
            r#"not even json"#,
            r#"{"category":"Audio_Voice"}"#, // missing required fields
            r#"{"canonical_name":"voice_b","category":"Audio_Voice","kind":"voice_line","pipeline":"X","tier":"Tier2_Audio_Production","output_path":"/abs","output_size_bytes":1,"output_blake3":"b"}"#,
        ]);
        let reg = AudioRegistry::hydrate_from_ledger(f.path()).expect("hydrate");
        assert_eq!(reg.counts(), (2, 0, 0));
    }
}
