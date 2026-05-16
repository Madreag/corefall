//! `act.shell.*` scriptable API surface — invoked by cfctl + cf-control's
//! action dispatcher. Mirrors the spec's "act.shell.* API: open_title,
//! open_main_menu, open_pause, save_to_slot, load_from_slot, quit_to_menu,
//! quit_to_desktop" requirement.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::state::{FreStep, SettingsTab, ShellScreen, ShellState, ShellTransition, TransitionSource};

/// Every act.shell.* command surface for cfctl scripting + cf-control wiring.
///
/// Per spec: shell UI is testable via cfctl, not hand-coded one-off overlays.
#[derive(Debug, Clone, Message, Serialize, Deserialize)]
pub enum ShellApiCommand {
    OpenTitle,
    OpenMainMenu,
    OpenPause,
    OpenSettings { tab: Option<SettingsTab> },
    OpenSaveLoad { mode: SaveLoadMode },
    OpenCredits,
    OpenLoadingScreen { scenario_id: String },
    OpenFreWizard { step: Option<FreStep> },
    OpenWorkshop,
    SaveToSlot { slot_index: usize, name: Option<String> },
    LoadFromSlot { slot_index: usize },
    DeleteSlot { slot_index: usize },
    QuitToMenu,
    QuitToDesktop,
    SkipSplash,
    ResumeMission,
    AdvanceFreStep,
    BackFreStep,
    /// **M12**: open the CCCP-style intro slideshow. `slot` distinguishes
    /// the first-launch intro from the Main Menu → Story → "Replay Intro"
    /// re-watch. Per spec § CCCP-style intro slideshow.
    OpenIntroSlideshow {
        slot: IntroSlideshowSlot,
    },
    /// **M12**: skip the currently-playing slideshow.
    SkipIntroSlideshow,
}

/// **M12**: slot identifying which slideshow surface is being opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntroSlideshowSlot {
    /// First-launch intro slideshow (8 slides — "you will now join the frontier").
    FirstLaunch,
    /// Main Menu → Story → "Replay Intro" re-watch.
    Replay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveLoadMode {
    Save,
    Load,
}

