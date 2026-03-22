use bevy::prelude::*;

use crate::audio::GameSettings;
use crate::checkpoint::CheckpointData;
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
            .add_systems(OnEnter(GameState::GameOver), check_and_save_high_score.in_set(HighScoreSet))
            .add_systems(OnExit(GameState::GameOver), clear_new_high_score_flag);
    }
}

fn check_and_save_high_score(
    mut commands: Commands,
    score: Res<Score>,
    mut high_score: ResMut<HighScore>,
    settings: Res<GameSettings>,
    mut checkpoint: ResMut<CheckpointData>,
) {
    if score.value > high_score.value {
        high_score.value = score.value;
        commands.insert_resource(NewHighScoreFlag);
    }

    // Game over = run ended. Clear checkpoint so "Continue" isn't offered
    // for a finished run — the player should start fresh.
    *checkpoint = CheckpointData::default();

    // Save high score + settings (checkpoint is now cleared)
    crate::save::save_to_disk(&settings, &high_score, &checkpoint);
    info!("Game data saved (high score: {})", high_score.value);
}

fn clear_new_high_score_flag(mut commands: Commands) {
    commands.remove_resource::<NewHighScoreFlag>();
}
