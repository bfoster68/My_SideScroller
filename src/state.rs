use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode, WindowPosition, WindowResolution};

use crate::checkpoint::CheckpointData;
use crate::constants::*;
use crate::save::ResumeFromCheckpoint;
use crate::transition::{ScreenTransition, start_transition};

/// Top-level game states controlling which systems run.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Settings,
    Playing,
    Paused,
    GameOver,
}

/// Tracks where the Settings screen was opened from so "Back" returns correctly.
#[derive(Resource, Default)]
pub struct SettingsReturnState(pub Option<GameState>);

/// Tracks which menu item is currently highlighted.
#[derive(Resource, Default)]
pub struct MenuSelection {
    pub index: usize,
}

/// Tracks which pause menu item is currently highlighted.
#[derive(Resource, Default)]
pub struct PauseSelection {
    pub index: usize,
}

/// Tracks which settings row is currently highlighted.
#[derive(Resource, Default)]
pub struct SettingsSelection {
    pub index: usize,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<SettingsReturnState>()
            .init_resource::<MenuSelection>()
            .init_resource::<PauseSelection>()
            .init_resource::<SettingsSelection>()
            .add_systems(Startup, apply_saved_window_settings)
            .add_systems(Update, handle_state_input);
    }
}

/// Apply saved resolution/fullscreen settings to the window on startup.
fn apply_saved_window_settings(
    settings: Res<crate::audio::GameSettings>,
    mut window_query: Query<&mut Window>,
) {
    let Ok(mut window) = window_query.single_mut() else { return };
    let (w, h) = RESOLUTIONS[settings.resolution_index.min(RESOLUTIONS.len() - 1)];
    window.resolution = WindowResolution::new(w, h);
    window.position = WindowPosition::Centered(MonitorSelection::Current);
    if settings.fullscreen {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    }
}

fn handle_state_input(
    mut commands: Commands,
    game_input: Res<crate::input::GameInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    transition: Option<Res<ScreenTransition>>,
    mut menu_sel: ResMut<MenuSelection>,
    mut pause_sel: ResMut<PauseSelection>,
    mut settings_sel: ResMut<SettingsSelection>,
    mut settings_return: ResMut<SettingsReturnState>,
    mut settings: ResMut<crate::audio::GameSettings>,
    checkpoint: Res<CheckpointData>,
    mut window_query: Query<&mut Window>,
    high_score: Res<crate::highscore::HighScore>,
) {
    // Don't process input while a transition is active
    if transition.is_some() {
        return;
    }

    match current_state.get() {
        GameState::Menu => {
            let has_checkpoint = checkpoint.last_checkpoint_score > 0;
            let count = if has_checkpoint { 4 } else { 3 }; // Continue/Play/Settings/Quit or Play/Settings/Quit
            if game_input.up_pressed {
                menu_sel.index = if menu_sel.index == 0 { count - 1 } else { menu_sel.index - 1 };
            }
            if game_input.down_pressed {
                menu_sel.index = (menu_sel.index + 1) % count;
            }
            if game_input.confirm_pressed {
                // Map selection index to action based on whether Continue is shown
                let action = if has_checkpoint {
                    menu_sel.index // 0=Continue, 1=New Game, 2=Settings, 3=Quit
                } else {
                    menu_sel.index + 1 // offset: 1=New Game, 2=Settings, 3=Quit
                };
                match action {
                    0 => {
                        // Continue from checkpoint
                        commands.insert_resource(ResumeFromCheckpoint);
                        start_transition(&mut commands, GameState::Playing);
                    }
                    1 => start_transition(&mut commands, GameState::Playing),
                    2 => {
                        settings_return.0 = Some(GameState::Menu);
                        settings_sel.index = 0;
                        next_state.set(GameState::Settings);
                    }
                    3 => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }
        GameState::Playing => {
            if game_input.pause_pressed {
                pause_sel.index = 0;
                next_state.set(GameState::Paused);
            }
        }
        GameState::Paused => {
            let count = 3; // Resume, Settings, Quit to Menu
            if game_input.pause_pressed {
                next_state.set(GameState::Playing);
            }
            if game_input.up_pressed {
                pause_sel.index = if pause_sel.index == 0 { count - 1 } else { pause_sel.index - 1 };
            }
            if game_input.down_pressed {
                pause_sel.index = (pause_sel.index + 1) % count;
            }
            if game_input.confirm_pressed {
                match pause_sel.index {
                    0 => next_state.set(GameState::Playing),
                    1 => {
                        settings_return.0 = Some(GameState::Paused);
                        settings_sel.index = 0;
                        next_state.set(GameState::Settings);
                    }
                    2 => {
                        start_transition(&mut commands, GameState::Menu);
                    }
                    _ => {}
                }
            }
        }
        GameState::Settings => {
            let count = 6; // Master, SFX, Music, Resolution, Fullscreen, Back
            if game_input.up_pressed {
                settings_sel.index = if settings_sel.index == 0 { count - 1 } else { settings_sel.index - 1 };
            }
            if game_input.down_pressed {
                settings_sel.index = (settings_sel.index + 1) % count;
            }

            let step = VOLUME_STEP;
            let adjust_left = game_input.left_pressed;
            let adjust_right = game_input.right_pressed;

            if adjust_left || adjust_right {
                match settings_sel.index {
                    0 => {
                        let delta = if adjust_right { step } else { -step };
                        settings.master_volume = (settings.master_volume + delta).clamp(0.0, 1.0);
                    }
                    1 => {
                        let delta = if adjust_right { step } else { -step };
                        settings.sfx_volume = (settings.sfx_volume + delta).clamp(0.0, 1.0);
                    }
                    2 => {
                        let delta = if adjust_right { step } else { -step };
                        settings.music_volume = (settings.music_volume + delta).clamp(0.0, 1.0);
                    }
                    3 => {
                        // Resolution cycling
                        let max = RESOLUTIONS.len();
                        if adjust_right {
                            settings.resolution_index = (settings.resolution_index + 1) % max;
                        } else if settings.resolution_index == 0 {
                            settings.resolution_index = max - 1;
                        } else {
                            settings.resolution_index -= 1;
                        }
                        // Apply resolution and re-center window
                        if let Ok(mut window) = window_query.single_mut() {
                            let (w, h) = RESOLUTIONS[settings.resolution_index];
                            window.resolution = WindowResolution::new(w, h);
                            window.position = WindowPosition::Centered(MonitorSelection::Current);
                        }
                    }
                    4 => {
                        // Fullscreen toggle
                        settings.fullscreen = !settings.fullscreen;
                        if let Ok(mut window) = window_query.single_mut() {
                            if settings.fullscreen {
                                window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
                            } else {
                                window.mode = WindowMode::Windowed;
                                window.position = WindowPosition::Centered(MonitorSelection::Current);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Back: Escape or confirm on "Back" item
            let back_index = count - 1; // 5
            if keyboard.just_pressed(KeyCode::Escape)
                || (game_input.confirm_pressed && settings_sel.index == back_index)
            {
                // Save settings when leaving
                crate::save::save_to_disk(&settings, &high_score, &checkpoint);

                if let Some(return_state) = settings_return.0 {
                    next_state.set(return_state);
                } else {
                    next_state.set(GameState::Menu);
                }
            }
        }
        GameState::GameOver => {
            if game_input.confirm_pressed {
                start_transition(&mut commands, GameState::Playing);
            }
        }
    }
}
