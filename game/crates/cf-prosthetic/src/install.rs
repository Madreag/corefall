//! **M14I** § Prosthetic install / maintain / tune state machine.

use serde::{Deserialize, Serialize};

use cf_wound::registry::{OriginId, ZoneId};

use super::{ProstheticKind, ProstheticSpec, ProstheticTier};

/// Spec § "Install — 60s sequence". Real seconds at the canonical 60 Hz
/// tick rate.
pub const PROSTHETIC_INSTALL_SECONDS: f32 = 60.0;

/// Spec § "Prosthetic maintenance interval = 7 in-game days". One in-game
/// year is 3600 sim seconds (cf-aging's canonical convention); 7 in-game
/// days = 7 / 365.25 × 3600 ≈ 69 sim seconds. Modders override per spec
/// via `ProstheticSpec.maintenance_interval_seconds`.
pub const PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS: f32 = 7.0 * 3600.0 / 365.25;

/// Spec § "Per-week wear check; below threshold, prosthetic malfunctions".
/// "Threshold 0.6" is the locked malfunction trigger.
pub const PROSTHETIC_MALFUNCTION_THRESHOLD: f32 = 0.6;

/// **M14I** § installed prosthetic instance on an actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProstheticInstance {
    pub kind: ProstheticKind,
    pub tier: ProstheticTier,
    pub zone: ZoneId,
    /// Tick the prosthetic was installed.
    pub installed_tick: u64,
    /// Tick of the last maintenance pass.
    pub last_maintained_tick: u64,
    /// Cumulative wear `[0, 1]`. 0.0 = pristine; 1.0 = scrap.
    pub wear_pct: f32,
    /// True once `wear_pct` crossed [`PROSTHETIC_MALFUNCTION_THRESHOLD`].
    pub malfunctioning: bool,
}

impl ProstheticInstance {
    pub fn new(kind: ProstheticKind, zone: ZoneId, installed_tick: u64) -> Self {
        Self {
            kind,
            tier: kind.tier(),
            zone,
            installed_tick,
            last_maintained_tick: installed_tick,
            wear_pct: 0.0,
            malfunctioning: false,
        }
    }

    /// Advance wear by `dt_seconds`. Returns `true` if the prosthetic
    /// just crossed the malfunction threshold (caller emits
    /// `prosthetic.malfunctioned`).
    pub fn advance_wear(&mut self, dt_seconds: f32, interval_seconds: f32) -> bool {
        if self.malfunctioning {
            // Wear continues to accrue, but no fresh malfunction event.
            self.wear_pct = (self.wear_pct + dt_seconds / interval_seconds).min(1.0);
            return false;
        }
        self.wear_pct = (self.wear_pct + dt_seconds / interval_seconds).min(1.0);
        if self.wear_pct >= PROSTHETIC_MALFUNCTION_THRESHOLD {
            self.malfunctioning = true;
            return true;
        }
        false
    }

    /// Routine maintenance — resets wear to 0 and clears the malfunction
    /// flag.
    pub fn maintain(&mut self, current_tick: u64) {
        self.wear_pct = 0.0;
        self.malfunctioning = false;
        self.last_maintained_tick = current_tick;
    }

    /// Tune the prosthetic up by one tier (T1 → T2 → T3).
    pub fn tune_up(&mut self) -> bool {
        match self.tier {
            ProstheticTier::T1 => {
                self.tier = ProstheticTier::T2;
                true
            }
            ProstheticTier::T2 => {
                self.tier = ProstheticTier::T3;
                true
            }
            ProstheticTier::T3 => false,
        }
    }

    /// Functional restoration multiplier (tier × (1 - wear) ramp).
    pub fn current_restoration(&self) -> f32 {
        let base = self.tier.functional_restoration();
        let wear_penalty = if self.malfunctioning {
            0.5
        } else {
            1.0 - 0.5 * self.wear_pct
        };
        (base * wear_penalty).clamp(0.0, 1.0)
    }
}

/// **M14I** § install-session state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallSession {
    pub actor_id: u64,
    pub kind: ProstheticKind,
    pub zone: ZoneId,
    pub seconds_remaining: f32,
    pub seconds_total: f32,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum InstallError {
    #[error("origin incompatible with prosthetic kind")]
    WrongOrigin,
    #[error("zone is not in the prosthetic's target zones")]
    WrongZone,
    #[error("missing medic_t2 skill")]
    MissingSkill,
    #[error("missing surgery table")]
    MissingTool,
    #[error("zone is not in Severed state")]
    NotSevered,
}

impl InstallSession {
    /// Construct a fresh install session. Validates origin / zone /
    /// skill / tool pre-flight per the M14I spec § "Install".
    pub fn start(
        actor_id: u64,
        kind: ProstheticKind,
        zone: ZoneId,
        spec: &ProstheticSpec,
        origin: &OriginId,
        has_medic_t2: bool,
        has_surgery_table: bool,
        zone_is_severed: bool,
    ) -> Result<Self, InstallError> {
        if !spec.compatible_origins.iter().any(|o| o == origin) {
            return Err(InstallError::WrongOrigin);
        }
        if !spec.target_zones.iter().any(|z| z == &zone) {
            return Err(InstallError::WrongZone);
        }
        if !has_medic_t2 {
            return Err(InstallError::MissingSkill);
        }
        if !has_surgery_table {
            return Err(InstallError::MissingTool);
        }
        if !zone_is_severed {
            return Err(InstallError::NotSevered);
        }
        Ok(Self {
            actor_id,
            kind,
            zone,
            seconds_remaining: spec.install_seconds,
            seconds_total: spec.install_seconds,
            completed: false,
        })
    }

