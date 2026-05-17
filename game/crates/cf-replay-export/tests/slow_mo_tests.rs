//! VAL-M10B-SLOW-MO integration tests.
//!
//! `--slow-mo 2x` / `--slow-mo 4x` scale output duration deterministically;
//! non-integer multipliers return typed errors.

use cf_replay_export::slow_mo::{
    SlowMoError, SlowMoMultiplier, DEFAULT_SLOW_MO_MULTIPLIER, MAX_SLOW_MO_MULTIPLIER,
};

/// VAL-M10B-SLOW-MO: 2x doubles, 4x quadruples.
#[test]
fn slow_mo_integer_multipliers() {
    let two = SlowMoMultiplier::parse("2x").expect("2x parses");
    assert_eq!(two.value(), 2);
    assert_eq!(two.scale_duration_seconds(30.0), 60.0);
    assert_eq!(two.scale_ticks(1_800), 3_600);

    let four = SlowMoMultiplier::parse("4x").expect("4x parses");
    assert_eq!(four.value(), 4);
    assert_eq!(four.scale_duration_seconds(30.0), 120.0);
    assert_eq!(four.scale_ticks(1_800), 7_200);
}

/// VAL-M10B-SLOW-MO: 3.5x non-integer returns typed error.
#[test]
fn slow_mo_non_integer_rejected_with_typed_error() {
    let err = SlowMoMultiplier::parse("3.5x").expect_err("3.5x must error");
    match err {
        SlowMoError::NonInteger { got } => assert_eq!(got, "3.5x"),
        other => panic!("expected NonInteger, got {other:?}"),
    }
}

/// SlowMoMultiplier::default() returns `1` (no slow-mo).
#[test]
fn slow_mo_default_is_one() {
    let default = SlowMoMultiplier::default();
    assert_eq!(default.value(), DEFAULT_SLOW_MO_MULTIPLIER);
    assert!(default.is_noop());
}

/// Cap enforcement: multipliers above MAX_SLOW_MO_MULTIPLIER reject.
#[test]
fn slow_mo_above_max_rejected() {
    let err = SlowMoMultiplier::parse("99").expect_err("99 exceeds cap");
    match err {
        SlowMoError::TooLarge { got: 99, max } => assert_eq!(max, MAX_SLOW_MO_MULTIPLIER),
        other => panic!("expected TooLarge, got {other:?}"),
    }
}
