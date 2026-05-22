//! **M14I** § Per-veteran narrative tab (consumed by M48C pilot dossier).
//!
//! Renders the per-veteran long-term-consequence dossier:
//! - **Header** — display name + origin + age + retirement status.
//! - **Scar timeline** — chronological list of every ScarRecord with
//!   wound kind / zone / closure method / functional debuff.
//! - **Prosthetic registry** — per-installed prosthetic kind / tier /
//!   zone / wear / malfunction flag.
//! - **Trait list** — phantom_limb / memory_loss_* / chronic_* / retired.
//! - **Aging panel** — caloric_max / max_speed / heal_rate decay
//!   percentages.
//! - **Chronic pain baseline** — cumulative pain points.
//! - **Radiation dose** — cumulative dose + cancer handoff flag.
//!
//! The renderer is text-only (`String`-returning helpers) so cf-app's
//! Bevy bridge or the cfctl `observe` reader can both consume the same
//! surface. cf-ui's bevy components are not used here — this is a pure
//! data → string transform that the dossier panel renders.

use cf_actor::long_term::{LongTermState, ZoneLongTermState};
use cf_actor::traits::ids as trait_ids;
use cf_aging::BiologicalAge;
use cf_prosthetic::ProstheticInstance;
use cf_scar::ScarRecord;
use cf_veteran::VeteranDossier;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VeteranDossierView {
    pub actor_id: u64,
    pub display_name: String,
    pub origin_label: String,
    pub age_summary: String,
    pub retirement_state: String,
    pub scar_rows: Vec<ScarRow>,
    pub prosthetic_rows: Vec<ProstheticRow>,
    pub zone_rows: Vec<ZoneRow>,
    pub trait_rows: Vec<String>,
    pub aging_rows: Vec<String>,
    pub chronic_pain_baseline: f32,
    pub cumulative_radiation_dose: f32,
    pub cancer_handoff_fired: bool,
    pub concussion_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScarRow {
    pub scar_id: u64,
    pub wound_kind: String,
    pub zone: String,
    pub severity_at_close: f32,
    pub closure_method: String,
    pub functional_debuff: String,
    pub tick_acquired: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProstheticRow {
    pub kind: String,
    pub tier: String,
    pub zone: String,
    pub wear_pct: f32,
    pub malfunctioning: bool,
    pub installed_tick: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ZoneRow {
    pub zone: String,
    pub state: String,
}

/// state + optional VeteranDossier (e.g. retired-veteran persistence).
#[must_use]
pub fn build_view(
    actor_id: u64,
    display_name: &str,
    origin_label: &str,
    lt: &LongTermState,
    persisted: Option<&VeteranDossier>,
) -> VeteranDossierView {
    let mut view = VeteranDossierView {
        actor_id,
        display_name: display_name.to_string(),
        origin_label: origin_label.to_string(),
        ..VeteranDossierView::default()
    };
    view.age_summary = age_summary(lt.biological_age.as_ref());
    view.retirement_state = retirement_state(lt, persisted);
    view.scar_rows = lt
        .scar_timeline
        .iter()
        .map(scar_row)
        .collect();
    view.prosthetic_rows = lt.prosthetics.iter().map(prosthetic_row).collect();
    view.zone_rows = lt
        .zone_states()
        .into_iter()
        .map(|(zone, state)| ZoneRow {
            zone: zone.as_str().to_string(),
            state: state.as_str().to_string(),
        })
        .collect();
    view.trait_rows = lt.traits.iter().cloned().collect();
    view.aging_rows = aging_rows(lt.biological_age.as_ref());
    view.chronic_pain_baseline = lt.chronic_pain_baseline;
    view.cumulative_radiation_dose = lt.cumulative_radiation_dose;
    view.cancer_handoff_fired = lt.cancer_handoff_fired;
    view.concussion_count = lt.concussion_count;
    view
}

fn scar_row(s: &ScarRecord) -> ScarRow {
    ScarRow {
        scar_id: s.scar_id.raw(),
        wound_kind: s.source_wound_kind.as_str().to_string(),
        zone: s.zone.as_str().to_string(),
        severity_at_close: s.severity_at_close,
        closure_method: s.closure_method.as_str().to_string(),
        functional_debuff: s.functional_debuff.tag().to_string(),
        tick_acquired: s.tick_acquired,
    }
}

fn prosthetic_row(p: &ProstheticInstance) -> ProstheticRow {
    ProstheticRow {
        kind: p.kind.as_str().to_string(),
        tier: p.tier.as_str().to_string(),
        zone: p.zone.as_str().to_string(),
        wear_pct: p.wear_pct,
        malfunctioning: p.malfunctioning,
        installed_tick: p.installed_tick,
    }
}

fn age_summary(age: Option<&BiologicalAge>) -> String {
    match age {
        Some(a) => format!(
            "Age {:.1} ({}) — retirement {:.0}, terminal {:.0}",
            a.age_in_game_years,
            a.origin.as_str(),
            a.retirement_age,
            a.terminal_age
        ),
        None => "Age: unknown".to_string(),
    }
}

fn aging_rows(age: Option<&BiologicalAge>) -> Vec<String> {
    let mut rows = Vec::new();
    if let Some(a) = age {
        rows.push(format!(
            "caloric_max decay: {:.2}%",
            a.caloric_max_decay * 100.0
        ));
        rows.push(format!(
            "max_speed decay: {:.2}%",
            a.max_speed_decay * 100.0
        ));
        rows.push(format!(
            "wound_heal_rate decay: {:.2}%",
            a.heal_rate_decay * 100.0
        ));
        if a.terminal_age_reached {
            rows.push(format!(
                "terminal-age reached — {} terminal rolls fired",
                a.terminal_rolls_fired
            ));
        }
        if a.died_of_old_age {
            rows.push("Died of old age".to_string());
        }
    }
    rows
}

fn retirement_state(lt: &LongTermState, persisted: Option<&VeteranDossier>) -> String {
    if let Some(d) = persisted {
        if d.retired {
            return format!("Retired at tick {} (advisor NPC)", d.retired_tick);
        }
    }
    if lt.retired {
        format!("Retired at tick {} (advisor NPC)", lt.retired_tick)
    } else if lt.retirement_offered {
        "Retirement offered — awaiting decision".to_string()
    } else if lt.traits.has(trait_ids::RETIRED_VETERAN) {
        "Retired".to_string()
    } else {
        "Active veteran".to_string()
    }
}

/// roster row).
#[must_use]
pub fn render_summary_line(view: &VeteranDossierView) -> String {
    format!(
        "{} | {} | scars {} | prosthetics {} | concussions {} | {}",
        view.display_name,
        view.age_summary,
        view.scar_rows.len(),
        view.prosthetic_rows.len(),
        view.concussion_count,
        view.retirement_state
    )
}

/// by cfctl observe + headless UI).
#[must_use]
pub fn render_dossier(view: &VeteranDossierView) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Veteran #{}: {} ({})\n",
        view.actor_id, view.display_name, view.origin_label
    ));
    out.push_str(&format!("  {}\n", view.age_summary));
    out.push_str(&format!("  Status: {}\n", view.retirement_state));
    if !view.aging_rows.is_empty() {
        out.push_str("  Aging:\n");
        for row in &view.aging_rows {
            out.push_str(&format!("    - {row}\n"));
        }
    }
    if view.concussion_count > 0 {
        out.push_str(&format!(
            "  Concussion count: {}\n",
            view.concussion_count
        ));
    }
    if view.chronic_pain_baseline > 0.0 {
        out.push_str(&format!(
            "  Chronic pain baseline: {:.1} pts\n",
            view.chronic_pain_baseline
        ));
    }
    if view.cumulative_radiation_dose > 0.0 {
        out.push_str(&format!(
            "  Radiation dose: {:.2} (cancer handoff: {})\n",
            view.cumulative_radiation_dose,
            if view.cancer_handoff_fired { "fired" } else { "pending" }
        ));
    }
    if !view.scar_rows.is_empty() {
        out.push_str("  Scar timeline:\n");
        for r in &view.scar_rows {
            out.push_str(&format!(
                "    - #{} {} @ {} sev={:.2} closure={} debuff={}\n",
                r.scar_id,
                r.wound_kind,
                r.zone,
                r.severity_at_close,
                r.closure_method,
                r.functional_debuff
            ));
        }
    }
    if !view.prosthetic_rows.is_empty() {
        out.push_str("  Prosthetics:\n");
        for p in &view.prosthetic_rows {
            out.push_str(&format!(
                "    - {} ({}) @ {} wear={:.2}{}\n",
                p.kind,
                p.tier,
                p.zone,
                p.wear_pct,
                if p.malfunctioning { " MALFUNCTIONING" } else { "" }
            ));
        }
    }
    if !view.zone_rows.is_empty() {
        out.push_str("  Zone states:\n");
        for z in &view.zone_rows {
            out.push_str(&format!("    - {} : {}\n", z.zone, z.state));
        }
    }
    if !view.trait_rows.is_empty() {
        out.push_str("  Traits:\n");
        for t in &view.trait_rows {
            out.push_str(&format!("    - {}\n", t));
        }
    }
    out
}

