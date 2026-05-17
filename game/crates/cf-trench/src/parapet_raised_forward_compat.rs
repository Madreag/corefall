//! M9B § "Notes for the implementer":
//! > "Sandbag breastwork consumes M9C-owned sandbag inventory; if M9C
//! >  not yet shipped, `parapet_raised` variant fails validation with
//! >  `requires_m9c=true` event (forward-compat)."
//!
//! This module exposes the typed event payload + the dig-time
//! validator. The validator's branch is selected at compile time
//! through the `m9c` Cargo feature (default-on). Workers in m9b-3 hook
//! [`parapet_raised_dig_validate`] into the cfctl
//! `act.player.dig_trench_segment` handler so the warning event fires
//! before any sim state is touched (VAL-M9B-PARAPET-FWDCOMPAT-001).
//!
//! Test-only path: invoking `cargo test -p cf-trench
//! parapet_raised_forward_compat --no-default-features` compiles the
//! `not(feature = "m9c")` branch + asserts the warning is emitted with
//! `requires_m9c=true`.

use serde::{Deserialize, Serialize};

use crate::segment::SegmentVariant;

/// Typed payload of the `trench.parapet_raised_requires_m9c` warning
/// event the cfctl handler emits when an actor attempts to dig the
/// `parapet_raised` variant in a build that lacks the M9C wiring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParapetRaisedRequiresM9c {
    pub variant: SegmentVariant,
    pub requires_m9c: bool,
    pub reason: String,
}

impl ParapetRaisedRequiresM9c {
    pub const EVENT_KIND: &'static str = "trench_parapet_raised_requires_m9c";
}

/// Validate at dig-time that the build supports the `parapet_raised`
/// variant. Returns `Ok(())` when the `m9c` feature is on (the default
/// for the workspace), or a [`ParapetRaisedRequiresM9c`] warning
/// payload when the feature is off. The cfctl handler is expected to
/// transform the `Err` arm into a `trench.parapet_raised_requires_m9c`
/// replay event + refuse the dig.
#[allow(unreachable_code)]
pub fn parapet_raised_dig_validate() -> Result<(), ParapetRaisedRequiresM9c> {
    #[cfg(feature = "m9c")]
    {
        return Ok(());
    }
    #[cfg(not(feature = "m9c"))]
    {
        return Err(ParapetRaisedRequiresM9c {
            variant: SegmentVariant::ParapetRaised,
            requires_m9c: true,
            reason: "parapet_raised embeds the M9C-owned `breastwork` sandbag module; \
                     build cf-trench with the `m9c` feature (default-on) to support \
                     this variant"
                .to_string(),
        });
    }
}

/// Spec-literal flag used by cf-mod + cfctl to distinguish the pre-M9C
/// warning from generic validation errors.
#[must_use]
pub const fn warning_event_kind() -> &'static str {
    ParapetRaisedRequiresM9c::EVENT_KIND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "m9c")]
    #[test]
    fn parapet_raised_dig_ok_with_m9c_feature() {
        let result = parapet_raised_dig_validate();
        assert!(
            result.is_ok(),
            "with `m9c` feature on, parapet_raised dig must validate"
        );
    }

    /// VAL-M9B-PARAPET-FWDCOMPAT-001 — running this test via
    /// `cargo test -p cf-trench parapet_raised_forward_compat
    /// --no-default-features` asserts the warning payload fires.
    #[cfg(not(feature = "m9c"))]
    #[test]
    fn parapet_raised_pre_m9c_warns() {
        let result = parapet_raised_dig_validate();
        let err = result.expect_err("expected requires_m9c warning when m9c feature is off");
        assert!(err.requires_m9c);
        assert_eq!(err.variant, SegmentVariant::ParapetRaised);
        assert!(!err.reason.is_empty(), "warning reason must not be empty");
    }

    #[test]
    fn warning_event_kind_matches_schema_filename_stem() {
        assert_eq!(
            warning_event_kind(),
            "trench_parapet_raised_requires_m9c",
            "kind must match the schema filename stem (game/crates/cf-replay/schemas/event/trench_parapet_raised_requires_m9c.json)"
        );
    }
}
