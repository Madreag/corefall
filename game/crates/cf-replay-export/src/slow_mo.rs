//! M10B `--slow-mo` integer multiplier.
//!
//! Spec § "Out of scope":
//!
//! > Frame interpolation / slow-motion smoothing beyond integer
//! > slow-down (`--slow-mo 2x / 4x` integer multiplier ships;
//! > AI-frame-interp does not).
//!
//! VAL-M10B-SLOW-MO: `--slow-mo 2x/4x` integer multiplier extends
//! duration deterministically (`duration_seconds * multiplier`,
//! within ±1 frame). Non-integer multipliers (`--slow-mo 1.5`,
//! `3.5x`, etc.) are rejected with a typed error declaring "integer
//! multiplier required".
//!
//! The actual encode pipeline (m10b-4 + later) consumes a
//! [`SlowMoMultiplier`] in two places:
//!
//! 1. **Frame stream**: every source frame is emitted `multiplier`
//!    times in a row, so the output's frame count scales by
//!    `multiplier` while leaving each frame's pixel content
//!    unchanged.
//! 2. **Audio mix**: the deterministic audio base mix + commentary
//!    mix is rendered at `multiplier × original_duration` and
//!    interpolated zero-order-hold (linear-interpolation only when
//!    explicit per spec § Notes), preserving determinism across
//!    libav versions.
//!
//! The parser also accepts the user-facing CLI shape
//! (`--slow-mo 2x`, `--slow-mo 4`) so the CLI surface matches the
//! spec's bullet exactly.

use thiserror::Error;

/// Default multiplier: `1x` (no slow-mo). Returned by
/// [`SlowMoMultiplier::default`] so the CLI's `--slow-mo`-omitted
/// path produces a 1:1 frame stream.
pub const DEFAULT_SLOW_MO_MULTIPLIER: u32 = 1;

/// The maximum multiplier the export CLI accepts. Beyond this the
/// output bitrate budget becomes pathological + the ffmpeg bridge's
/// audio resample loses determinism on the very large stretch
/// factor. The spec's Out of scope bullet enumerates 2x + 4x as the
/// supported set; we accept any integer in `[1, MAX_SLOW_MO]` so
/// agents can experiment with 3x as well, but cap at 16x as a
/// sanity bound.
pub const MAX_SLOW_MO_MULTIPLIER: u32 = 16;

/// Typed errors surfaced by the slow-mo parser. Per VAL-M10B-SLOW-MO
/// non-integer multipliers + multipliers outside the supported range
/// must produce typed variants — never a generic `String` / `bail!`.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SlowMoError {
    #[error(
        "--slow-mo `{got}` is not an integer; integer multiplier required (e.g. `--slow-mo 2x`, \
         `--slow-mo 4`). Non-integer slow-mo (`3.5x`, `1.5x`) is out of scope per M10B spec §Out \
         of scope."
    )]
    NonInteger { got: String },
    #[error(
        "--slow-mo `{got}` is zero / negative; multiplier must be ≥ 1 (e.g. `--slow-mo 2x`)"
    )]
    NotPositive { got: i64 },
    #[error(
        "--slow-mo `{got}` exceeds maximum supported multiplier {max} (raise the cap if you \
         intentionally want longer-than-{max}× duration)"
    )]
    TooLarge { got: u32, max: u32 },
    #[error("--slow-mo argument is empty (expected an integer multiplier like `2x` or `4`)")]
    Empty,
}

/// Validated integer multiplier. Constructible only via
/// [`SlowMoMultiplier::parse`] or [`SlowMoMultiplier::from_u32`]
/// (the latter is also fallible for the zero / above-cap cases).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SlowMoMultiplier(u32);

