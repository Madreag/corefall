//! M6: magazine system (PopNextRound) — regular + tracer rounds in deterministic
//! interleave per CCCP `Magazine::RTTRatio`.
//!
//! Spec § "Magazine system per CCCP `Magazine::PopNextRound`: regular + tracer
//! rounds with `RTTRatio` (1 tracer per N rounds, deterministic).":
//!
//! The magazine pops rounds in a fixed pattern; the cf-control engine threads
//! the tracer flag into projectile spawn events so cf-render-2d can swap to
//! the tracer visual preset.

use serde::{Deserialize, Serialize};

/// One round popped from the magazine.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PoppedRound {
    pub round_kind: RoundKind,
    pub remaining_in_mag: u32,
}

/// Round kind. Regular vs tracer is the only distinction at M6; M14C adds
/// `Heat` (shaped-charge anti-tank) and `Apfsds` (long-rod kinetic) for
/// tank-grade ammo. Future extensions (AP, frangible) get their own variants.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundKind {
    Regular = 0,
    Tracer = 1,
    HighExplosive = 2,
    Pellet = 3,
    Heat = 4,
    Apfsds = 5,
}

impl RoundKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RoundKind::Regular => "regular",
            RoundKind::Tracer => "tracer",
            RoundKind::HighExplosive => "high_explosive",
            RoundKind::Pellet => "pellet",
            RoundKind::Heat => "heat",
            RoundKind::Apfsds => "apfsds",
        }
    }

    /// `cfctl.act.player.fire ammo_kind=...`). Returns `None` for unknown
    /// labels.
    pub fn from_str_snake(s: &str) -> Option<Self> {
        match s {
            "regular" => Some(RoundKind::Regular),
            "tracer" => Some(RoundKind::Tracer),
            "high_explosive" => Some(RoundKind::HighExplosive),
            "pellet" => Some(RoundKind::Pellet),
            "heat" => Some(RoundKind::Heat),
            "apfsds" => Some(RoundKind::Apfsds),
            _ => None,
        }
    }
}

/// Magazine state. M6 owns the deterministic pop interleave.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Magazine {
    pub capacity: u32,
    pub remaining: u32,
    /// 1-in-N tracer cadence. 0 = no tracers.
    pub tracer_round_to_total_ratio: u32,
    /// Counter incremented per pop; deterministic across replays.
    pub round_counter: u64,
    /// Round kind override for special magazines (HE for grenade launcher,
    /// Pellet for shotgun).
    pub primary_round: RoundKind,
}

impl Default for Magazine {
    fn default() -> Self {
        Self {
            capacity: 30,
            remaining: 30,
            tracer_round_to_total_ratio: 0,
            round_counter: 0,
            primary_round: RoundKind::Regular,
        }
    }
}

impl Magazine {
    pub fn new(capacity: u32, ratio: u32, primary: RoundKind) -> Self {
        Self {
            capacity,
            remaining: capacity,
            tracer_round_to_total_ratio: ratio,
            round_counter: 0,
            primary_round: primary,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.remaining == 0
    }

    pub fn refill(&mut self) {
        self.remaining = self.capacity;
        self.round_counter = 0;
    }

    /// Deterministic "next round is a tracer" check before pop. Useful for
    /// HUD preview.
    pub fn next_is_tracer(&self) -> bool {
        if self.tracer_round_to_total_ratio == 0 {
            return false;
        }
        ((self.round_counter + 1) % u64::from(self.tracer_round_to_total_ratio)) == 0
    }

    /// Pop the next round. Returns None when empty.
    pub fn pop_next_round(&mut self) -> Option<PoppedRound> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        self.round_counter += 1;
        let kind = if self.tracer_round_to_total_ratio > 0
            && (self.round_counter % u64::from(self.tracer_round_to_total_ratio)) == 0
        {
            RoundKind::Tracer
        } else {
            self.primary_round
        };
        Some(PoppedRound {
            round_kind: kind,
            remaining_in_mag: self.remaining,
        })
    }