/// Bevy system that consumes `ShellApiCommand` events and emits
/// `ShellTransition` events accordingly. Side effects (save-write,
/// load-read, quit-to-desktop) are routed via cf-save / cf-app at
/// the binary integration layer.
pub fn handle_shell_api_commands(
    mut cmds: MessageReader<ShellApiCommand>,
    mut transitions: MessageWriter<ShellTransition>,
    mut state: ResMut<ShellState>,
) {
    for cmd in cmds.read() {
        match cmd {
            ShellApiCommand::OpenTitle => {
                transitions.write(ShellTransition {
                    to: ShellScreen::Title,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenMainMenu => {
                transitions.write(ShellTransition {
                    to: ShellScreen::MainMenu,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenPause => {
                if state.mission_active {
                    transitions.write(ShellTransition {
                        to: ShellScreen::Pause,
                        source: TransitionSource::ScriptedApi,
                    });
                }
            }
            ShellApiCommand::OpenSettings { tab } => {
                transitions.write(ShellTransition {
                    to: ShellScreen::Settings(tab.unwrap_or_default()),
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenSaveLoad { .. } => {
                transitions.write(ShellTransition {
                    to: ShellScreen::SaveLoad,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenCredits => {
                transitions.write(ShellTransition {
                    to: ShellScreen::Credits,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenLoadingScreen { scenario_id: _ } => {
                transitions.write(ShellTransition {
                    to: ShellScreen::Loading,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenFreWizard { step } => {
                transitions.write(ShellTransition {
                    to: ShellScreen::FreWizard(step.unwrap_or_default()),
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::OpenWorkshop => {
                transitions.write(ShellTransition {
                    to: ShellScreen::Workshop,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::SaveToSlot { slot_index, name } => {
                tracing::info!(target = "cf-shell", slot_index, name = ?name, "save_to_slot scripted");
                state.has_save = true;
            }
            ShellApiCommand::LoadFromSlot { slot_index } => {
                tracing::info!(target = "cf-shell", slot_index, "load_from_slot scripted");
                transitions.write(ShellTransition {
                    to: ShellScreen::Loading,
                    source: TransitionSource::ScriptedApi,
                });
                state.mission_active = true;
            }
            ShellApiCommand::DeleteSlot { slot_index } => {
                tracing::info!(target = "cf-shell", slot_index, "delete_slot scripted");
            }
            ShellApiCommand::QuitToMenu => {
                state.mission_active = false;
                transitions.write(ShellTransition {
                    to: ShellScreen::MainMenu,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::QuitToDesktop => {
                tracing::info!(target = "cf-shell", "quit_to_desktop scripted");
            }
            ShellApiCommand::SkipSplash => {
                if state.splash_skippable {
                    transitions.write(ShellTransition {
                        to: ShellScreen::Title,
                        source: TransitionSource::UserInput,
                    });
                }
            }
            ShellApiCommand::ResumeMission => {
                if state.mission_active {
                    transitions.write(ShellTransition {
                        to: ShellScreen::InMission,
                        source: TransitionSource::ScriptedApi,
                    });
                }
            }
            ShellApiCommand::AdvanceFreStep => {
                if let ShellScreen::FreWizard(step) = state.current {
                    if let Some(next) = step.next() {
                        transitions.write(ShellTransition {
                            to: ShellScreen::FreWizard(next),
                            source: TransitionSource::UserInput,
                        });
                    } else {
                        state.fre_completed = true;
                        transitions.write(ShellTransition {
                            to: ShellScreen::MainMenu,
                            source: TransitionSource::AutoAfterMissionResolved,
                        });
                    }
                }
            }
            ShellApiCommand::BackFreStep => {
                if let ShellScreen::FreWizard(step) = state.current {
                    if let Some(prev) = step.prev() {
                        transitions.write(ShellTransition {
                            to: ShellScreen::FreWizard(prev),
                            source: TransitionSource::UserInput,
                        });
                    }
                }
            }
            ShellApiCommand::OpenIntroSlideshow { slot } => {
                state.intro_slideshow_slot = Some(*slot);
                transitions.write(ShellTransition {
                    to: ShellScreen::IntroSlideshow,
                    source: TransitionSource::ScriptedApi,
                });
            }
            ShellApiCommand::SkipIntroSlideshow => {
                if state.current == ShellScreen::IntroSlideshow {
                    let next = match state.intro_slideshow_slot {
                        Some(IntroSlideshowSlot::FirstLaunch) => {
                            if state.fre_completed {
                                ShellScreen::MainMenu
                            } else {
                                ShellScreen::FreWizard(FreStep::default())
                            }
                        }
                        _ => ShellScreen::MainMenu,
                    };
                    state.intro_slideshow_slot = None;
                    transitions.write(ShellTransition {
                        to: next,
                        source: TransitionSource::UserInput,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::SaveSlotMetadataIndex;

    fn build_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<ShellState>()
            .init_resource::<SaveSlotMetadataIndex>()
            .add_message::<ShellTransition>()
            .add_message::<ShellApiCommand>()
            .add_systems(
                Update,
                (
                    handle_shell_api_commands,
                    crate::state::apply_shell_transitions,
                ).chain(),
            );
        app
    }

    #[test]
    fn open_title_writes_transition() {
        let mut app = build_test_app();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenTitle);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::Title);
    }

    #[test]
    fn open_pause_requires_mission_active() {
        let mut app = build_test_app();
        // Mission not active — pause should not fire
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenPause);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_ne!(state.current, ShellScreen::Pause);

        // Activate mission and retry
        app.world_mut().resource_mut::<ShellState>().mission_active = true;
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenPause);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::Pause);
    }

    #[test]
    fn save_to_slot_marks_has_save() {
        let mut app = build_test_app();
        assert!(!app.world().resource::<ShellState>().has_save);
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::SaveToSlot { slot_index: 0, name: Some("test".to_string()) });
        app.update();
        assert!(app.world().resource::<ShellState>().has_save);
    }

    #[test]
    fn quit_to_menu_clears_mission_active() {
        let mut app = build_test_app();
        app.world_mut().resource_mut::<ShellState>().mission_active = true;
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::QuitToMenu);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert!(!state.mission_active);
        assert_eq!(state.current, ShellScreen::MainMenu);
    }

    #[test]
    fn skip_splash_advances_to_title_when_skippable() {
        let mut app = build_test_app();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::SkipSplash);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::Title);
    }

    #[test]
    fn fre_step_advance_completes_after_step_6() {
        let mut app = build_test_app();
        app.world_mut().resource_mut::<ShellState>().current = ShellScreen::FreWizard(FreStep::StarterWorldRecommendation);
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::AdvanceFreStep);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert!(state.fre_completed);
        assert_eq!(state.current, ShellScreen::MainMenu);
    }

    #[test]
    fn open_intro_slideshow_transitions_to_slideshow_screen() {
        let mut app = build_test_app();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenIntroSlideshow {
                slot: IntroSlideshowSlot::Replay,
            });
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::IntroSlideshow);
        assert_eq!(state.intro_slideshow_slot, Some(IntroSlideshowSlot::Replay));
    }

    #[test]
    fn skip_intro_slideshow_returns_to_main_menu_for_replay_slot() {
        let mut app = build_test_app();
        // Open then skip.
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenIntroSlideshow {
                slot: IntroSlideshowSlot::Replay,
            });
        app.update();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::SkipIntroSlideshow);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::MainMenu);
        assert!(state.intro_slideshow_slot.is_none());
    }

    #[test]
    fn skip_intro_slideshow_routes_first_launch_to_fre_wizard_when_incomplete() {
        let mut app = build_test_app();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenIntroSlideshow {
                slot: IntroSlideshowSlot::FirstLaunch,
            });
        app.update();
        // FRE not completed yet — skip should route to FreWizard step 1.
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::SkipIntroSlideshow);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert!(matches!(state.current, ShellScreen::FreWizard(_)));
    }

    #[test]
    fn skip_intro_slideshow_routes_first_launch_to_main_menu_after_fre() {
        let mut app = build_test_app();
        app.world_mut().resource_mut::<ShellState>().fre_completed = true;
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::OpenIntroSlideshow {
                slot: IntroSlideshowSlot::FirstLaunch,
            });
        app.update();
        app.world_mut()
            .resource_mut::<Messages<ShellApiCommand>>()
            .write(ShellApiCommand::SkipIntroSlideshow);
        app.update();
        let state = app.world().resource::<ShellState>();
        assert_eq!(state.current, ShellScreen::MainMenu);
    }
}
