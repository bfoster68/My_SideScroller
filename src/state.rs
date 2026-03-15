use bevy::prelude::*;

use crate::transition::{ScreenTransition, start_transition};

/// Top-level game states controlling which systems run.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
    Paused,
    GameOver,
}

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Update, handle_state_input);
    }
}

fn handle_state_input(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
    transition: Option<Res<ScreenTransition>>,
) {
    // Don't process input while a transition is active
    if transition.is_some() {
        return;
    }

    match current_state.get() {
        GameState::Menu => {
            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                start_transition(&mut commands, GameState::Playing);
            }
        }
        GameState::Playing => {
            if keyboard.just_pressed(KeyCode::Escape) {
                // Pause is instant — no fade
                next_state.set(GameState::Paused);
            }
        }
        GameState::Paused => {
            if keyboard.just_pressed(KeyCode::Escape) {
                // Unpause is instant — no fade
                next_state.set(GameState::Playing);
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
