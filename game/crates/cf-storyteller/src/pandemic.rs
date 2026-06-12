//! M16B § Pandemic outbreak event registration + declaration.
//!
//! A [`PandemicTracker`] wraps the deterministic `cf_disease::PandemicMonitor`
//! sliding-window detector. When the infected/total ratio holds above the
//! threshold for the contiguous window, it emits a [`PandemicDeclaredEvent`]
//! and the engine drives the base lockdown (M19E auto-quarantine + M28D
//! airlock dampers + the M11 chatter line). M25's storyteller runtime
//! consumes the registered narrative id to fire the outbreak beat.

use std::collections::BTreeMap;

use cf_disease::{DiseaseKind, PandemicMonitor};
use serde::{Deserialize, Serialize};

/// Narrative event id (locked string) M25 subscribes to.
pub const NARRATIVE_EVENT_ID_PANDEMIC_DECLARED: &str = "narrative.m16b.pandemic_declared";

/// M11 chatter ticker line surfaced when a pandemic locks down the base.
pub const PANDEMIC_LOCKDOWN_CHATTER: &str = "PANDEMIC: locking down quarters";

/// `disease.pandemic_declared` payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PandemicDeclaredEvent {
    pub tick: u64,
    pub strain: DiseaseKind,
    pub infected_count: u32,
    pub total_count: u32,
    pub infected_fraction: f32,
    /// True when the declaration triggers a base-wide lockdown (always true
    /// for the Class A pandemic strains).
    pub base_lockdown: bool,
}

/// Wraps the deterministic monitor with the strain under watch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PandemicTracker {
    pub strain: DiseaseKind,
    pub monitor: PandemicMonitor,
}

impl PandemicTracker {
    pub fn new(strain: DiseaseKind) -> Self {
        Self {
            strain,
            monitor: PandemicMonitor::default(),
        }
    }

    pub fn with_monitor(strain: DiseaseKind, monitor: PandemicMonitor) -> Self {
        Self { strain, monitor }
    }

    /// Feed one tick's infected/total tally. Returns the declared event on
    /// the tick the pandemic is first declared.
    pub fn observe(
        &mut self,
        infected: u32,
        total: u32,
        tick: u64,
        tick_rate_hz: u32,
    ) -> Option<PandemicDeclaredEvent> {
        if self.monitor.observe(infected, total, tick, tick_rate_hz) {
            let infected_fraction = if total == 0 {
                0.0
            } else {
                infected as f32 / total as f32
            };
            Some(PandemicDeclaredEvent {
                tick,
                strain: self.strain,
                infected_count: infected,
                total_count: total,
                infected_fraction,
                base_lockdown: true,
            })
        } else {
            None
        }
    }
}

/// Base-wide lockdown actions triggered by a pandemic declaration. The
/// engine consumes these to drive M19E / M28D / M11.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaseLockdown {
    /// M19E: auto-quarantine triggers across all rooms.
    pub auto_quarantine_all_rooms: bool,
    /// M28D: close the damper on every airlock.
    pub close_all_airlocks: bool,
    /// M11 chatter ticker line.
    pub chatter_line: String,
}

/// Build the base lockdown action set for a pandemic declaration.
pub fn declare_base_lockdown() -> BaseLockdown {
    BaseLockdown {
        auto_quarantine_all_rooms: true,
        close_all_airlocks: true,
        chatter_line: PANDEMIC_LOCKDOWN_CHATTER.to_string(),
    }
}

/// One registered pandemic narrative hook (mirrors `M16NarrativeRegistration`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PandemicNarrativeRegistration {
    pub narrative_event_id: String,
    pub default_intensity: f32,
}

/// Registry of pandemic narrative event ids for M25 storyteller directors.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PandemicNarrativeRegistry {
    pub by_id: BTreeMap<String, PandemicNarrativeRegistration>,
}

impl PandemicNarrativeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, registration: PandemicNarrativeRegistration) {
        self.by_id
            .insert(registration.narrative_event_id.clone(), registration);
    }

    pub fn get(&self, id: &str) -> Option<&PandemicNarrativeRegistration> {
        self.by_id.get(id)
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

/// Register the pandemic narrative beat (high-intensity outbreak spike).
pub fn register_pandemic_narratives(registry: &mut PandemicNarrativeRegistry) {
    registry.register(PandemicNarrativeRegistration {
        narrative_event_id: NARRATIVE_EVENT_ID_PANDEMIC_DECLARED.to_string(),
        default_intensity: 0.95,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pandemic_declares_after_24h_window_and_locks_down() {
        let mut tracker = PandemicTracker::new(DiseaseKind::InfluenzaPandemic);
        let window_ticks = (86_400.0_f32 * 60.0) as u64;
        let mut declared = None;
        for tick in 0..=window_ticks + 2 {
            // 15% infected, sustained.
            if let Some(ev) = tracker.observe(15, 100, tick, 60) {
                declared = Some(ev);
                break;
            }
        }
        let ev = declared.expect("pandemic must declare after the 24h window");
        assert_eq!(ev.strain, DiseaseKind::InfluenzaPandemic);
        assert!(ev.base_lockdown);
        assert!((ev.infected_fraction - 0.15).abs() < 1e-6);

        let lockdown = declare_base_lockdown();
        assert!(lockdown.auto_quarantine_all_rooms);
        assert!(lockdown.close_all_airlocks);
        assert_eq!(lockdown.chatter_line, "PANDEMIC: locking down quarters");
    }

    #[test]
    fn below_threshold_never_declares() {
        let mut tracker = PandemicTracker::new(DiseaseKind::Flu);
        let window_ticks = (86_400.0_f32 * 60.0) as u64;
        for tick in 0..=window_ticks + 100 {
            assert!(tracker.observe(5, 100, tick, 60).is_none());
        }
    }

    #[test]
    fn declaration_is_latched_once() {
        let mut tracker = PandemicTracker::new(DiseaseKind::InfluenzaPandemic);
        let window_ticks = (86_400.0_f32 * 60.0) as u64;
        let mut count = 0;
        for tick in 0..=window_ticks + 500 {
            if tracker.observe(20, 100, tick, 60).is_some() {
                count += 1;
            }
        }
        assert_eq!(count, 1, "pandemic.declared must fire exactly once");
    }

    #[test]
    fn registry_populates() {
        let mut reg = PandemicNarrativeRegistry::new();
        register_pandemic_narratives(&mut reg);
        assert!(reg.get(NARRATIVE_EVENT_ID_PANDEMIC_DECLARED).is_some());
        assert_eq!(reg.len(), 1);
    }
}
