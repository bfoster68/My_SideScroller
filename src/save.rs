use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;

use crate::audio::GameSettings;
use crate::checkpoint::CheckpointData;
use crate::constants::{HIGHSCORE_FILE, SAVE_FILE};
use crate::highscore::HighScore;

/// Serializable save data — everything that persists between sessions.
#[derive(Serialize, Deserialize, Default)]
pub struct SaveData {
    pub high_score: u32,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub resolution_index: usize,
    pub fullscreen: bool,
    // Checkpoint data for "Continue" feature
    pub checkpoint_score: u32,
    pub checkpoint_x: f32,
    pub checkpoint_y: f32,
    pub checkpoint_section: u32,
}

/// Marker resource: inserted when the player chooses "Continue" from the menu.
/// Systems check for this to restore checkpoint state instead of resetting.
#[derive(Resource)]
pub struct ResumeFromCheckpoint;

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_save_data);
    }
}

/// Load save data at startup, populating GameSettings, HighScore, and CheckpointData.
/// Migrates legacy highscore.dat if save.json doesn't exist yet.
fn load_save_data(
    mut settings: ResMut<GameSettings>,
    mut high_score: ResMut<HighScore>,
    mut checkpoint: ResMut<CheckpointData>,
) {
    if let Ok(contents) = fs::read_to_string(SAVE_FILE) {
        if let Ok(data) = serde_json::from_str::<SaveData>(&contents) {
            high_score.value = data.high_score;
            settings.master_volume = data.master_volume;
            settings.sfx_volume = data.sfx_volume;
            settings.music_volume = data.music_volume;
            settings.resolution_index = data.resolution_index;
            settings.fullscreen = data.fullscreen;

            if data.checkpoint_score > 0 {
                checkpoint.last_checkpoint_score = data.checkpoint_score;
                checkpoint.checkpoint_x = data.checkpoint_x;
                checkpoint.checkpoint_y = data.checkpoint_y;
                checkpoint.section = data.checkpoint_section;
                // Don't set initialized = true here; that happens on entering Playing
            }

            info!("Loaded save data (high score: {}, checkpoint score: {})", data.high_score, data.checkpoint_score);
            return;
        }
    }

    // Migration: check for legacy highscore.dat
    if let Ok(contents) = fs::read_to_string(HIGHSCORE_FILE) {
        if let Ok(value) = contents.trim().parse::<u32>() {
            high_score.value = value;
            info!("Migrated high score from highscore.dat: {}", value);
            // Save in new format
            save_to_disk(&settings, &high_score, &checkpoint);
        }
    }
}

/// Write current game state to save.json.
pub fn save_to_disk(
    settings: &GameSettings,
    high_score: &HighScore,
    checkpoint: &CheckpointData,
) {
    let data = SaveData {
        high_score: high_score.value,
        master_volume: settings.master_volume,
        sfx_volume: settings.sfx_volume,
        music_volume: settings.music_volume,
        resolution_index: settings.resolution_index,
        fullscreen: settings.fullscreen,
        checkpoint_score: checkpoint.last_checkpoint_score,
        checkpoint_x: checkpoint.checkpoint_x,
        checkpoint_y: checkpoint.checkpoint_y,
        checkpoint_section: checkpoint.section,
    };

    match serde_json::to_string_pretty(&data) {
        Ok(json) => {
            if let Err(e) = fs::write(SAVE_FILE, json) {
                warn!("Failed to write save file: {}", e);
            }
        }
        Err(e) => {
            warn!("Failed to serialize save data: {}", e);
        }
    }
}
