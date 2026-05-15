//! **M9A § "Per-faction palette swap"**: runtime palette swap helper for the
//! Tier-1 SVG asset pipeline.
//!
//! cf-render-2d caches a small set of palette tables loaded from the M9A
//! palette JSONs. A `PaletteSwap` is applied by re-coloring the loaded
//! texture's pixel buffer at startup (or on faction-change) by matching
//! source-palette hex codes and emitting destination-palette hex codes.
//! This is the cosmetic-only knob the spec calls out:
//!
//! > runtime palette swap for material overlay modes + faction color-shift
//!
//! It is NOT a re-bake — the SVG sources stay untouched on disk. Per spec
//! "Per-faction palette swap available at runtime (no re-bake needed for
//! color variations)."
//!
//! Cosmetic-only contract: this module never reads or writes engine state,
//! never produces sim-effecting RNG, never depends on tick number. The output
//! is a deterministic function of (palette_id, pixel) → pixel.

use std::collections::HashMap;

use bevy::prelude::*;

/// One palette entry: a stable `role` token (`primary`, `accent`, `metal`,
/// etc.) and the hex RGB it maps to in the current context.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PaletteEntry {
    pub role: String,
    pub rgb: [u8; 3],
}

/// One palette loaded from a JSON file under `tools/asset_gen/palettes/`.
#[derive(Debug, Clone, Default)]
pub struct Palette {
    pub palette_id: String,
    pub entries: Vec<PaletteEntry>,
}

impl Palette {
    pub fn color_for_role(&self, role: &str) -> Option<[u8; 3]> {
        self.entries.iter().find(|e| e.role == role).map(|e| e.rgb)
    }
}

/// A palette swap table: maps "from" hex (the placeholder default) to "to"
/// hex (the target faction or material variant).
#[derive(Debug, Clone, Default)]
pub struct PaletteSwap {
    pub from_palette_id: String,
    pub to_palette_id: String,
    pub map: HashMap<[u8; 3], [u8; 3]>,
}

impl PaletteSwap {
    /// Apply the swap to a single RGB pixel. If the pixel doesn't match any
    /// `from` entry, it passes through unchanged.
    #[inline]
    pub fn apply_pixel(&self, rgb: [u8; 3]) -> [u8; 3] {
        self.map.get(&rgb).copied().unwrap_or(rgb)
    }

    /// Apply the swap to an RGBA pixel buffer in-place. The alpha channel is
    /// preserved verbatim.
    pub fn apply_rgba_buffer(&self, buffer: &mut [u8]) {
        for chunk in buffer.chunks_exact_mut(4) {
            let rgb = [chunk[0], chunk[1], chunk[2]];
            let swapped = self.apply_pixel(rgb);
            chunk[0] = swapped[0];
            chunk[1] = swapped[1];
            chunk[2] = swapped[2];
        }
    }
}

/// Build a palette swap from two registered palettes by role-matching their
/// entries. Roles that appear in both palettes are wired across; roles that
/// appear in only one side are ignored.
pub fn build_role_swap(from: &Palette, to: &Palette) -> PaletteSwap {
    let mut map: HashMap<[u8; 3], [u8; 3]> = HashMap::new();
    for entry in &from.entries {
        if let Some(target_rgb) = to.color_for_role(&entry.role) {
            map.insert(entry.rgb, target_rgb);
        }
    }
    PaletteSwap {
        from_palette_id: from.palette_id.clone(),
        to_palette_id: to.palette_id.clone(),
        map,
    }
}