#[allow(dead_code)]
const _ZONE_LINK: ZoneLongTermState = ZoneLongTermState::Intact;

#[cfg(test)]
mod tests {
    use super::*;
    use cf_aging::AgingOrigin;
    use cf_scar::{FunctionalDebuff, ScarRecord, ScarId};
    use cf_wound::registry::{TreatmentKind, VisualDecalId, ZoneId};
    use cf_wound::WoundKind;

    fn make_lt_with_scar() -> LongTermState {
        let mut lt = LongTermState::new();
        lt.biological_age = Some(BiologicalAge::new_for_origin(AgingOrigin::Human, 42.0));
        let scar = ScarRecord::new(
            ScarId(1),
            WoundKind::LacerationSevere,
            ZoneId::from("arm_left"),
            0.8,
            TreatmentKind::SutureKit,
            100,
            VisualDecalId::from("scar_suture_line"),
        );
        lt.scar_timeline.scars.push(scar);
        lt.aggregate.add_debuff(&FunctionalDebuff::ReducedZoneStrength {
            zone: ZoneId::from("arm_left"),
            pct: 0.05,
        });
        lt
    }

    #[test]
    fn view_includes_scar_row() {
        let lt = make_lt_with_scar();
        let view = build_view(7, "Hawthorne", "human", &lt, None);
        assert_eq!(view.scar_rows.len(), 1);
        assert_eq!(view.scar_rows[0].wound_kind, "LacerationSevere");
        assert_eq!(view.scar_rows[0].functional_debuff, "reduced_zone_strength");
    }

    #[test]
    fn render_dossier_contains_key_sections() {
        let lt = make_lt_with_scar();
        let view = build_view(1, "Lt. Hawthorne", "human", &lt, None);
        let text = render_dossier(&view);
        assert!(text.contains("Lt. Hawthorne"));
        assert!(text.contains("LacerationSevere"));
        assert!(text.contains("Age "));
    }

    #[test]
    fn render_summary_line_shape() {
        let lt = make_lt_with_scar();
        let view = build_view(1, "Hawthorne", "human", &lt, None);
        let line = render_summary_line(&view);
        assert!(line.contains("Hawthorne"));
        assert!(line.contains("scars 1"));
    }

    #[test]
    fn retirement_state_reflects_long_term() {
        let mut lt = LongTermState::new();
        lt.retirement_offered = true;
        let v1 = build_view(1, "n", "human", &lt, None);
        assert!(v1.retirement_state.contains("Retirement offered"));
        lt.retired = true;
        lt.retired_tick = 999;
        let v2 = build_view(1, "n", "human", &lt, None);
        assert!(v2.retirement_state.contains("Retired"));
    }
}
