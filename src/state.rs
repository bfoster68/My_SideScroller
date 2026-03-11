use bevy::prelude::*;

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
    keyboard: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    match current_state.get() {
        GameState::Menu => {
            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                next_state.set(GameState::Playing);
            }
        }
        GameState::Playing => {
            if keyboard.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Paused);
            }
        }
        GameState::Paused => {
            if keyboard.just_pressed(KeyCode::Escape) {
                next_state.set(GameState::Playing);
            }
        }
        GameState::GameOver => {
            if keyboard.just_pressed(KeyCode::Enter)
                || keyboard.just_pressed(KeyCode::Space)
            {
                next_state.set(GameState::Playing);
            }
        }
    }
}
