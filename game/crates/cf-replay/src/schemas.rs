//! **M1 Gap H**: per-event JSON schema validation for the prototype-recorder
//! event payloads.
//!
//! The full schemas live under `cf-replay/schemas/event/` (one JSON file per
//! `(category, event_type)` pair). The validator here is intentionally a
//! minimal "required field present + type matches" check rather than a
//! full draft-2020-12 implementation: pulling in a JSON Schema crate just
//! to assert payload shapes would balloon the dependency surface for a
//! benefit M1 doesn't need. The validator handles:
//!
//! - `required` array (every listed field MUST exist in the payload).
//! - per-field `type` (`object`, `array`, `string`, `number`, `integer`,
//!   `boolean`; arrays of types are interpreted as a union).
//! - `minItems` + `maxItems` on arrays.
//! - `minimum` on numeric values.
//! - `enum` on strings.
//!
//! `additionalProperties: true` is implicit — payloads may carry extra
//! fields beyond the schema without rejection (the recorder envelope is
//! intentionally extensible).
//!
//! `cf-mod validate-bundle` calls `validate_event_payload` on every event
//! in a run bundle; the workspace test under `cf-replay/tests` walks a
//! freshly-recorded smoke bundle to prove the schemas accept real events.
//!
//! This file is a facade: raw schema sources live in `schemas_consts.rs`,
//! the `(category, event_type) → schema` lookup table in `schemas_lookup.rs`,
//! and the validator walker in `schemas_validate.rs`. Public API is
//! re-exported from this module so external callers continue to use
//! `cf_replay::schemas::{event_schema_for, validate_event_payload,
//! ValidationResult}`.

pub use crate::schemas_lookup::event_schema_for;
pub use crate::schemas_validate::{validate_event_payload, ValidationResult};
