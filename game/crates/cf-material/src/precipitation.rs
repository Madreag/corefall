//! **M15B** § Steam → cloud → rain precipitation cycle.
//!
//! Per the M15B spec § "Player-facing behavior":
//! > Steam rising from a geyser nucleates into visible cloud at altitude
//! > greater than 80px and ambient temp under 80°C; cloud accumulates;
//! > eventually precipitates as rain that flows back down per terrain.
//!
//! > Acid rain (Vulcan ambient): airborne pollutant + steam react →
//! > acid_droplets fall → corrode metal_nohook surfaces over time.
//!
//! > Player can observe the full water cycle: ground evaporates → cloud
//! > forms → rain falls → puddles flow back into low ground → repeat.
//!
//! ## Material ids (per `content/materials/material_registry.json`)
//!
//! - 50 = steam (gas)
//! - 71 = cloud (gas — accumulated steam at altitude)
//! - 87 = rain (liquid — falls from saturated cloud)
//! - 88 = acid_droplet (liquid — Vulcan ambient precipitation)
//! - 13 = water (liquid — pools back into puddles via cf-terrain liquid_flow)
//! - 62 = smoke / pollutant_x (vulcan pollutant)
//!
//! ## Gating rules (locked per spec)
//!
//! - **Nucleation threshold**: altitude > 80px AND ambient_temp < 80°C
//!   (353.15 K). The nucleation rate scales with steam density at the
//!   cell and with the (1 - pollutant_fraction) factor.
//! - **Saturation threshold**: cloud accumulates per cell; when
//!   saturation crosses 80%, precipitation starts.
//! - **Precipitation tick gate**: 60 ticks (1 sim-second at 60Hz) after
//!   the saturation crossing fires `material_precipitation_started`.
//! - **Acid rain trigger**: pollutant fraction > 5% in the precipitation
//!   cell → rain droplets become `acid_droplet`.
//! - **Vulcan ambient** is a per-scenario constant (per spec §
//!   "Vulcan = always-rain due to high humidity from oceans; Mimas =
//!   never rain (vacuum); Mars = rare rain (thin atm)") — surfaced as
//!   [`AmbientWorld::vulcan`], etc.

use serde::{Deserialize, Serialize};

use cf_terrain::chunked::ChunkedTerrain;

use crate::MaterialId;

/// Stable across mod loads because the launch registry pins them.
pub mod ids {
    use super::MaterialId;
    pub const STEAM: MaterialId = 50;
    pub const CLOUD: MaterialId = 71;
    pub const RAIN: MaterialId = 87;
    pub const ACID_DROPLET: MaterialId = 88;
    pub const WATER: MaterialId = 13;
    pub const POLLUTANT_PROXY: MaterialId = 62; // smoke / pollutant_x for Vulcan ambient
}

/// into cloud. Per spec literal: "altitude > 80 px".
pub const NUCLEATION_ALTITUDE_PX: f32 = 80.0;

/// literal: "ambient temp < 80°C" = 353.15 K. Above this, steam stays
/// gaseous.
pub const NUCLEATION_TEMP_K_MAX: f32 = 353.15;

/// Per spec literal: "cloud at saturation > 80% (locked threshold)".
pub const PRECIPITATION_SATURATION_THRESHOLD: f32 = 0.80;

/// `material_precipitation_started` event firing. Per spec literal:
/// "When 60 ticks elapse Then material_precipitation_started event
/// fires".
pub const PRECIPITATION_TICK_GATE: u64 = 60;

/// "pollutant fraction > 5% Then the rain droplets become acid_droplet".
pub const ACID_RAIN_POLLUTANT_FRACTION_MIN: f32 = 0.05;

/// Used as the pressure denominator in the precipitation rate multiplier
/// so worlds at ambient Earth pressure produce a multiplier of 1.0.
pub const REFERENCE_PRESSURE_KPA: f32 = 101.325;

/// can nucleate regardless of humidity / pollutant fraction. Per spec
/// literal: "Mimas = never rain (vacuum)". Mimas's pressure (effectively
/// zero) sits below this gate; Mars's ~0.6 kPa thin atmosphere is also
/// gated out unless the per-world humidity preset overrides via
/// [`AmbientWorld::always_precipitates`].
pub const NUCLEATION_PRESSURE_MIN_KPA: f32 = 1.0;

/// Per real meteorology: low-pressure systems → faster cloud formation
/// (adiabatic cooling lets water vapor expand + condense). High-pressure
/// systems → slower formation. The multiplier is `REFERENCE_PRESSURE_KPA
/// / ambient_pressure_kpa` clamped to this `[lo, hi]` range so a vacuum
/// chamber doesn't divide-by-zero + a deep mine doesn't slow to a halt.
pub const PRESSURE_MULTIPLIER_RANGE: (f32, f32) = (0.5, 2.0);

/// the spec-locked constants but allows content-driven overrides via
/// `content/materials/precipitation_config.json`. The schema is a
/// 1:1 JSON map of this struct; missing fields fall back to the
/// `default_*` baseline values via serde defaults.
///
/// Modders + tuners edit the JSON file to tweak how easily clouds
/// form, when precipitation triggers, how acid rain detects pollutant,
/// etc. — without touching the engine source.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationConfig {
    #[serde(default = "default_nucleation_altitude_px")]
    pub nucleation_altitude_px: f32,
    #[serde(default = "default_nucleation_temp_k_max")]
    pub nucleation_temp_k_max: f32,
    #[serde(default = "default_precipitation_saturation_threshold")]
    pub precipitation_saturation_threshold: f32,
    #[serde(default = "default_precipitation_tick_gate")]
    pub precipitation_tick_gate: u64,
    #[serde(default = "default_acid_rain_pollutant_fraction_min")]
    pub acid_rain_pollutant_fraction_min: f32,
    #[serde(default = "default_reference_pressure_kpa")]
    pub reference_pressure_kpa: f32,
    #[serde(default = "default_nucleation_pressure_min_kpa")]
    pub nucleation_pressure_min_kpa: f32,
    #[serde(default = "default_pressure_multiplier_lo")]
    pub pressure_multiplier_lo: f32,
    #[serde(default = "default_pressure_multiplier_hi")]
    pub pressure_multiplier_hi: f32,
}

