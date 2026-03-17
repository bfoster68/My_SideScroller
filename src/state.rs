use bevy::prelude::*;

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
            .add_systems(Update, handle_state_input);
    }
}

fn handle_state_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    transition: Option<Res<ScreenTransition>>,
    mut menu_sel: ResMut<MenuSelection>,
    mut pause_sel: ResMut<PauseSelection>,
    mut settings_sel: ResMut<SettingsSelection>,
    mut settings_return: ResMut<SettingsReturnState>,
    mut settings: ResMut<crate::audio::GameSettings>,
) {
    // Don't process input while a transition is active
    if transition.is_some() {
        return;
    }

    match current_state.get() {
        GameState::Menu => {
            let count = 3; // Play, Settings, Quit
            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                menu_sel.index = if menu_sel.index == 0 { count - 1 } else { menu_sel.index - 1 };
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                menu_sel.index = (menu_sel.index + 1) % count;
            }
            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
                match menu_sel.index {
                    0 => start_transition(&mut commands, GameState::Playing),
                    1 => {
                        settings_return.0 = Some(GameState::Menu);
                        settings_sel.index = 0;
                        next_state.set(GameState::Settings);
                    }
                    2 => {
                        std::process::exit(0);
                    }
                    _ => {}
                }
            }
        }
        GameState::Playing => {
            if keyboard.just_pressed(KeyCode::Escape) {
                pause_sel.index = 0;
                next_state.set(GameState::Paused);
            }
        }
        GameState::Paused => {
            let count = 3; // Resume, Settings, Quit to Menu
            if keyboard.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Playing);
            }
            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                pause_sel.index = if pause_sel.index == 0 { count - 1 } else { pause_sel.index - 1 };
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                pause_sel.index = (pause_sel.index + 1) % count;
            }
            if keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space) {
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
            let count = 4; // Master, SFX, Music, Back
            if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW) {
                settings_sel.index = if settings_sel.index == 0 { count - 1 } else { settings_sel.index - 1 };
            }
            if keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS) {
                settings_sel.index = (settings_sel.index + 1) % count;
            }

            let step = crate::constants::VOLUME_STEP;
            let adjust_left = keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);
            let adjust_right = keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);

            if adjust_left || adjust_right {
                let delta = if adjust_right { step } else { -step };
                match settings_sel.index {
                    0 => settings.master_volume = (settings.master_volume + delta).clamp(0.0, 1.0),
                    1 => settings.sfx_volume = (settings.sfx_volume + delta).clamp(0.0, 1.0),
                    2 => settings.music_volume = (settings.music_volume + delta).clamp(0.0, 1.0),
                    _ => {}
                }
            }

            if keyboard.just_pressed(KeyCode::Escape)
                || (keyboard.just_pressed(KeyCode::Enter) && settings_sel.index == 3)
                || (keyboard.just_pressed(KeyCode::Space) && settings_sel.index == 3)
            {
                if let Some(return_state) = settings_return.0 {
                    next_state.set(return_state);
                } else {
                    next_state.set(GameState::Menu);
                }
            }
        }
        GameState::GameOver => {
            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                start_transition(&mut commands, GameState::Playing);
            }
        }
    }
}
