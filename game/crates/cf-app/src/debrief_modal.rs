//! M10B post-mission debrief modal — Export Last Replay CTA.
//!
//! Spec § "Notes for the implementer":
//!
//! > The "Export Last Replay" CTA in the post-mission debrief modal
//! > (cf-app) defaults to the `clip_compact` preset; one-click export
//! > to `~/Movies/Corefall/` on macOS, `~/Videos/Corefall/` on Linux,
//! > `~/Videos/Corefall/` on Windows. Path resolution uses `dirs-next`.
//!
//! VAL-M10B-DEBRIEF-CTA: "The cf-app post-mission debrief modal
//! contains an 'Export Last Replay' CTA button that, when activated,
//! spawns `cf-tools-replay-viewer export` against the current run
//! bundle using the `clip_compact` preset by default."
//!
//! This module exposes a small DTO-style abstraction (no Bevy / egui
//! types) so the validation test can headlessly assert:
//!
//! - The button is in the modal's widget tree.
//! - The button's documented id / label match.
//! - The button's on-click dispatch produces the argv
//!   `cf-tools-replay-viewer export <bundle> --preset clip_compact
//!   --out <platform_default_path>`.
//!
//! The Bevy / egui front-end consumes this DTO to render the actual
//! pixels (m10b-2 / m10b-3 land the rendering glue); the DTO + the
//! dispatch logic stay in the library so cargo tests don't need to
//! spin up a Bevy app to verify the CTA contract.

use std::path::{Path, PathBuf};

use cf_tools_replay_viewer::export_cmd::build_cta_argv;

/// Canonical id for the Export Last Replay CTA button. Read by the
/// VAL-M10B-DEBRIEF-CTA test + the Bevy / egui front-end so the
/// widget id stays stable across binary updates.
pub const EXPORT_LAST_REPLAY_BUTTON_ID: &str = "debrief_modal.export_last_replay";

/// Canonical human-readable label for the Export Last Replay CTA.
/// Routed through `cf-localization` at render time; the DTO carries
/// the canonical en-US string so the dispatch contract is stable.
pub const EXPORT_LAST_REPLAY_BUTTON_LABEL: &str = "Export Last Replay";

/// Default preset name the CTA dispatches with. Spec § Notes:
/// "defaults to the `clip_compact` preset".
pub const EXPORT_LAST_REPLAY_DEFAULT_PRESET: &str = "clip_compact";

/// Canonical `cf-tools-replay-viewer` binary name. The Bevy / egui
/// frontend resolves this via PATH + a colocated-binary search in the
/// production build; the DTO carries the canonical name so the
/// dispatch contract stays stable.
pub const VIEWER_BIN_NAME: &str = "cf-tools-replay-viewer";

/// One button entry inside the debrief modal's widget tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebriefModalButton {
    pub id: String,
    pub label: String,
    /// `Some(...)` when the button has an on-click dispatch (the
    /// Export Last Replay CTA); `None` for cosmetic / pass-through
    /// buttons.
    pub on_click: Option<ExportCtaDispatch>,
}

/// Dispatch contract for the Export Last Replay CTA. Captures the
/// argv the on-click handler passes to `std::process::Command` so the
/// validation test can assert the dispatch is correct WITHOUT
/// spawning a real subprocess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCtaDispatch {
    pub binary: String,
    pub argv: Vec<String>,
    pub bundle_dir: PathBuf,
    pub preset: String,
    pub out_path: PathBuf,
}

impl ExportCtaDispatch {
    /// Build the dispatch payload for a given run bundle. Resolves
    /// the platform-default `--out` path via dirs-next per spec §
    /// Notes; if dirs-next can't resolve a platform directory (CI /
    /// sandboxed envs without HOME) falls back to `./<run_id>.mp4`
    /// next to the binary's CWD.
    #[must_use]
    pub fn new(bundle_dir: &Path, run_id: &str) -> Self {
        let argv = build_cta_argv(bundle_dir, run_id);
        // `argv` shape: ["export", <bundle>, "--preset", "clip_compact",
        //                "--out", <platform_default_path>]
        let preset = argv
            .iter()
            .position(|s| s == "--preset")
            .and_then(|i| argv.get(i + 1).cloned())
            .unwrap_or_else(|| EXPORT_LAST_REPLAY_DEFAULT_PRESET.into());
        let out_path = argv
            .iter()
            .position(|s| s == "--out")
            .and_then(|i| argv.get(i + 1).cloned())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(format!("{run_id}.mp4")));
        Self {
            binary: VIEWER_BIN_NAME.into(),
            argv,
            bundle_dir: bundle_dir.to_path_buf(),
            preset,
            out_path,
        }
    }
}

