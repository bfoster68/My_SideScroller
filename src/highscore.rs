use bevy::prelude::*;
use std::fs;

use crate::constants::HIGHSCORE_FILE;
use crate::player::Score;
use crate::state::GameState;

/// Persisted high score — survives between sessions.
#[derive(Resource, Default)]
pub struct HighScore {
    pub value: u32,
}

/// Marker resource: inserted when the player beats the high score.
/// Present during the GameOver state so the overlay can show "NEW HIGH SCORE!".
#[derive(Resource)]
pub struct NewHighScoreFlag;

/// System set so other systems can order themselves after the high score check.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct HighScoreSet;

pub struct HighScorePlugin;

impl Plugin for HighScorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HighScore>()
            .add_systems(Startup, load_high_score)
            .add_systems(OnEnter(GameState::GameOver), check_and_save_high_score.in_set(HighScoreSet))
            .add_systems(OnExit(GameState::GameOver), clear_new_high_score_flag);
    }
}

fn load_high_score(mut high_score: ResMut<HighScore>) {
    if let Ok(contents) = fs::read_to_string(HIGHSCORE_FILE) {
        if let Ok(value) = contents.trim().parse::<u32>() {
            high_score.value = value;
            info!("Loaded high score: {}", value);
        }
    }
}

fn check_and_save_high_score(
    mut commands: Commands,
    score: Res<Score>,
    mut high_score: ResMut<HighScore>,
) {
    if score.value > high_score.value {
        high_score.value = score.value;
        commands.insert_resource(NewHighScoreFlag);

        if let Err(e) = fs::write(HIGHSCORE_FILE, high_score.value.to_string()) {
            warn!("Failed to save high score: {}", e);
        } else {
            info!("New high score saved: {}", high_score.value);
        }
    }
}

fn clear_new_high_score_flag(mut commands: Commands) {
    commands.remove_resource::<NewHighScoreFlag>();
}