    /// Advance the install timer.
    pub fn tick(&mut self, dt_seconds: f32) {
        if self.completed {
            return;
        }
        self.seconds_remaining -= dt_seconds;
        if self.seconds_remaining <= 0.0 {
            self.completed = true;
        }
    }

    /// Run the install session to completion and return the instance.
    pub fn install(&mut self, current_tick: u64) -> ProstheticInstance {
        self.completed = true;
        self.seconds_remaining = 0.0;
        ProstheticInstance::new(self.kind, self.zone.clone(), current_tick)
    }
}

/// **M14I** § maintenance pass error.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MaintenanceError {
    #[error("no prosthetic installed on the target zone")]
    NotInstalled,
    #[error("missing medic_t1 skill")]
    MissingSkill,
}

/// **M14I** § maintenance outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum MaintenanceOutcome {
    /// Wear reset to 0; malfunction flag cleared.
    Restored,
    /// No wear had accumulated; no-op.
    NoOp,
}

/// **M14I** § run a maintenance pass on a prosthetic instance. Returns
/// `Restored` when the wear/malfunction was reset, `NoOp` if there was
/// nothing to fix.
pub fn maintain_prosthetic(
    inst: &mut ProstheticInstance,
    has_medic_t1: bool,
    current_tick: u64,
) -> Result<MaintenanceOutcome, MaintenanceError> {
    if !has_medic_t1 {
        return Err(MaintenanceError::MissingSkill);
    }
    if inst.wear_pct == 0.0 && !inst.malfunctioning {
        return Ok(MaintenanceOutcome::NoOp);
    }
    inst.maintain(current_tick);
    Ok(MaintenanceOutcome::Restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::prosthetic_spec;

    #[test]
    fn install_60s_completes() {
        let spec = prosthetic_spec(ProstheticKind::ProstheticLegT1);
        let origin = OriginId::from("human");
        let mut s = InstallSession::start(
            42,
            ProstheticKind::ProstheticLegT1,
            ZoneId::from("leg_right"),
            &spec,
            &origin,
            true,
            true,
            true,
        )
        .expect("start install");
        s.tick(30.0);
        assert!(!s.completed);
        s.tick(31.0);
        assert!(s.completed);
    }

    #[test]
    fn install_rejects_robot() {
        let spec = prosthetic_spec(ProstheticKind::ProstheticLegT1);
        let origin = OriginId::from("robot");
        let r = InstallSession::start(
            1,
            ProstheticKind::ProstheticLegT1,
            ZoneId::from("leg_right"),
            &spec,
            &origin,
            true,
            true,
            true,
        );
        assert!(matches!(r, Err(InstallError::WrongOrigin)));
    }

    #[test]
    fn install_rejects_wrong_zone() {
        let spec = prosthetic_spec(ProstheticKind::ProstheticLegT1);
        let r = InstallSession::start(
            1,
            ProstheticKind::ProstheticLegT1,
            ZoneId::from("arm_left"),
            &spec,
            &OriginId::from("human"),
            true,
            true,
            true,
        );
        assert!(matches!(r, Err(InstallError::WrongZone)));
    }

    #[test]
    fn malfunction_at_60pct_wear() {
        let mut inst =
            ProstheticInstance::new(ProstheticKind::CyberneticLegT2, ZoneId::from("leg_right"), 0);
        // Accumulate wear in 7 in-game days × 0.5 increments.
        let crossed_a = inst.advance_wear(PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS * 0.5, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS);
        assert!(!crossed_a);
        let crossed_b = inst.advance_wear(PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS * 0.2, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS);
        assert!(crossed_b);
        assert!(inst.malfunctioning);
    }

    #[test]
    fn maintenance_clears_wear() {
        let mut inst =
            ProstheticInstance::new(ProstheticKind::CyberneticLegT2, ZoneId::from("leg_right"), 0);
        inst.advance_wear(PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS * 0.7, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS);
        assert!(inst.malfunctioning);
        let r = maintain_prosthetic(&mut inst, true, 100);
        assert!(matches!(r, Ok(MaintenanceOutcome::Restored)));
        assert_eq!(inst.wear_pct, 0.0);
        assert!(!inst.malfunctioning);
    }

    #[test]
    fn tune_up_t1_to_t2() {
        let mut inst =
            ProstheticInstance::new(ProstheticKind::ProstheticLegT1, ZoneId::from("leg_right"), 0);
        assert_eq!(inst.tier, ProstheticTier::T1);
        assert!(inst.tune_up());
        assert_eq!(inst.tier, ProstheticTier::T2);
        assert!(inst.tune_up());
        assert_eq!(inst.tier, ProstheticTier::T3);
        // T3 → no further tier.
        assert!(!inst.tune_up());
    }

    #[test]
    fn current_restoration_reflects_wear() {
        let mut inst =
            ProstheticInstance::new(ProstheticKind::ProstheticLegT1, ZoneId::from("leg_right"), 0);
        assert!((inst.current_restoration() - 0.70).abs() < 1e-6);
        // Push wear to 0.5 — non-malfunctioning yet (threshold 0.6).
        inst.advance_wear(PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS * 0.5, PROSTHETIC_MAINTENANCE_INTERVAL_SECONDS);
        let r = inst.current_restoration();
        assert!(r < 0.70 && r > 0.50);
    }
}