fn default_nucleation_altitude_px() -> f32 {
    NUCLEATION_ALTITUDE_PX
}
fn default_nucleation_temp_k_max() -> f32 {
    NUCLEATION_TEMP_K_MAX
}
fn default_precipitation_saturation_threshold() -> f32 {
    PRECIPITATION_SATURATION_THRESHOLD
}
fn default_precipitation_tick_gate() -> u64 {
    PRECIPITATION_TICK_GATE
}
fn default_acid_rain_pollutant_fraction_min() -> f32 {
    ACID_RAIN_POLLUTANT_FRACTION_MIN
}
fn default_reference_pressure_kpa() -> f32 {
    REFERENCE_PRESSURE_KPA
}
fn default_nucleation_pressure_min_kpa() -> f32 {
    NUCLEATION_PRESSURE_MIN_KPA
}
fn default_pressure_multiplier_lo() -> f32 {
    PRESSURE_MULTIPLIER_RANGE.0
}
fn default_pressure_multiplier_hi() -> f32 {
    PRESSURE_MULTIPLIER_RANGE.1
}

impl Default for PrecipitationConfig {
    fn default() -> Self {
        Self {
            nucleation_altitude_px: NUCLEATION_ALTITUDE_PX,
            nucleation_temp_k_max: NUCLEATION_TEMP_K_MAX,
            precipitation_saturation_threshold: PRECIPITATION_SATURATION_THRESHOLD,
            precipitation_tick_gate: PRECIPITATION_TICK_GATE,
            acid_rain_pollutant_fraction_min: ACID_RAIN_POLLUTANT_FRACTION_MIN,
            reference_pressure_kpa: REFERENCE_PRESSURE_KPA,
            nucleation_pressure_min_kpa: NUCLEATION_PRESSURE_MIN_KPA,
            pressure_multiplier_lo: PRESSURE_MULTIPLIER_RANGE.0,
            pressure_multiplier_hi: PRESSURE_MULTIPLIER_RANGE.1,
        }
    }
}

impl PrecipitationConfig {
    /// fields fall back to the spec-locked baseline values via serde
    /// defaults.
    pub fn load_from_file(
        path: impl AsRef<std::path::Path>,
    ) -> Result<Self, PrecipitationConfigLoadError> {
        let path_ref = path.as_ref();
        let raw =
            std::fs::read_to_string(path_ref).map_err(|source| PrecipitationConfigLoadError::Io {
                path: path_ref.to_path_buf(),
                source,
            })?;
        let cfg: PrecipitationConfig = serde_json::from_str(&raw).map_err(|source| {
            PrecipitationConfigLoadError::Parse {
                path: path_ref.to_path_buf(),
                source,
            }
        })?;
        Ok(cfg)
    }

    #[must_use]
    pub fn locate_default() -> Option<std::path::PathBuf> {
        for candidate in [
            std::path::PathBuf::from("content/materials/precipitation_config.json"),
            std::path::PathBuf::from("../content/materials/precipitation_config.json"),
            std::path::PathBuf::from("game/content/materials/precipitation_config.json"),
        ] {
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }

    /// baked constants when the JSON file isn't present.
    ///
    /// **Modder feedback**: emits a `tracing::warn!` when the JSON
    /// file IS present but fails to parse so a typo isn't silently
    /// swallowed.
    #[must_use]
    pub fn load_default_or_baseline() -> Self {
        if let Some(path) = Self::locate_default() {
            match Self::load_from_file(&path) {
                Ok(c) => return c,
                Err(err) => {
                    tracing::warn!(
                        target: "cf_material::precipitation",
                        path = %path.display(),
                        error = ?err,
                        "precipitation_config.json present but failed to load — falling back to baseline defaults"
                    );
                }
            }
        }
        Self::default()
    }

    /// range (vs the module-level constant). Modders override the
    /// `pressure_multiplier_lo/hi` to tune cloud-formation aggression.
    #[must_use]
    pub fn pressure_rate_multiplier(&self, ambient_pressure_kpa: f32) -> f32 {
        let p = ambient_pressure_kpa.max(0.001);
        let raw = self.reference_pressure_kpa / p;
        raw.clamp(self.pressure_multiplier_lo, self.pressure_multiplier_hi)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PrecipitationConfigLoadError {
    #[error("failed to read precipitation config at {}: {source}", path.display())]
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse precipitation config at {}: {source}", path.display())]
    Parse {
        path: std::path::PathBuf,
        source: serde_json::Error,
    },
}

/// scales with per-world humidity; the spec literal enumerates three:
/// > Vulcan = always-rain due to high humidity from oceans; Mimas =
/// > never rain (vacuum); Mars = rare rain (thin atm).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbientWorld {
    /// Earth-baseline humidity (~0.55). Default for unspecified scenarios.
    #[default]
    Earth,
    /// "always-rain" preset; humidity 0.95, pollutant 0.10 → defaults to
    /// acid rain.
    Vulcan,
    /// "never rain" preset; humidity 0.0 (vacuum).
    Mimas,
    /// "rare rain" preset; humidity 0.05 (thin atm).
    Mars,
}

impl AmbientWorld {
    /// Per-world baseline humidity (0..1). Drives the nucleation rate.
    #[must_use]
    pub fn humidity(self) -> f32 {
        match self {
            AmbientWorld::Earth => 0.55,
            AmbientWorld::Vulcan => 0.95,
            AmbientWorld::Mimas => 0.0,
            AmbientWorld::Mars => 0.05,
        }
    }

    /// Per-world baseline pollutant fraction (0..1). Drives the acid
    /// rain trigger.
    #[must_use]
    pub fn pollutant_fraction(self) -> f32 {
        match self {
            AmbientWorld::Earth => 0.01,
            AmbientWorld::Vulcan => 0.10,
            // Mimas (vacuum) + Mars (thin atm) carry no industrial
            // pollutant in their default baseline.
            AmbientWorld::Mimas | AmbientWorld::Mars => 0.0,
        }
    }

    /// Per-world default "always-on" precipitation flag — when true, the
    /// precipitation cycle bypasses the saturation gate (Vulcan always
    /// rains).
    #[must_use]
    pub fn always_precipitates(self) -> bool {
        matches!(self, AmbientWorld::Vulcan)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AmbientWorld::Earth => "earth",
            AmbientWorld::Vulcan => "vulcan",
            AmbientWorld::Mimas => "mimas",
            AmbientWorld::Mars => "mars",
        }
    }
}

/// precipitation cycle scans every cell with a non-zero cloud level
/// per tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CloudCell {
    pub world_x: i32,
    pub world_y: i32,
    /// Saturation (0..1). When > [`PRECIPITATION_SATURATION_THRESHOLD`],
    /// the cell starts the precipitation countdown.
    pub saturation: f32,
    /// Fraction of pollutant (0..1) trapped in the cloud. Drives the
    /// acid-rain trigger.
    pub pollutant_fraction: f32,
    /// Tick the cell first crossed the saturation threshold. `None`
    /// when below threshold.
    pub saturated_at_tick: Option<u64>,
    /// True when the cell has emitted `material_precipitation_started`
    /// and is actively raining.
    pub raining: bool,
}

