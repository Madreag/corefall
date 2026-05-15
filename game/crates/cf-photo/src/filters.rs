//! Photo mode filters — sepia / B&W / color grading / cyberpunk neon
//! per spec § Photo mode (4 launch filters; M33+ codex extends).
//!
//! Each filter takes a flat `[r, g, b, r, g, b, ...]` byte buffer and
//! produces a new buffer of the same length. Operations are deterministic
//! per pixel so cf-replay can re-bake the same shot byte-identically.

use serde::{Deserialize, Serialize};

/// One of the 5 launch filter states (None + 4 named per spec).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PhotoFilter {
    /// No filter applied.
    #[default]
    None,
    /// Warm sepia tone.
    Sepia,
    /// Greyscale.
    BlackAndWhite,
    /// Cool color grade (push toward blue/teal highlights).
    ColorGrade,
    /// High-saturation cyberpunk neon (red+blue boost; green crush).
    CyberpunkNeon,
}

impl PhotoFilter {
    /// Every variant in declaration order.
    pub const ALL: [PhotoFilter; 5] = [
        PhotoFilter::None,
        PhotoFilter::Sepia,
        PhotoFilter::BlackAndWhite,
        PhotoFilter::ColorGrade,
        PhotoFilter::CyberpunkNeon,
    ];

    /// Canonical snake_case identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            PhotoFilter::None => "none",
            PhotoFilter::Sepia => "sepia",
            PhotoFilter::BlackAndWhite => "black_and_white",
            PhotoFilter::ColorGrade => "color_grade",
            PhotoFilter::CyberpunkNeon => "cyberpunk_neon",
        }
    }

    /// Parse from the cfctl wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<PhotoFilter> {
        Some(match value {
            "none" => PhotoFilter::None,
            "sepia" => PhotoFilter::Sepia,
            "black_and_white" => PhotoFilter::BlackAndWhite,
            "color_grade" => PhotoFilter::ColorGrade,
            "cyberpunk_neon" => PhotoFilter::CyberpunkNeon,
            _ => return None,
        })
    }

    /// Cycle to the next filter in declaration order.
    pub fn next(self) -> PhotoFilter {
        let i = match self {
            PhotoFilter::None => 0,
            PhotoFilter::Sepia => 1,
            PhotoFilter::BlackAndWhite => 2,
            PhotoFilter::ColorGrade => 3,
            PhotoFilter::CyberpunkNeon => 4,
        };
        PhotoFilter::ALL[(i + 1) % PhotoFilter::ALL.len()]
    }
}

/// Apply a filter to an RGB byte buffer in-place. The buffer length MUST
/// be a multiple of 3; trailing bytes are left untouched.
pub fn apply_filter(buf: &mut [u8], filter: PhotoFilter) {
    let chunks = buf.chunks_exact_mut(3);
    for chunk in chunks {
        let r = chunk[0] as f32;
        let g = chunk[1] as f32;
        let b = chunk[2] as f32;
        let (nr, ng, nb) = match filter {
            PhotoFilter::None => (r, g, b),
            PhotoFilter::Sepia => sepia(r, g, b),
            PhotoFilter::BlackAndWhite => grayscale(r, g, b),
            PhotoFilter::ColorGrade => color_grade(r, g, b),
            PhotoFilter::CyberpunkNeon => cyberpunk_neon(r, g, b),
        };
        chunk[0] = nr.clamp(0.0, 255.0) as u8;
        chunk[1] = ng.clamp(0.0, 255.0) as u8;
        chunk[2] = nb.clamp(0.0, 255.0) as u8;
    }
}

fn sepia(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    (
        0.393 * r + 0.769 * g + 0.189 * b,
        0.349 * r + 0.686 * g + 0.168 * b,
        0.272 * r + 0.534 * g + 0.131 * b,
    )
}

fn grayscale(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    (lum, lum, lum)
}

fn color_grade(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let r2 = r * 0.95;
    let g2 = g * 1.0;
    let b2 = (b * 1.15).min(255.0);
    (r2, g2, b2)
}

fn cyberpunk_neon(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let r2 = (r * 1.4).min(255.0);
    let g2 = g * 0.7;
    let b2 = (b * 1.4).min(255.0);
    (r2, g2, b2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_str() {
        for f in PhotoFilter::ALL {
            assert_eq!(PhotoFilter::from_str(f.as_str()), Some(f));
        }
    }

    #[test]
    fn next_walks_all_variants() {
        let mut f = PhotoFilter::None;
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..PhotoFilter::ALL.len() {
            seen.insert(f.as_str());
            f = f.next();
        }
        assert_eq!(seen.len(), PhotoFilter::ALL.len());
    }

    #[test]
    fn none_is_identity() {
        let mut buf = vec![10, 20, 30, 40, 50, 60];
        let snapshot = buf.clone();
        apply_filter(&mut buf, PhotoFilter::None);
        assert_eq!(buf, snapshot);
    }

    #[test]
    fn grayscale_collapses_channels() {
        let mut buf = vec![255, 0, 0];
        apply_filter(&mut buf, PhotoFilter::BlackAndWhite);
        assert_eq!(buf[0], buf[1]);
        assert_eq!(buf[1], buf[2]);
    }

    #[test]
    fn sepia_warms_pixels() {
        let mut buf = vec![100, 100, 100];
        apply_filter(&mut buf, PhotoFilter::Sepia);
        assert!(buf[0] >= buf[2], "sepia must keep red >= blue");
    }

    #[test]
    fn neon_boosts_red_and_blue() {
        let mut buf = vec![100, 100, 100];
        apply_filter(&mut buf, PhotoFilter::CyberpunkNeon);
        assert!(buf[0] > 100);
        assert!(buf[2] > 100);
        assert!(buf[1] < 100);
    }
}