/// Convert a hex string ("#a5b6c7" or "a5b6c7") to RGB. Returns `None` on
/// any parse failure so the loader can skip malformed lines without
/// panicking.
pub fn parse_hex_rgb(hex: &str) -> Option<[u8; 3]> {
    let s = hex.trim_start_matches('#');
    let s = if s.len() == 3 {
        s.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        s.to_string()
    };
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Registry resource: every palette loaded at startup keyed by palette_id.
#[derive(Resource, Default, Debug, Clone)]
pub struct PaletteRegistry {
    pub palettes: HashMap<String, Palette>,
    pub default_faction: String,
}

impl PaletteRegistry {
    pub fn register(&mut self, palette: Palette) {
        self.palettes.insert(palette.palette_id.clone(), palette);
    }

    pub fn get(&self, palette_id: &str) -> Option<&Palette> {
        self.palettes.get(palette_id)
    }

    /// Build a palette swap from `from_id` to `to_id`. Both must be
    /// registered; returns `None` if either is missing.
    pub fn build_swap(&self, from_id: &str, to_id: &str) -> Option<PaletteSwap> {
        let from = self.palettes.get(from_id)?;
        let to = self.palettes.get(to_id)?;
        Some(build_role_swap(from, to))
    }
}

/// The 5-mode M3 material overlay uses these tints; engine selects one at
/// runtime via `OverlayModeState`. Values match M3 spec § "5-mode overlay"
/// and are duplicated here so the asset pipeline knows the canonical hex
/// values used when baking the material-overlay tinted variants.
pub const OVERLAY_TINT_INTEGRITY: [u8; 3] = [0x5b, 0xd0, 0x78];
pub const OVERLAY_TINT_PATHABILITY: [u8; 3] = [0x3a, 0x8c, 0xff];
pub const OVERLAY_TINT_MOBILITY: [u8; 3] = [0xda, 0xb4, 0x38];
pub const OVERLAY_TINT_HAZARD: [u8; 3] = [0xc9, 0x30, 0x30];
pub const OVERLAY_TINT_BUILD_REPAIR: [u8; 3] = [0x8a, 0x78, 0xff];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_strings() {
        assert_eq!(parse_hex_rgb("#a5b6c7"), Some([0xa5, 0xb6, 0xc7]));
        assert_eq!(parse_hex_rgb("a5b6c7"), Some([0xa5, 0xb6, 0xc7]));
        assert_eq!(parse_hex_rgb("#fff"), Some([0xff, 0xff, 0xff]));
        assert_eq!(parse_hex_rgb(""), None);
        assert_eq!(parse_hex_rgb("not-a-hex"), None);
    }

    #[test]
    fn build_role_swap_maps_overlapping_roles() {
        let from = Palette {
            palette_id: "hostile_corp".to_string(),
            entries: vec![
                PaletteEntry {
                    role: "primary".to_string(),
                    rgb: [0x2a, 0x2f, 0x3a],
                },
                PaletteEntry {
                    role: "accent".to_string(),
                    rgb: [0xd9, 0x24, 0x34],
                },
            ],
        };
        let to = Palette {
            palette_id: "drone_collective".to_string(),
            entries: vec![
                PaletteEntry {
                    role: "primary".to_string(),
                    rgb: [0x1c, 0x22, 0x30],
                },
                PaletteEntry {
                    role: "accent".to_string(),
                    rgb: [0x3a, 0x8c, 0xff],
                },
                PaletteEntry {
                    role: "glow".to_string(),
                    rgb: [0xc0, 0xe8, 0xff],
                }, // ignored: not in from
            ],
        };
        let swap = build_role_swap(&from, &to);
        assert_eq!(swap.from_palette_id, "hostile_corp");
        assert_eq!(swap.to_palette_id, "drone_collective");
        assert_eq!(swap.apply_pixel([0x2a, 0x2f, 0x3a]), [0x1c, 0x22, 0x30]);
        assert_eq!(swap.apply_pixel([0xd9, 0x24, 0x34]), [0x3a, 0x8c, 0xff]);
        // Unknown pixels pass through.
        assert_eq!(swap.apply_pixel([0x00, 0x00, 0x00]), [0x00, 0x00, 0x00]);
    }

    #[test]
    fn apply_rgba_buffer_preserves_alpha_and_remaps_rgb() {
        let swap = PaletteSwap {
            from_palette_id: "from".to_string(),
            to_palette_id: "to".to_string(),
            map: HashMap::from([([10, 20, 30], [99, 88, 77])]),
        };
        let mut buf = vec![
            10, 20, 30, 255, // pixel 0 — should remap
            5, 5, 5, 100, // pixel 1 — should pass through
            10, 20, 30, 0, // pixel 2 — should remap, alpha kept
        ];
        swap.apply_rgba_buffer(&mut buf);
        assert_eq!(buf[0..4], [99, 88, 77, 255]);
        assert_eq!(buf[4..8], [5, 5, 5, 100]);
        assert_eq!(buf[8..12], [99, 88, 77, 0]);
    }

    #[test]
    fn palette_color_for_role_returns_none_when_missing() {
        let p = Palette {
            palette_id: "x".to_string(),
            entries: vec![PaletteEntry {
                role: "primary".to_string(),
                rgb: [1, 2, 3],
            }],
        };
        assert_eq!(p.color_for_role("primary"), Some([1, 2, 3]));
        assert!(p.color_for_role("accent").is_none());
    }

    #[test]
    fn registry_build_swap_returns_none_for_unknown() {
        let mut reg = PaletteRegistry::default();
        reg.register(Palette {
            palette_id: "a".to_string(),
            entries: vec![PaletteEntry {
                role: "primary".to_string(),
                rgb: [1, 1, 1],
            }],
        });
        assert!(reg.build_swap("a", "b").is_none());
        assert!(reg.build_swap("b", "a").is_none());
    }

    #[test]
    fn overlay_tints_are_distinct() {
        let tints = [
            OVERLAY_TINT_INTEGRITY,
            OVERLAY_TINT_PATHABILITY,
            OVERLAY_TINT_MOBILITY,
            OVERLAY_TINT_HAZARD,
            OVERLAY_TINT_BUILD_REPAIR,
        ];
        for i in 0..tints.len() {
            for j in (i + 1)..tints.len() {
                assert_ne!(tints[i], tints[j], "tint {i} == tint {j}");
            }
        }
    }
}
