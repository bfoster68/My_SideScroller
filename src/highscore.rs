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
    mut active_slot: ResMut<crate::save::ActiveSlot>,
) {
    if score.value > high_score.value {
        high_score.value = score.value;
        commands.insert_resource(NewHighScoreFlag);
    }

    // Game over = run ended. Clear checkpoint data.
    *checkpoint = CheckpointData::default();

    // Detach from the loaded save slot so the next run creates a fresh slot
    // instead of overwriting the one that was loaded (Load -> die -> retry).
    active_slot.0 = None;

    // Save settings + high score (don't touch save slots — game over doesn't delete saves)
    crate::save::save_settings(&settings, &high_score);
    info!("Game data saved (high score: {})", high_score.value);
}

fn clear_new_high_score_flag(mut commands: Commands) {
    commands.remove_resource::<NewHighScoreFlag>();
}