impl SlowMoMultiplier {
    /// Parse the CLI argument. Accepts `2`, `2x`, `4x`, `04` —
    /// trailing whitespace is stripped. Rejects every non-integer
    /// pattern (`1.5`, `1.5x`, `3.5x`, `2.0`) with
    /// [`SlowMoError::NonInteger`].
    pub fn parse(raw: &str) -> Result<Self, SlowMoError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(SlowMoError::Empty);
        }
        // Strip the optional trailing `x` / `X`.
        let stripped = trimmed
            .strip_suffix('x')
            .or_else(|| trimmed.strip_suffix('X'))
            .unwrap_or(trimmed);
        if stripped.is_empty() {
            return Err(SlowMoError::Empty);
        }
        // Reject any non-integer payload — `parse::<i64>` is the cleanest
        // rejection that catches `1.5`, `2.0`, `-1`, `+2.5`, scientific
        // notation, etc.
        let parsed: i64 = stripped.parse::<i64>().map_err(|_| SlowMoError::NonInteger {
            got: raw.to_string(),
        })?;
        if parsed <= 0 {
            return Err(SlowMoError::NotPositive { got: parsed });
        }
        let casted = u32::try_from(parsed).map_err(|_| SlowMoError::TooLarge {
            got: u32::MAX,
            max: MAX_SLOW_MO_MULTIPLIER,
        })?;
        Self::from_u32(casted)
    }

    /// Construct from a known u32. Fallible — surfaces
    /// [`SlowMoError::NotPositive`] for `0` and
    /// [`SlowMoError::TooLarge`] for `> MAX_SLOW_MO_MULTIPLIER`.
    pub fn from_u32(value: u32) -> Result<Self, SlowMoError> {
        if value == 0 {
            return Err(SlowMoError::NotPositive { got: 0 });
        }
        if value > MAX_SLOW_MO_MULTIPLIER {
            return Err(SlowMoError::TooLarge {
                got: value,
                max: MAX_SLOW_MO_MULTIPLIER,
            });
        }
        Ok(Self(value))
    }

    /// Raw multiplier value.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }

    /// Scale a source tick count by the multiplier. Used by the
    /// encode pipeline to size the output frame stream and the audio
    /// buffer.
    #[must_use]
    pub const fn scale_ticks(self, source_ticks: u64) -> u64 {
        source_ticks.saturating_mul(self.0 as u64)
    }

    /// Scale a source duration (seconds, `f64`) by the multiplier.
    /// Used by the ffmpeg bridge to set the output container's
    /// duration tag (m10b-4 audit log + chapter-marker offsets are
    /// unchanged — only the playback duration scales).
    #[must_use]
    pub fn scale_duration_seconds(self, source_duration_seconds: f64) -> f64 {
        source_duration_seconds * self.0 as f64
    }

    /// `true` when the multiplier is `1` (no-op). Lets callers skip
    /// the per-frame duplication path entirely in the common case.
    #[must_use]
    pub const fn is_noop(self) -> bool {
        self.0 == DEFAULT_SLOW_MO_MULTIPLIER
    }
}

impl Default for SlowMoMultiplier {
    fn default() -> Self {
        Self(DEFAULT_SLOW_MO_MULTIPLIER)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_mo_2x_doubles_duration() {
        let m = SlowMoMultiplier::parse("2x").expect("2x parses");
        assert_eq!(m.value(), 2);
        assert_eq!(m.scale_duration_seconds(30.0), 60.0);
        assert_eq!(m.scale_ticks(1800), 3600);
    }

    #[test]
    fn slow_mo_4x_quadruples_duration() {
        let m = SlowMoMultiplier::parse("4x").expect("4x parses");
        assert_eq!(m.value(), 4);
        assert_eq!(m.scale_duration_seconds(30.0), 120.0);
        assert_eq!(m.scale_ticks(1800), 7200);
    }

    #[test]
    fn slow_mo_non_integer_rejected_with_typed_error() {
        let err = SlowMoMultiplier::parse("3.5x").expect_err("3.5x must error");
        assert!(matches!(err, SlowMoError::NonInteger { ref got } if got == "3.5x"));
        // Also without the `x` suffix.
        let err2 = SlowMoMultiplier::parse("1.5").expect_err("1.5 must error");
        assert!(matches!(err2, SlowMoError::NonInteger { ref got } if got == "1.5"));
    }

    #[test]
    fn slow_mo_accepts_integer_without_x_suffix() {
        let m = SlowMoMultiplier::parse("4").expect("plain int parses");
        assert_eq!(m.value(), 4);
    }

    #[test]
    fn slow_mo_rejects_zero() {
        let err = SlowMoMultiplier::parse("0x").expect_err("0x must error");
        assert!(matches!(err, SlowMoError::NotPositive { got: 0 }));
    }

    #[test]
    fn slow_mo_rejects_negative() {
        let err = SlowMoMultiplier::parse("-1").expect_err("-1 must error");
        assert!(matches!(err, SlowMoError::NotPositive { got: -1 }));
    }

    #[test]
    fn slow_mo_rejects_empty() {
        let err = SlowMoMultiplier::parse("").expect_err("empty must error");
        assert!(matches!(err, SlowMoError::Empty));
        let err2 = SlowMoMultiplier::parse("x").expect_err("just `x` must error");
        assert!(matches!(err2, SlowMoError::Empty));
    }

    #[test]
    fn slow_mo_rejects_too_large() {
        let err = SlowMoMultiplier::parse("100").expect_err("100 must exceed cap");
        assert!(matches!(
            err,
            SlowMoError::TooLarge { got: 100, max: MAX_SLOW_MO_MULTIPLIER }
        ));
    }

    #[test]
    fn slow_mo_default_is_one() {
        let m = SlowMoMultiplier::default();
        assert_eq!(m.value(), 1);
        assert!(m.is_noop());
    }

    #[test]
    fn slow_mo_uppercase_x_suffix_accepted() {
        let m = SlowMoMultiplier::parse("2X").expect("2X parses");
        assert_eq!(m.value(), 2);
    }
}