    /// Force-load a specific round into the next-pop slot (for cheat / debug).
    pub fn force_set_remaining(&mut self, remaining: u32) {
        self.remaining = remaining.min(self.capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracer_cadence_3_yields_3_tracers_in_9() {
        let mut m = Magazine::new(30, 3, RoundKind::Regular);
        let mut tracers = 0;
        for _ in 0..9 {
            if let Some(r) = m.pop_next_round() {
                if r.round_kind == RoundKind::Tracer {
                    tracers += 1;
                }
            }
        }
        assert_eq!(tracers, 3);
    }

    #[test]
    fn deterministic_across_two_pops() {
        let mut a = Magazine::new(40, 5, RoundKind::Regular);
        let mut b = Magazine::new(40, 5, RoundKind::Regular);
        for _ in 0..20 {
            assert_eq!(a.pop_next_round(), b.pop_next_round());
        }
    }

    #[test]
    fn empty_returns_none() {
        let mut m = Magazine::new(2, 0, RoundKind::Regular);
        assert!(m.pop_next_round().is_some());
        assert!(m.pop_next_round().is_some());
        assert!(m.pop_next_round().is_none());
    }

    #[test]
    fn refill_restores_capacity() {
        let mut m = Magazine::new(3, 0, RoundKind::Regular);
        let _ = m.pop_next_round();
        m.refill();
        assert_eq!(m.remaining, 3);
    }

    #[test]
    fn shotgun_primary_pellet() {
        let mut m = Magazine::new(8, 0, RoundKind::Pellet);
        let r = m.pop_next_round().unwrap();
        assert_eq!(r.round_kind, RoundKind::Pellet);
    }

    /// **VAL-M14C-001**: `RoundKind::Heat` + `RoundKind::Apfsds` variants
    /// resolve and round-trip through `as_str` / `from_str_snake` without
    /// reaching `unreachable!()`.
    #[test]
    fn heat_and_apfsds_round_kinds_resolve() {
        let kinds = [RoundKind::Heat, RoundKind::Apfsds];
        for k in kinds {
            let s = k.as_str();
            assert!(matches!(s, "heat" | "apfsds"));
            let parsed = RoundKind::from_str_snake(s).expect("snake_case round-trip");
            assert_eq!(parsed, k);
        }
        // pattern-match exhaustiveness check — if a future RoundKind variant
        // is added, this `match` will force a compile-time visit here.
        for k in [
            RoundKind::Regular,
            RoundKind::Tracer,
            RoundKind::HighExplosive,
            RoundKind::Pellet,
            RoundKind::Heat,
            RoundKind::Apfsds,
        ] {
            match k {
                RoundKind::Regular
                | RoundKind::Tracer
                | RoundKind::HighExplosive
                | RoundKind::Pellet
                | RoundKind::Heat
                | RoundKind::Apfsds => {}
            }
        }
    }

    /// **VAL-M14C-001 follow-on**: a HEAT magazine pops HEAT rounds and an
    /// APFSDS magazine pops APFSDS rounds.
    #[test]
    fn heat_and_apfsds_magazines_pop_their_kind() {
        let mut heat = Magazine::new(2, 0, RoundKind::Heat);
        let r = heat.pop_next_round().unwrap();
        assert_eq!(r.round_kind, RoundKind::Heat);
        let mut apfsds = Magazine::new(2, 0, RoundKind::Apfsds);
        let r = apfsds.pop_next_round().unwrap();
        assert_eq!(r.round_kind, RoundKind::Apfsds);
    }

    #[test]
    fn next_is_tracer_preview() {
        let mut m = Magazine::new(10, 3, RoundKind::Regular);
        assert!(!m.next_is_tracer());
        let _ = m.pop_next_round();
        let _ = m.pop_next_round();
        assert!(m.next_is_tracer());
    }
}