impl CloudCell {
    #[must_use]
    pub fn new(world_x: i32, world_y: i32) -> Self {
        Self {
            world_x,
            world_y,
            saturation: 0.0,
            pollutant_fraction: 0.0,
            saturated_at_tick: None,
            raining: false,
        }
    }
}

/// pixel. Mirrors the `material_phase_nucleated.json` schema in
/// `cf-replay/schemas/event/`.
///
/// Both the numeric id pair (`from_material` / `to_material`) AND the
/// snake_case name pair (`from` / `to`) are emitted so the JSON payload
/// is self-describing — per spec § acceptance scenario 3:
/// > material_phase_nucleated event fires with from="steam" to="cloud"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseNucleatedEvent {
    /// Material that nucleated (e.g. steam → cloud).
    pub from_material: MaterialId,
    pub to_material: MaterialId,
    /// Snake-case human-readable name of `from_material` (e.g.
    /// `"steam"`). Resolved via [`material_id_to_name`].
    pub from: String,
    /// Snake-case human-readable name of `to_material` (e.g. `"cloud"`).
    pub to: String,
    pub pos: [i32; 2],
    pub altitude_px: f32,
    pub temperature_k: f32,
    pub tick: u64,
}

/// chain. Used by [`PhaseNucleatedEvent`] + [`PrecipitationStartedEvent`]
/// payload builders so the JSON event is self-describing.
#[must_use]
pub fn material_id_to_name(id: MaterialId) -> &'static str {
    match id {
        0 => "air",
        13 => "water",
        21 => "acid",
        50 => "steam",
        62 => "smoke",
        65 => "fire_intense",
        71 => "cloud",
        87 => "rain",
        88 => "acid_droplet",
        _ => "unknown",
    }
}

impl PhaseNucleatedEvent {
    /// shape matches `cf-replay/schemas/event/material_phase_nucleated.json`
    /// 1:1. The engine layer plugs this into
    /// `Recorder::record(tick, sim_time_ms, "material",
    /// "phase_nucleated", payload, None)`.
    #[must_use]
    pub fn to_recorder_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "from_material": self.from_material,
            "to_material": self.to_material,
            "from": self.from,
            "to": self.to,
            "pos": [self.pos[0], self.pos[1]],
            "altitude_px": self.altitude_px,
            "temperature_k": self.temperature_k,
        })
    }
}

impl PrecipitationStartedEvent {
    /// shape matches
    /// `cf-replay/schemas/event/material_precipitation_started.json`.
    #[must_use]
    pub fn to_recorder_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "material": self.material,
            "pos": [self.pos[0], self.pos[1]],
            "saturation": self.saturation,
            "pollutant_fraction": self.pollutant_fraction,
            "ambient": self.ambient,
        })
    }
}

/// gate and begins raining.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrecipitationStartedEvent {
    pub pos: [i32; 2],
    pub material: MaterialId, // 87 (rain) or 88 (acid_droplet)
    pub saturation: f32,
    pub pollutant_fraction: f32,
    pub ambient: String,
    pub tick: u64,
}

///
/// temperature + pressure inputs for precipitation gating" — this
/// struct accepts BOTH temperature and pressure from the M19 air-
/// pressure + heat fields. The `ambient_pressure_kpa` field is the
/// per-cell sample from [`cf_terrain::AirField::pressure_at_world`]
/// when M19 is wired into the engine; tests + simple scenarios may
/// omit it via [`PrecipitationInputs::with_default_pressure`] which
/// defaults to Earth sea-level pressure.
#[derive(Debug, Clone, Copy)]
pub struct PrecipitationInputs {
    pub material: MaterialId,
    pub world_x: i32,
    pub world_y: i32,
    pub altitude_px: f32,
    pub ambient_temp_k: f32,
    /// is the canonical producer; defaults to [`REFERENCE_PRESSURE_KPA`]
    /// (Earth sea level) when not supplied. Drives the
    /// [`pressure_rate_multiplier`] applied to nucleation + saturation.
    pub ambient_pressure_kpa: f32,
    pub ambient_world: AmbientWorld,
    pub pollutant_fraction_local: f32,
    pub tick: u64,
}

impl PrecipitationInputs {
    /// Construct an input row without an explicit pressure sample —
    /// uses [`REFERENCE_PRESSURE_KPA`] (Earth sea-level ambient). Used
    /// by tests + scenarios that don't yet wire M19 atmospherics.
    #[must_use]
    pub fn with_default_pressure(
        material: MaterialId,
        world_x: i32,
        world_y: i32,
        altitude_px: f32,
        ambient_temp_k: f32,
        ambient_world: AmbientWorld,
        pollutant_fraction_local: f32,
        tick: u64,
    ) -> Self {
        Self {
            material,
            world_x,
            world_y,
            altitude_px,
            ambient_temp_k,
            ambient_pressure_kpa: REFERENCE_PRESSURE_KPA,
            ambient_world,
            pollutant_fraction_local,
            tick,
        }
    }
}

/// saturation rates. Per real meteorology: low-pressure systems
/// accelerate cloud formation (adiabatic cooling expands rising air),
/// high-pressure systems slow it. The multiplier is `REFERENCE_PRESSURE_KPA
/// / ambient_pressure_kpa` clamped to [`PRESSURE_MULTIPLIER_RANGE`].
///
/// Ambient world overrides:
/// - At sea-level Earth pressure (101.325 kPa) the multiplier is 1.0
///   (unchanged behaviour vs M15B pre-pressure baseline).
/// - At 50 kPa (e.g., 5 km altitude) the multiplier is 2.0 (faster).
/// - At 200 kPa (e.g., deep mine / pressurized dome) the multiplier
///   is 0.51 (slower).
/// - At vacuum / near-zero pressure: clamped to 2.0 (no divide-by-zero).
#[must_use]
pub fn pressure_rate_multiplier(ambient_pressure_kpa: f32) -> f32 {
    let p = ambient_pressure_kpa.max(0.001);
    let raw = REFERENCE_PRESSURE_KPA / p;
    raw.clamp(PRESSURE_MULTIPLIER_RANGE.0, PRESSURE_MULTIPLIER_RANGE.1)
}

