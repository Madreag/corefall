//! cf-material::registry — canonical home for [`MaterialDef`] +
//! [`MaterialRegistry`] re-exports per M12B spec § Files literal
//! (`game/crates/cf-material/src/registry.rs`).
//!
//! The actual struct definitions live in [`crate::lib`] for backward
//! compatibility with the M2-vintage `cf_material::MaterialDef`
//! shorthand path. This module re-exports them so consumers reaching
//! for `cf_material::registry::MaterialDef` (the path matching the spec
//! `cf-material::registry` table entry) resolve cleanly.

pub use crate::loader::{
    load_registry_from_file, validate_registry, validate_registry_json, RegistryLoadError, RegistryValidationError,
    RegistryValidationReport,
};
pub use crate::{
    AcousticDefaults, AcousticProfile, MaterialDef, MaterialId, MaterialRegistry, PhaseChange,
    LAUNCH_MATERIAL_IDS, LAUNCH_MATERIAL_NAMES, MATERIAL_SCHEMA_VERSION,
};
