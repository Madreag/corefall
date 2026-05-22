//! cf-localization — M8 keyed strings table + ICU MessageFormat scaffold.
//!
//! Per spec § Localization (cf-localization NEW crate): the launch keys
//! cover HUD labels, banners, captions, settings labels, tooltips,
//! weapon/tool names, command labels, achievement names, and error
//! messages. English baseline ships at M8; Tier-A 11 languages reserved
//! for T-ACC-PLUS BP9+. ICU MessageFormat compliance is forward-compat —
//! the M8 implementation honors `{placeholder}` substitution and the
//! `{count, plural, one {...} other {...}}` form.
//!
//! The English defaults live in `game/content/localization/en.json` and
//! are bundled at compile time via `include_str!` so cf-app can render a
//! correct HUD on a fresh checkout without an external file load. cfctl
//! `observe.localization.current_language` returns the active language
//! code from the engine.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

pub mod icu_format;
pub mod keyed_strings;

pub use icu_format::{format_plural, format_with_args, parse_plural_form, PluralForm};
pub use keyed_strings::{LocalizationLoadError, LocalizationTable, EN_LANGUAGE_CODE, EN_TABLE_BYTES};