/// [`PhaseNucleatedEvent`] when the pixel crosses the altitude +
/// temperature gates, else `None`.
///
/// > When steam particles reach altitude > 80 px with ambient temp <
/// > 80°C Then material_phase_nucleated event fires with from="steam"
/// > to="cloud"
#[must_use]
pub fn evaluate_steam_nucleation(inputs: PrecipitationInputs) -> Option<PhaseNucleatedEvent> {
    if inputs.material != ids::STEAM {
        return None;
    }
    if inputs.altitude_px <= NUCLEATION_ALTITUDE_PX {
        return None;
    }
    if inputs.ambient_temp_k >= NUCLEATION_TEMP_K_MAX {
        return None;
    }
    // vacuum / near-vacuum pressures block nucleation entirely. The cutoff
    // is conservative (1.0 kPa) so Mars's ~0.6 kPa thin atmosphere is
    // gated out unless the per-world `always_precipitates` override fires.
    if inputs.ambient_pressure_kpa < NUCLEATION_PRESSURE_MIN_KPA && !inputs.ambient_world.always_precipitates() {
        return None;
    }
    Some(PhaseNucleatedEvent {
        from_material: ids::STEAM,
        to_material: ids::CLOUD,
        from: material_id_to_name(ids::STEAM).to_string(),
        to: material_id_to_name(ids::CLOUD).to_string(),
        pos: [inputs.world_x, inputs.world_y],
        altitude_px: inputs.altitude_px,
        temperature_k: inputs.ambient_temp_k,
        tick: inputs.tick,
    })
}

/// tick at a rate proportional to world humidity. Per-cell rate is
/// `humidity * 0.05` per tick — Vulcan saturates in ~21 ticks, Earth
/// in ~36 ticks, Mars in ~400 ticks. Default for unspecified is
/// `Earth`.
#[must_use]
pub fn saturation_rate_per_tick(world: AmbientWorld) -> f32 {
    world.humidity() * 0.05
}

/// Per real meteorology, low-pressure systems accelerate cloud
/// formation (adiabatic cooling). Multiplies the base humidity rate by
/// the [`pressure_rate_multiplier`].
///
/// Examples:
/// - Earth ambient (101.325 kPa) → multiplier 1.0 → identical to
///   [`saturation_rate_per_tick`].
/// - 50 kPa (high altitude / weather front) → multiplier 2.0 →
///   2× faster saturation.
/// - 200 kPa (pressurized dome / submarine) → multiplier ~0.5 →
///   ~2× slower saturation.
#[must_use]
pub fn saturation_rate_per_tick_with_pressure(world: AmbientWorld, ambient_pressure_kpa: f32) -> f32 {
    saturation_rate_per_tick(world) * pressure_rate_multiplier(ambient_pressure_kpa)
}

/// `Some(PrecipitationStartedEvent)` when this update crosses the
/// precipitation tick gate.
///
/// > Given an accumulated cloud at saturation > 80% (locked threshold)
/// > When 60 ticks elapse Then material_precipitation_started event
/// > fires
///
/// > When the precipitation cycle nucleates with pollutant fraction >
/// > 5% Then the rain droplets become "acid_droplet" material
pub fn update_cloud_cell(
    cell: &mut CloudCell,
    world: AmbientWorld,
    pollutant_fraction_inc: f32,
    tick: u64,
) -> Option<PrecipitationStartedEvent> {
    update_cloud_cell_with_pressure(cell, world, REFERENCE_PRESSURE_KPA, pollutant_fraction_inc, tick)
}

/// spec § dependency — M19 atmospherics provides the ambient pressure
/// sample; this entry point honors it via the
/// [`pressure_rate_multiplier`].
pub fn update_cloud_cell_with_pressure(
    cell: &mut CloudCell,
    world: AmbientWorld,
    ambient_pressure_kpa: f32,
    pollutant_fraction_inc: f32,
    tick: u64,
) -> Option<PrecipitationStartedEvent> {
    if cell.raining {
        // Already raining — saturation drains while the cell sheds rain.
        cell.saturation = (cell.saturation - 0.01).max(0.0);
        if cell.saturation <= 0.0 {
            cell.saturated_at_tick = None;
            cell.raining = false;
        }
        return None;
    }

    let inc = saturation_rate_per_tick_with_pressure(world, ambient_pressure_kpa);
    cell.saturation = (cell.saturation + inc).clamp(0.0, 1.0);
    cell.pollutant_fraction = (cell.pollutant_fraction + pollutant_fraction_inc).clamp(0.0, 1.0);

    if cell.saturation >= PRECIPITATION_SATURATION_THRESHOLD {
        if cell.saturated_at_tick.is_none() {
            cell.saturated_at_tick = Some(tick);
            return None;
        }
        let crossed_at = cell.saturated_at_tick.expect("just-set");
        let elapsed = tick.saturating_sub(crossed_at);
        if elapsed >= PRECIPITATION_TICK_GATE {
            cell.raining = true;
            let mat = if cell.pollutant_fraction >= ACID_RAIN_POLLUTANT_FRACTION_MIN
                || world.pollutant_fraction() >= ACID_RAIN_POLLUTANT_FRACTION_MIN
            {
                ids::ACID_DROPLET
            } else {
                ids::RAIN
            };
            return Some(PrecipitationStartedEvent {
                pos: [cell.world_x, cell.world_y],
                material: mat,
                saturation: cell.saturation,
                pollutant_fraction: cell.pollutant_fraction.max(world.pollutant_fraction()),
                ambient: world.as_str().to_string(),
                tick,
            });
        }
    } else {
        cell.saturated_at_tick = None;
    }
    None
}

/// cloud map + the per-tick output channel. The engine calls
/// [`PrecipitationCycle::step`] each tick with the inputs of every
/// active cell.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrecipitationCycle {
    pub world: AmbientWorld,
    pub cells: std::collections::BTreeMap<(i32, i32), CloudCell>,
    pub nucleated_events: Vec<PhaseNucleatedEvent>,
    pub precipitation_events: Vec<PrecipitationStartedEvent>,
}

impl PrecipitationCycle {
    #[must_use]
    pub fn new(world: AmbientWorld) -> Self {
        Self {
            world,
            cells: std::collections::BTreeMap::new(),
            nucleated_events: Vec::new(),
            precipitation_events: Vec::new(),
        }
    }

    /// Drain accumulated events and clear them from the buffer.
    pub fn drain_events(&mut self) -> (Vec<PhaseNucleatedEvent>, Vec<PrecipitationStartedEvent>) {
        (
            std::mem::take(&mut self.nucleated_events),
            std::mem::take(&mut self.precipitation_events),
        )
    }

