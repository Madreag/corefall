//! Run id + event id helpers per the run-bundle naming convention.

use blake3::Hasher;

/// Format a run id from milestone, started-at UTC, and seed bytes.
///
/// Layout: `<milestone>_<UTC ISO with hyphens>_<8-char blake3 short hash>`.
/// Example: `m0_2026-05-04T22-30-00Z_a1b2c3d4`.
pub fn make_run_id(milestone: &str, started_at_iso_hyphen_safe: &str, seed: u64, scenario: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(milestone.as_bytes());
    hasher.update(started_at_iso_hyphen_safe.as_bytes());
    hasher.update(scenario.as_bytes());
    hasher.update(&seed.to_le_bytes());
    let digest = hasher.finalize();
    let short = hex::encode(&digest.as_bytes()[..4]);
    format!("{milestone}_{started_at_iso_hyphen_safe}_{short}")
}

/// Format the hyphen-safe ISO-8601 string (`YYYY-MM-DDTHH-MM-SSZ`) used in run ids and dirs.
pub fn iso_hyphen_safe(now: chrono::DateTime<chrono::Utc>) -> String {
    now.format("%Y-%m-%dT%H-%M-%SZ").to_string()
}

/// Compose an event id `<run_id>:<tick>:<seq>` exactly as the schema requires.
pub fn make_event_id(run_id: &str, tick: u64, seq: u64) -> String {
    format!("{run_id}:{tick}:{seq}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_id_is_stable_for_inputs() {
        let a = make_run_id("m0", "2026-05-05T01-02-03Z", 42, "m0_blank");
        let b = make_run_id("m0", "2026-05-05T01-02-03Z", 42, "m0_blank");
        assert_eq!(a, b);
        assert!(a.starts_with("m0_2026-05-05T01-02-03Z_"));
        assert_eq!(a.len(), "m0_2026-05-05T01-02-03Z_".len() + 8);
    }

    #[test]
    fn run_id_changes_with_seed() {
        let a = make_run_id("m0", "2026-05-05T01-02-03Z", 1, "m0_blank");
        let b = make_run_id("m0", "2026-05-05T01-02-03Z", 2, "m0_blank");
        assert_ne!(a, b);
    }

    #[test]
    fn event_id_format() {
        let id = make_event_id("m0_2026-05-05T01-02-03Z_aabbccdd", 42, 7);
        assert_eq!(id, "m0_2026-05-05T01-02-03Z_aabbccdd:42:7");
    }
}
