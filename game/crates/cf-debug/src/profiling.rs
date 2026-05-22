//! M8A § Files / Profiling — Tracy / Puffin profiling hooks scaffold.
//!
//! Per M8A spec § Files / Profiling: optional Tracy / Puffin integration
//! behind feature flags. Off by default (zero cost in release).
//!
//! M8A ships the scaffold + named feature surfaces so M9+ can wire the
//! actual `profiling` crate deps without churning the workspace lock
//! file at the M8A close gate. The integration plan:
//!
//! - `game/Cargo.toml` adds `profiling = "1.0"` with optional features
//!   `tracy` + `puffin` (M9+).
//! - Every tick-hot crate (`cf-actor`, `cf-ai`, `cf-physics`,
//!   `cf-terrain`, `cf-replay`, `cf-mission`, `cf-control`) gains a
//!   `[features] profiling-tracy = ["dep:profiling/tracy"]`,
//!   `profiling-puffin = ["dep:profiling/puffin"]`.
//! - Tick-hot entry points (`tick_actor`, `tick_ai_guard`,
//!   `tick_projectile`, `apply_dirty_chunks`, `merge_shards_canonical`,
//!   `tick_phase`, `drive_tick`) are annotated `#[profiling::function]`.
//! - `cargo build --workspace --release` (no profiling feature) compiles
//!   with all `profiling::function` macros as zero-cost no-ops; this is
//!   true at M8A by construction because the deps aren't pulled.
//!
//! Until M9+, the `profiling_zone!` and `profiling_function!` macros
//! below are no-ops at the source level (the crate has no profiling
//! dep), so any call site using them compiles whether profiling is
//! enabled or not.

/// M9+ wires the real `profiling::scope!` macro behind a feature flag.
#[macro_export]
macro_rules! cf_profiling_zone {
    ($name:literal) => {
        let _cf_profiling_zone = ();
    };
    ($name:literal, $($field:tt)*) => {
        let _cf_profiling_zone = ();
    };
}

/// callers should annotate tick-hot entry points using
/// `cf_profiling_zone!("<name>")` at the function body; M9+ migrates to
/// the proper `#[profiling::function]` attribute when the dep lands.
pub const M8A_PROFILING_SCAFFOLD_NOTE: &str =
    "Profiling hooks shipped as scaffolds at M8A; M9+ wires the profiling crate dep.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_macro_compiles() {
        cf_profiling_zone!("test_zone");
        cf_profiling_zone!("test_zone_fields", x = 1, y = 2);
    }

    #[test]
    fn scaffold_note_documents_integration_plan() {
        assert!(M8A_PROFILING_SCAFFOLD_NOTE.contains("M9+"));
    }
}