    /// when the cell crosses the gates AND advances the per-cell cloud
    /// saturation toward the precipitation threshold. Honors the per-
    /// cell ambient pressure sample from the M19 atmospherics input.
    pub fn observe_steam_pixel(&mut self, inputs: PrecipitationInputs) {
        if let Some(evt) = evaluate_steam_nucleation(inputs) {
            // Track the underlying cloud cell (cell coords = world coords).
            let key = (inputs.world_x, inputs.world_y);
            let cell = self
                .cells
                .entry(key)
                .or_insert_with(|| CloudCell::new(inputs.world_x, inputs.world_y));
            // Update once per nucleation observation, threading the
            // pressure sample through the saturation-rate multiplier.
            if let Some(precip) = update_cloud_cell_with_pressure(
                cell,
                self.world,
                inputs.ambient_pressure_kpa,
                inputs.pollutant_fraction_local,
                inputs.tick,
            ) {
                self.precipitation_events.push(precip);
            }
            self.nucleated_events.push(evt);
        }
    }

    /// observation). Used by the orchestrator to keep already-saturated
    /// cells running toward the precipitation tick gate. Pressure
    /// defaults to Earth reference (101.325 kPa) — callers wishing to
    /// thread the M19 atmospherics sample call
    /// [`Self::step_cell_with_pressure`] instead.
    pub fn step_cell(&mut self, world_x: i32, world_y: i32, tick: u64) {
        self.step_cell_with_pressure(world_x, world_y, REFERENCE_PRESSURE_KPA, tick);
    }

    /// (kPa). Per spec § dependency — M19 atmospherics provides the
    /// pressure input that drives the pressure-rate multiplier.
    pub fn step_cell_with_pressure(&mut self, world_x: i32, world_y: i32, ambient_pressure_kpa: f32, tick: u64) {
        let key = (world_x, world_y);
        let cell = self.cells.entry(key).or_insert_with(|| CloudCell::new(world_x, world_y));
        if let Some(precip) = update_cloud_cell_with_pressure(cell, self.world, ambient_pressure_kpa, 0.0, tick) {
            self.precipitation_events.push(precip);
        }
    }

    /// Total cells currently raining (telemetry).
    #[must_use]
    pub fn raining_cells(&self) -> usize {
        self.cells.values().filter(|c| c.raining).count()
    }

    /// pass. For every nucleated steam pixel, transform it into a
    /// cloud (id=71) pixel; for every raining cloud cell, spawn one
    /// rain/acid_droplet pixel one row below the cloud position. This
    /// is the canonical "side-effect" the engine layer would invoke
    /// once per tick after running [`Self::observe_steam_pixel`] /
    /// [`Self::step_cell`] for the live steam pixel set.
    ///
    /// > And cloud material accumulates in the upper atmospheric layer
    /// > And rain droplet particles spawn falling toward the terrain
    ///
    /// Returns `(clouds_written, droplets_spawned)` for telemetry.
    pub fn apply_to_terrain(&self, terrain: &mut ChunkedTerrain, tick: u64) -> (u32, u32) {
        let mut clouds = 0u32;
        let mut droplets = 0u32;
        let width = terrain.width_px as i64;
        let height = terrain.height_px as i64;
        // Nucleation pass: transform steam pixels to cloud at the
        // nucleated position.
        for evt in &self.nucleated_events {
            let x = evt.pos[0] as i64;
            let y = evt.pos[1] as i64;
            if x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            // Only transform if the source is still steam (the CA
            // stepper may have moved the steam pixel since the event
            // was recorded).
            if terrain.material_at(x, y) == ids::STEAM {
                terrain.set_material_pixel(x, y, ids::CLOUD, tick);
                terrain.add_updated_material_area([x as f32, y as f32], [(x + 1) as f32, (y + 1) as f32]);
                clouds = clouds.saturating_add(1);
            }
        }
        // Precipitation pass: spawn droplets one row below the raining
        // cloud cell when the cell is currently raining + the row below
        // is air.
        for evt in &self.precipitation_events {
            let x = evt.pos[0] as i64;
            let y = evt.pos[1] as i64;
            let below = y + 1;
            if x < 0 || below < 0 || x >= width || below >= height {
                continue;
            }
            if terrain.material_at(x, below) == 0 {
                terrain.set_material_pixel(x, below, evt.material, tick);
                terrain.add_updated_material_area([x as f32, below as f32], [(x + 1) as f32, (below + 1) as f32]);
                droplets = droplets.saturating_add(1);
            }
        }
        (clouds, droplets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M15B-precip-001: nucleation gate at altitude ≤ 80 px blocks
    /// the event.
    #[test]
    fn steam_below_altitude_does_not_nucleate() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            50.0,
            300.0,
            AmbientWorld::Earth,
            0.0,
            1,
        );
        assert!(evaluate_steam_nucleation(inputs).is_none());
    }