/// The debrief modal's widget tree DTO. Holds the buttons the modal
/// renders + the active run bundle context. The Bevy / egui frontend
/// reads off this DTO to render the actual pixels; tests read off it
/// to assert the CTA contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebriefModal {
    pub bundle_dir: PathBuf,
    pub run_id: String,
    pub buttons: Vec<DebriefModalButton>,
}

impl DebriefModal {
    /// Locate the Export Last Replay CTA in the widget tree.
    #[must_use]
    pub fn export_last_replay_button(&self) -> Option<&DebriefModalButton> {
        self.buttons.iter().find(|b| b.id == EXPORT_LAST_REPLAY_BUTTON_ID)
    }

    /// `true` when the Export Last Replay CTA is wired with a dispatch
    /// handler (the production path; the test asserts this).
    #[must_use]
    pub fn has_export_last_replay_cta(&self) -> bool {
        self.export_last_replay_button()
            .map(|b| b.on_click.is_some())
            .unwrap_or(false)
    }
}

/// Build the debrief modal's widget tree for the given run bundle.
/// The "active run bundle" is the post-mission run the player just
/// finished — the cf-app shell tracks it in its session state and
/// passes the path here.
#[must_use]
pub fn build_debrief_modal(bundle_dir: &Path, run_id: &str) -> DebriefModal {
    let dispatch = ExportCtaDispatch::new(bundle_dir, run_id);
    let export_button = DebriefModalButton {
        id: EXPORT_LAST_REPLAY_BUTTON_ID.into(),
        label: EXPORT_LAST_REPLAY_BUTTON_LABEL.into(),
        on_click: Some(dispatch),
    };
    DebriefModal {
        bundle_dir: bundle_dir.to_path_buf(),
        run_id: run_id.to_string(),
        buttons: vec![export_button],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M10B-DEBRIEF-CTA: the modal's widget tree contains the
    /// Export Last Replay CTA with the documented id + label.
    #[test]
    fn export_last_replay_button_is_in_widget_tree() {
        let modal = build_debrief_modal(Path::new("/tmp/test_bundle"), "test_run_42");
        let button = modal.export_last_replay_button().expect("button present");
        assert_eq!(button.id, EXPORT_LAST_REPLAY_BUTTON_ID);
        assert_eq!(button.label, EXPORT_LAST_REPLAY_BUTTON_LABEL);
        assert!(button.on_click.is_some(), "CTA must have an on-click dispatch");
    }

    /// VAL-M10B-DEBRIEF-CTA: the button's on-click handler invokes
    /// `cf-tools-replay-viewer export <bundle> --preset clip_compact
    /// --out <platform_default_path>`.
    #[test]
    fn export_last_replay_button_dispatches_clip_compact_with_bundle() {
        let bundle = Path::new("/tmp/test_bundle");
        let modal = build_debrief_modal(bundle, "test_run_42");
        let button = modal.export_last_replay_button().expect("button present");
        let dispatch = button.on_click.as_ref().expect("dispatch present");
        assert_eq!(dispatch.binary, VIEWER_BIN_NAME);
        assert_eq!(dispatch.preset, EXPORT_LAST_REPLAY_DEFAULT_PRESET);
        assert_eq!(dispatch.argv[0], "export");
        assert_eq!(dispatch.argv[1], bundle.display().to_string());
        let preset_idx = dispatch.argv.iter().position(|s| s == "--preset").unwrap();
        assert_eq!(dispatch.argv[preset_idx + 1], "clip_compact");
        let out_idx = dispatch.argv.iter().position(|s| s == "--out").unwrap();
        // The platform default path lands under `Corefall` subdir
        // resolved via dirs-next.
        assert!(
            dispatch.argv[out_idx + 1].contains("Corefall")
                || dispatch.argv[out_idx + 1].ends_with("test_run_42.mp4"),
            "out path must include Corefall subdir or fallback name; got {}",
            dispatch.argv[out_idx + 1]
        );
    }

    /// `has_export_last_replay_cta` returns true when the modal is
    /// constructed via [`build_debrief_modal`].
    #[test]
    fn modal_has_export_last_replay_cta() {
        let modal = build_debrief_modal(Path::new("/tmp/b"), "rid");
        assert!(modal.has_export_last_replay_cta());
    }
}