    /// VAL-M15B-precip-002: nucleation gate at temp ≥ 80°C blocks.
    #[test]
    fn steam_above_temp_threshold_does_not_nucleate() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            100.0,
            400.0, // > 353.15 K
            AmbientWorld::Earth,
            0.0,
            1,
        );
        assert!(evaluate_steam_nucleation(inputs).is_none());
    }

    /// VAL-M15B-precip-003: nucleation fires above 80 px + below 80°C.
    #[test]
    fn steam_nucleates_above_threshold() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            100,
            50,
            120.0,
            290.0,
            AmbientWorld::Earth,
            0.0,
            7,
        );
        let evt = evaluate_steam_nucleation(inputs).expect("must fire");
        assert_eq!(evt.from_material, ids::STEAM);
        assert_eq!(evt.to_material, ids::CLOUD);
        assert_eq!(evt.pos, [100, 50]);
        assert_eq!(evt.tick, 7);
    }

    /// VAL-M15B-precip-004: non-steam input is ignored.
    #[test]
    fn non_steam_inputs_skip_nucleation() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::WATER, // water, not steam
            0,
            0,
            200.0,
            280.0,
            AmbientWorld::Earth,
            0.0,
            1,
        );
        assert!(evaluate_steam_nucleation(inputs).is_none());
    }

    /// VAL-M15B-precip-005: saturation rate matches world humidity.
    #[test]
    fn saturation_rate_matches_world_humidity() {
        assert!(saturation_rate_per_tick(AmbientWorld::Earth) > 0.0);
        assert!(saturation_rate_per_tick(AmbientWorld::Vulcan) > saturation_rate_per_tick(AmbientWorld::Earth));
        assert!(saturation_rate_per_tick(AmbientWorld::Mimas).abs() < f32::EPSILON);
        assert!(saturation_rate_per_tick(AmbientWorld::Mars) < saturation_rate_per_tick(AmbientWorld::Earth));
    }

    /// VAL-M15B-precip-006: cloud cell reaches saturation threshold
    /// then fires precipitation after 60-tick gate. Per spec §
    /// acceptance scenario 4.
    #[test]
    fn cloud_cell_precipitates_after_60_tick_gate() {
        let mut cell = CloudCell::new(10, 5);
        let mut precip_fired = false;
        // Run for many ticks; Earth's saturation rate of 0.0275/tick
        // crosses 0.80 around tick 30, then needs +60 more.
        for t in 0..200 {
            let evt = update_cloud_cell(&mut cell, AmbientWorld::Earth, 0.0, t);
            if let Some(e) = evt {
                precip_fired = true;
                assert!(e.saturation >= PRECIPITATION_SATURATION_THRESHOLD);
                // For Earth ambient (pollutant 0.01 < 0.05), material
                // must be regular rain.
                assert_eq!(e.material, ids::RAIN);
                break;
            }
        }
        assert!(precip_fired, "precipitation must fire within 200 ticks on Earth");
    }

    /// VAL-M15B-precip-007: precipitation gate is at least 60 ticks
    /// past the saturation crossing.
    #[test]
    fn precipitation_waits_60_tick_gate() {
        let mut cell = CloudCell::new(0, 0);
        // Force saturation high without crossing the precipitation gate.
        cell.saturation = 0.85;
        cell.saturated_at_tick = Some(1000);
        // Tick = 1059 (59 elapsed) — must NOT fire.
        let evt = update_cloud_cell(&mut cell, AmbientWorld::Earth, 0.0, 1059);
        assert!(evt.is_none(), "must not fire before 60-tick gate");
        // Tick = 1060 + something to push saturation past threshold again.
        cell.saturation = 0.85;
        let evt2 = update_cloud_cell(&mut cell, AmbientWorld::Earth, 0.0, 1060);
        assert!(evt2.is_some(), "must fire at the 60-tick gate");
    }

    /// VAL-M15B-precip-008: vulcan ambient → acid_droplet output.
    #[test]
    fn vulcan_ambient_produces_acid_droplets() {
        let mut cell = CloudCell::new(0, 0);
        let mut acid_fired = false;
        for t in 0..200 {
            if let Some(e) = update_cloud_cell(&mut cell, AmbientWorld::Vulcan, 0.0, t) {
                assert_eq!(e.material, ids::ACID_DROPLET, "Vulcan ambient must produce acid");
                assert!(e.pollutant_fraction >= ACID_RAIN_POLLUTANT_FRACTION_MIN);
                acid_fired = true;
                break;
            }
        }
        assert!(acid_fired, "Vulcan must precipitate as acid within 200 ticks");
    }

    /// VAL-M15B-precip-009: pollutant pump (e.g. industrial smoke
    /// cloud) → acid_droplet output even in Earth ambient.
    #[test]
    fn high_local_pollutant_yields_acid_droplet() {
        let mut cell = CloudCell::new(0, 0);
        let mut acid_fired = false;
        for t in 0..200 {
            // Pump pollutant per tick so the threshold crosses early.
            let evt = update_cloud_cell(&mut cell, AmbientWorld::Earth, 0.01, t);
            if let Some(e) = evt {
                assert_eq!(e.material, ids::ACID_DROPLET, "polluted cloud must produce acid");
                acid_fired = true;
                break;
            }
        }
        assert!(acid_fired);
    }

    /// VAL-M15B-precip-010: Mimas (vacuum) never precipitates.
    #[test]
    fn mimas_vacuum_never_precipitates() {
        let mut cell = CloudCell::new(0, 0);
        let mut precip_fired = false;
        for t in 0..1000 {
            if update_cloud_cell(&mut cell, AmbientWorld::Mimas, 0.0, t).is_some() {
                precip_fired = true;
                break;
            }
        }
        assert!(!precip_fired, "Mimas vacuum must never precipitate");
        assert!(cell.saturation < PRECIPITATION_SATURATION_THRESHOLD);
    }

    /// VAL-M15B-precip-011: full cycle orchestrator records both event
    /// streams.
    #[test]
    fn precipitation_cycle_records_both_event_streams() {
        let mut cycle = PrecipitationCycle::new(AmbientWorld::Vulcan);
        for t in 0..200 {
            cycle.observe_steam_pixel(PrecipitationInputs::with_default_pressure(
                ids::STEAM,
                10,
                100,
                120.0,
                290.0,
                AmbientWorld::Vulcan,
                0.0,
                t,
            ));
        }
        assert!(!cycle.nucleated_events.is_empty(), "nucleation events recorded");
        assert!(
            !cycle.precipitation_events.is_empty(),
            "Vulcan must precipitate within 200 ticks"
        );
        let (nucs, precips) = cycle.drain_events();
        assert!(!nucs.is_empty());
        assert!(!precips.is_empty());
        // After drain the buffer empties.
        assert!(cycle.nucleated_events.is_empty());
        assert!(cycle.precipitation_events.is_empty());
    }

    /// VAL-M15B-precip-012: events round-trip via serde.
    #[test]
    fn events_round_trip_via_serde() {
        let n = PhaseNucleatedEvent {
            from_material: ids::STEAM,
            to_material: ids::CLOUD,
            from: "steam".to_string(),
            to: "cloud".to_string(),
            pos: [3, 4],
            altitude_px: 100.0,
            temperature_k: 290.0,
            tick: 5,
        };
        let json = serde_json::to_string(&n).unwrap();
        let back: PhaseNucleatedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, n);

        let p = PrecipitationStartedEvent {
            pos: [0, 0],
            material: ids::ACID_DROPLET,
            saturation: 0.9,
            pollutant_fraction: 0.1,
            ambient: "vulcan".to_string(),
            tick: 12,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: PrecipitationStartedEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    /// VAL-M15B-precip-013: ambient world constants are stable.
    #[test]
    fn ambient_world_constants() {
        assert_eq!(AmbientWorld::Earth.as_str(), "earth");
        assert_eq!(AmbientWorld::Vulcan.as_str(), "vulcan");
        assert_eq!(AmbientWorld::Mimas.as_str(), "mimas");
        assert_eq!(AmbientWorld::Mars.as_str(), "mars");
        assert!(AmbientWorld::Vulcan.always_precipitates());
        assert!(!AmbientWorld::Earth.always_precipitates());
    }

    /// VAL-M15B-precip-014: ID constants match the registry literals.
    #[test]
    fn material_id_constants_match_registry() {
        assert_eq!(ids::STEAM, 50);
        assert_eq!(ids::CLOUD, 71);
        assert_eq!(ids::RAIN, 87);
        assert_eq!(ids::ACID_DROPLET, 88);
        assert_eq!(ids::WATER, 13);
        assert_eq!(ids::POLLUTANT_PROXY, 62);
    }

    /// VAL-M15B-precip-015: spec literal "material_phase_nucleated
    /// event fires with from='steam' to='cloud'". The struct emits
    /// both the numeric ids AND the string names so the JSON event is
    /// self-describing per the schema in
    /// `cf-replay/schemas/event/material_phase_nucleated.json`.
    #[test]
    fn phase_nucleated_event_carries_spec_literal_names() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            100.0,
            290.0,
            AmbientWorld::Earth,
            0.0,
            1,
        );
        let evt = evaluate_steam_nucleation(inputs).expect("fires");
        assert_eq!(evt.from, "steam", "spec literal from='steam'");
        assert_eq!(evt.to, "cloud", "spec literal to='cloud'");
        assert_eq!(evt.from_material, ids::STEAM);
        assert_eq!(evt.to_material, ids::CLOUD);
    }

    /// VAL-M15B-precip-016: material_id_to_name covers the M15B
    /// precipitation chain.
    #[test]
    fn material_id_to_name_covers_precipitation_chain() {
        assert_eq!(material_id_to_name(ids::STEAM), "steam");
        assert_eq!(material_id_to_name(ids::CLOUD), "cloud");
        assert_eq!(material_id_to_name(ids::RAIN), "rain");
        assert_eq!(material_id_to_name(ids::ACID_DROPLET), "acid_droplet");
        assert_eq!(material_id_to_name(ids::WATER), "water");
        assert_eq!(material_id_to_name(0), "air");
        assert_eq!(material_id_to_name(255), "unknown");
    }

    /// VAL-M15B-precip-017: spec § acceptance scenario 3 — "cloud
    /// material accumulates in the upper atmospheric layer". After
    /// observe_steam_pixel + apply_to_terrain, a steam pixel at the
    /// nucleation position transforms into a cloud pixel.
    #[test]
    fn apply_to_terrain_transforms_steam_to_cloud() {
        let mut terrain = cf_terrain::chunked::ChunkedTerrain::new(16, 16, 0);
        // Seed a steam pixel above the nucleation altitude (y=4 is
        // 12 px above sea level; we'll use abstract altitude in the
        // PrecipitationInputs since terrain doesn't know about world
        // altitude — the engine layer projects world coords).
        terrain.set_material_pixel(8, 4, ids::STEAM, 0);

        let mut cycle = PrecipitationCycle::new(AmbientWorld::Earth);
        cycle.observe_steam_pixel(PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            8,
            4,
            200.0,
            290.0,
            AmbientWorld::Earth,
            0.0,
            1,
        ));
        let (clouds, _) = cycle.apply_to_terrain(&mut terrain, 1);
        assert_eq!(clouds, 1, "exactly one cloud written");
        assert_eq!(
            terrain.material_at(8, 4),
            ids::CLOUD,
            "steam pixel transformed to cloud"
        );
    }

    /// VAL-M15B-precip-018: spec § acceptance scenario 4 — "rain
    /// droplet particles spawn falling toward the terrain". After a
    /// precipitation event, apply_to_terrain spawns a droplet one row
    /// below the cloud position.
    #[test]
    fn apply_to_terrain_spawns_rain_droplet_below_cloud() {
        let mut terrain = cf_terrain::chunked::ChunkedTerrain::new(16, 16, 0);
        let mut cycle = PrecipitationCycle::new(AmbientWorld::Earth);
        // Force a precipitation event via direct insertion (the
        // saturation path is tested separately).
        cycle.precipitation_events.push(PrecipitationStartedEvent {
            pos: [4, 3],
            material: ids::RAIN,
            saturation: 0.85,
            pollutant_fraction: 0.0,
            ambient: "earth".to_string(),
            tick: 60,
        });
        let (_, droplets) = cycle.apply_to_terrain(&mut terrain, 60);
        assert_eq!(droplets, 1, "exactly one rain droplet spawned");
        assert_eq!(
            terrain.material_at(4, 4),
            ids::RAIN,
            "rain pixel must spawn one row below cloud"
        );
    }

    /// VAL-M15B-precip-019: spec § acceptance scenario 5 — Vulcan
    /// precipitation spawns acid_droplet pixels into the terrain.
    #[test]
    fn apply_to_terrain_spawns_acid_droplets_on_vulcan() {
        let mut terrain = cf_terrain::chunked::ChunkedTerrain::new(16, 16, 0);
        let mut cycle = PrecipitationCycle::new(AmbientWorld::Vulcan);
        cycle.precipitation_events.push(PrecipitationStartedEvent {
            pos: [4, 3],
            material: ids::ACID_DROPLET,
            saturation: 0.85,
            pollutant_fraction: 0.10,
            ambient: "vulcan".to_string(),
            tick: 60,
        });
        let (_, droplets) = cycle.apply_to_terrain(&mut terrain, 60);
        assert_eq!(droplets, 1);
        assert_eq!(terrain.material_at(4, 4), ids::ACID_DROPLET);
    }

    /// VAL-M15B-precip-020: apply_to_terrain doesn't double-write when
    /// the source pixel has moved (CA stepper drifted the steam).
    #[test]
    fn apply_to_terrain_skips_when_source_pixel_drifted() {
        let mut terrain = cf_terrain::chunked::ChunkedTerrain::new(16, 16, 0);
        let mut cycle = PrecipitationCycle::new(AmbientWorld::Earth);
        cycle.nucleated_events.push(PhaseNucleatedEvent {
            from_material: ids::STEAM,
            to_material: ids::CLOUD,
            from: "steam".to_string(),
            to: "cloud".to_string(),
            pos: [4, 4],
            altitude_px: 100.0,
            temperature_k: 290.0,
            tick: 5,
        });
        // No steam at (4, 4) — the CA stepper "moved" it.
        let (clouds, _) = cycle.apply_to_terrain(&mut terrain, 5);
        assert_eq!(clouds, 0, "no cloud written when source drifted");
        assert_eq!(terrain.material_at(4, 4), 0);
    }

    /// VAL-M15B-precip-pressure-001: pressure rate multiplier at Earth
    /// reference (101.325 kPa) is exactly 1.0 — pressure does not
    /// change behavior for default scenarios.
    #[test]
    fn pressure_rate_multiplier_at_earth_reference_is_one() {
        let m = pressure_rate_multiplier(REFERENCE_PRESSURE_KPA);
        assert!((m - 1.0).abs() < 1e-3, "multiplier at Earth ref must be 1.0; got {m}");
    }

    /// VAL-M15B-precip-pressure-002: low pressure accelerates
    /// saturation (per real meteorology: adiabatic cooling on rising
    /// air). At 50 kPa (~5 km altitude) the multiplier is 2.0.
    #[test]
    fn low_pressure_accelerates_saturation() {
        let m_lo = pressure_rate_multiplier(50.0);
        let m_ref = pressure_rate_multiplier(REFERENCE_PRESSURE_KPA);
        assert!(m_lo > m_ref, "low pressure must accelerate saturation");
        assert!((m_lo - 2.0).abs() < 1e-3, "50 kPa should clamp at 2.0; got {m_lo}");
    }

    /// VAL-M15B-precip-pressure-003: high pressure slows saturation
    /// (per real meteorology: high-pressure systems are typically
    /// fair-weather). At 200 kPa (pressurized dome) the multiplier is
    /// ~0.506 (101.325/200); at 300 kPa it clamps at the 0.5 floor.
    #[test]
    fn high_pressure_slows_saturation() {
        let m_200 = pressure_rate_multiplier(200.0);
        let m_300 = pressure_rate_multiplier(300.0);
        let m_ref = pressure_rate_multiplier(REFERENCE_PRESSURE_KPA);
        assert!(m_200 < m_ref, "200 kPa must slow saturation");
        assert!(m_300 < m_ref, "300 kPa must slow saturation");
        assert!(
            (m_300 - PRESSURE_MULTIPLIER_RANGE.0).abs() < 1e-3,
            "300 kPa must clamp at the {:?} floor; got {m_300}",
            PRESSURE_MULTIPLIER_RANGE.0
        );
    }

    /// VAL-M15B-precip-pressure-004: vacuum / near-zero pressure
    /// clamps the multiplier to the upper bound (no divide-by-zero).
    #[test]
    fn vacuum_pressure_clamps_at_upper_bound() {
        let m = pressure_rate_multiplier(0.0);
        assert!((m - PRESSURE_MULTIPLIER_RANGE.1).abs() < 1e-3);
        let m = pressure_rate_multiplier(-5.0);
        assert!((m - PRESSURE_MULTIPLIER_RANGE.1).abs() < 1e-3);
    }

    /// VAL-M15B-precip-pressure-005: nucleation pressure gate blocks
    /// near-vacuum (< 1 kPa) per spec § "Mimas = never rain (vacuum)".
    #[test]
    fn nucleation_pressure_gate_blocks_vacuum() {
        let mut inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            120.0,
            290.0,
            AmbientWorld::Mimas,
            0.0,
            1,
        );
        inputs.ambient_pressure_kpa = 0.001; // near-vacuum (Mimas)
        assert!(evaluate_steam_nucleation(inputs).is_none());
    }

    /// VAL-M15B-precip-pressure-006: nucleation pressure gate allows
    /// thin atmospheres (>= 1 kPa) — e.g., Mars at ~0.6 kPa is gated,
    /// but a thicker-atmosphere world at 5 kPa passes.
    #[test]
    fn nucleation_pressure_gate_allows_thin_atmosphere() {
        let mut inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            120.0,
            290.0,
            AmbientWorld::Mars,
            0.0,
            1,
        );
        inputs.ambient_pressure_kpa = 5.0;
        assert!(evaluate_steam_nucleation(inputs).is_some());
    }

    /// VAL-M15B-precip-pressure-007: saturation_rate_per_tick_with_pressure
    /// honors the M19 pressure input. Low pressure → faster saturation
    /// for the same humidity.
    #[test]
    fn saturation_rate_with_pressure_modulates_baseline() {
        let earth_base = saturation_rate_per_tick(AmbientWorld::Earth);
        let earth_ref = saturation_rate_per_tick_with_pressure(AmbientWorld::Earth, REFERENCE_PRESSURE_KPA);
        let earth_lo = saturation_rate_per_tick_with_pressure(AmbientWorld::Earth, 50.0);
        assert!((earth_ref - earth_base).abs() < 1e-6, "Earth ref pressure == baseline");
        assert!(earth_lo > earth_base, "low pressure must accelerate");
    }

    /// VAL-M15B-precip-pressure-008: explicit pressure input changes
    /// when precipitation fires. Pressurized dome (200 kPa) saturates
    /// slower than Earth sea level; the precipitation event arrives
    /// later in tick count.
    #[test]
    fn high_pressure_delays_precipitation_event() {
        fn ticks_until_precip(pressure_kpa: f32) -> Option<u64> {
            let mut cell = CloudCell::new(0, 0);
            for t in 0u64..1000 {
                if let Some(_evt) =
                    update_cloud_cell_with_pressure(&mut cell, AmbientWorld::Earth, pressure_kpa, 0.0, t)
                {
                    return Some(t);
                }
            }
            None
        }
        let t_earth = ticks_until_precip(REFERENCE_PRESSURE_KPA).expect("Earth precipitates");
        let t_dome = ticks_until_precip(200.0).expect("dome precipitates");
        assert!(
            t_dome > t_earth,
            "high pressure (dome) must delay precipitation; earth={t_earth} dome={t_dome}"
        );
    }

    /// VAL-M15B-precip-pressure-009: PrecipitationInputs::with_default_pressure
    /// produces the spec-locked Earth reference pressure.
    #[test]
    fn with_default_pressure_uses_earth_reference() {
        let inputs = PrecipitationInputs::with_default_pressure(
            ids::STEAM,
            0,
            0,
            120.0,
            290.0,
            AmbientWorld::Earth,
            0.0,
            1,
        );
        assert!((inputs.ambient_pressure_kpa - REFERENCE_PRESSURE_KPA).abs() < 1e-3);
    }

    /// VAL-M15B-precip-021: apply_to_terrain respects world bounds.
    #[test]
    fn apply_to_terrain_respects_world_bounds() {
        let mut terrain = cf_terrain::chunked::ChunkedTerrain::new(8, 8, 0);
        let mut cycle = PrecipitationCycle::new(AmbientWorld::Earth);
        cycle.precipitation_events.push(PrecipitationStartedEvent {
            pos: [4, 7], // at the bottom edge
            material: ids::RAIN,
            saturation: 0.85,
            pollutant_fraction: 0.0,
            ambient: "earth".to_string(),
            tick: 60,
        });
        let (_, droplets) = cycle.apply_to_terrain(&mut terrain, 60);
        assert_eq!(droplets, 0, "off-world droplet must be dropped");
    }
}
