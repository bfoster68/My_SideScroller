use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::audio::GameSettings;
use crate::checkpoint::CheckpointData;
use crate::highscore::HighScore;
use crate::state::GameState;

// Storage keys
const SETTINGS_KEY: &str = "my_sidescroller_settings";
const SLOT_INDEX_KEY: &str = "my_sidescroller_slots";
const SLOT_PREFIX: &str = "my_sidescroller_slot_";

// Native file paths
const SETTINGS_FILE: &str = "settings.json";
const SAVES_DIR: &str = "saves";

/// Maximum number of save slots kept on disk. Each New Game autosaves into a
/// fresh slot, so without a cap the list would grow without bound.
pub const MAX_SAVE_SLOTS: usize = 10;

/// Global settings — persisted independently from save slots.
#[derive(Serialize, Deserialize)]
pub struct SettingsData {
    pub high_score: u32,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub resolution_index: usize,
    pub fullscreen: bool,
}

/// A single save slot's game progress.
#[derive(Serialize, Deserialize, Clone)]
pub struct SlotData {
    pub score: u32,
    pub checkpoint_x: f32,
    pub checkpoint_y: f32,
    pub section: u32,
    pub timestamp: String,
}

/// Index entry for quick listing without loading full slot data.
#[derive(Serialize, Deserialize, Clone)]
pub struct SlotEntry {
    pub id: u32,
    pub score: u32,
    pub section: u32,
    pub timestamp: String,
}

/// The slot index file — lists all save slots.
#[derive(Serialize, Deserialize, Default)]
pub struct SlotIndex {
    pub slots: Vec<SlotEntry>,
    pub next_id: u32,
}

/// Tracks which save slot is currently active (being played).
#[derive(Resource, Default)]
pub struct ActiveSlot(pub Option<u32>);

/// Marker resource: inserted when the player chooses to load a save slot.
/// Systems check for this to restore checkpoint state instead of resetting.
#[derive(Resource)]
pub struct ResumeFromCheckpoint;

/// In-memory copy of the slot index so menus don't hit disk every frame.
/// Refreshed on entering Menu/SaveMenu and after any slot mutation.
#[derive(Resource, Default)]
pub struct SaveSlotCache {
    pub slots: Vec<SlotEntry>,
}

impl SaveSlotCache {
    pub fn refresh(&mut self) {
        self.slots = list_slots();
    }
}

fn refresh_save_slot_cache(mut cache: ResMut<SaveSlotCache>) {
    cache.refresh();
}

/// Legacy save format for migration.
#[derive(Serialize, Deserialize, Default)]
struct LegacySaveData {
    pub high_score: u32,
    pub master_volume: f32,
    pub sfx_volume: f32,
    pub music_volume: f32,
    pub resolution_index: usize,
    pub fullscreen: bool,
    pub checkpoint_score: u32,
    pub checkpoint_x: f32,
    pub checkpoint_y: f32,
    pub checkpoint_section: u32,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveSlot>()
            .init_resource::<SaveSlotCache>()
            .add_systems(Startup, load_settings_data)
            .add_systems(OnEnter(GameState::Menu), refresh_save_slot_cache)
            .add_systems(OnEnter(GameState::SaveMenu), refresh_save_slot_cache);
    }
}

// ---------------------------------------------------------------------------
// Platform-specific storage helpers
// ---------------------------------------------------------------------------

fn read_file(key: &str, native_path: &str) -> Option<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        std::fs::read_to_string(native_path).ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = native_path;
        let window = web_sys::window()?;
        let storage = window.local_storage().ok()??;
        storage.get_item(key).ok()?
    }
}

fn write_file(key: &str, native_path: &str, json: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(native_path).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        if let Err(e) = std::fs::write(native_path, json) {
            warn!("Failed to write {}: {}", native_path, e);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = native_path;
        let Some(window) = web_sys::window() else { return };
        let Ok(Some(storage)) = window.local_storage() else { return };
        if let Err(e) = storage.set_item(key, json) {
            warn!("Failed to write localStorage key {}: {:?}", key, e);
        }
    }
}

fn delete_file(key: &str, native_path: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        let _ = std::fs::remove_file(native_path);
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = native_path;
        let Some(window) = web_sys::window() else { return };
        let Ok(Some(storage)) = window.local_storage() else { return };
        let _ = storage.remove_item(key);
    }
}

// ---------------------------------------------------------------------------
// Settings (global, not per-slot)
// ---------------------------------------------------------------------------

fn load_settings_data(
    mut settings: ResMut<GameSettings>,
    mut high_score: ResMut<HighScore>,
    mut checkpoint: ResMut<CheckpointData>,
) {
    // Try new settings format first
    if let Some(contents) = read_file(SETTINGS_KEY, SETTINGS_FILE) {
        match serde_json::from_str::<SettingsData>(&contents) {
            Ok(data) => {
                high_score.value = data.high_score;
                settings.master_volume = data.master_volume;
                settings.sfx_volume = data.sfx_volume;
                settings.music_volume = data.music_volume;
                settings.resolution_index = data.resolution_index;
                settings.fullscreen = data.fullscreen;
                info!("Loaded settings (high score: {})", data.high_score);
                return;
            }
            Err(e) => {
                warn!("Settings file corrupted, checking for legacy format: {}", e);
            }
        }
    }

    // Migration: try legacy save.json format
    if let Some(contents) = read_file("my_sidescroller_save", "save.json") {
        if let Ok(legacy) = serde_json::from_str::<LegacySaveData>(&contents) {
            high_score.value = legacy.high_score;
            settings.master_volume = legacy.master_volume;
            settings.sfx_volume = legacy.sfx_volume;
            settings.music_volume = legacy.music_volume;
            settings.resolution_index = legacy.resolution_index;
            settings.fullscreen = legacy.fullscreen;
            info!("Migrated from legacy save.json (high score: {})", legacy.high_score);

            // Migrate checkpoint data to a save slot if present
            if legacy.checkpoint_score > 0 {
                let now = current_timestamp();
                let slot_data = SlotData {
                    score: legacy.checkpoint_score,
                    checkpoint_x: legacy.checkpoint_x,
                    checkpoint_y: legacy.checkpoint_y,
                    section: legacy.checkpoint_section,
                    timestamp: now.clone(),
                };
                save_slot(0, &slot_data);

                let index = SlotIndex {
                    slots: vec![SlotEntry {
                        id: 0,
                        score: legacy.checkpoint_score,
                        section: legacy.checkpoint_section,
                        timestamp: now,
                    }],
                    next_id: 1,
                };
                save_slot_index(&index);

                // Also load into checkpoint for backward compat
                checkpoint.last_checkpoint_score = legacy.checkpoint_score;
                checkpoint.checkpoint_x = legacy.checkpoint_x;
                checkpoint.checkpoint_y = legacy.checkpoint_y;
                checkpoint.section = legacy.checkpoint_section;
            }

            // Save in new format
            save_settings(&settings, &high_score);
            return;
        }
    }

    // Migration: check for legacy highscore.dat (native only)
    #[cfg(not(target_arch = "wasm32"))]
    {
        use crate::constants::HIGHSCORE_FILE;
        if let Ok(contents) = std::fs::read_to_string(HIGHSCORE_FILE) {
            if let Ok(value) = contents.trim().parse::<u32>() {
                high_score.value = value;
                info!("Migrated high score from highscore.dat: {}", value);
                save_settings(&settings, &high_score);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Save global settings (volumes, resolution, high score).
pub fn save_settings(settings: &GameSettings, high_score: &HighScore) {
    let data = SettingsData {
        high_score: high_score.value,
        master_volume: settings.master_volume,
        sfx_volume: settings.sfx_volume,
        music_volume: settings.music_volume,
        resolution_index: settings.resolution_index,
        fullscreen: settings.fullscreen,
    };
    match serde_json::to_string_pretty(&data) {
        Ok(json) => write_file(SETTINGS_KEY, SETTINGS_FILE, &json),
        Err(e) => warn!("Failed to serialize settings: {}", e),
    }
}

/// List all save slots (quick metadata, no full load).
pub fn list_slots() -> Vec<SlotEntry> {
    load_slot_index().slots
}

/// Load a specific save slot's data.
pub fn load_slot(id: u32) -> Option<SlotData> {
    let key = format!("{}{}", SLOT_PREFIX, id);
    let path = format!("{}/slot_{}.json", SAVES_DIR, id);
    let contents = read_file(&key, &path)?;
    match serde_json::from_str::<SlotData>(&contents) {
        Ok(data) => Some(data),
        Err(e) => {
            warn!("Failed to parse slot {}: {}", id, e);
            None
        }
    }
}

/// Save game progress to a specific slot.
pub fn save_slot(id: u32, data: &SlotData) {
    let key = format!("{}{}", SLOT_PREFIX, id);
    let path = format!("{}/slot_{}.json", SAVES_DIR, id);
    match serde_json::to_string_pretty(data) {
        Ok(json) => write_file(&key, &path, &json),
        Err(e) => warn!("Failed to serialize slot {}: {}", id, e),
    }
}

/// Delete a save slot.
pub fn delete_slot(id: u32) {
    let key = format!("{}{}", SLOT_PREFIX, id);
    let path = format!("{}/slot_{}.json", SAVES_DIR, id);
    delete_file(&key, &path);

    // Remove from index
    let mut index = load_slot_index();
    index.slots.retain(|s| s.id != id);
    save_slot_index(&index);
}

/// Create a new save slot, returning its ID.
pub fn create_slot(data: &SlotData) -> u32 {
    let mut index = load_slot_index();

    // Enforce the slot cap: evict the oldest slots (lowest id, since ids are
    // monotonically increasing) until there is room for the new one.
    while index.slots.len() >= MAX_SAVE_SLOTS {
        let Some(oldest_id) = index.slots.iter().map(|s| s.id).min() else { break };
        let key = format!("{}{}", SLOT_PREFIX, oldest_id);
        let path = format!("{}/slot_{}.json", SAVES_DIR, oldest_id);
        delete_file(&key, &path);
        index.slots.retain(|s| s.id != oldest_id);
    }

    let id = index.next_id;
    index.next_id += 1;
    index.slots.push(SlotEntry {
        id,
        score: data.score,
        section: data.section,
        timestamp: data.timestamp.clone(),
    });
    save_slot_index(&index);
    save_slot(id, data);
    id
}

/// Update an existing slot's data and index entry.
pub fn update_slot(id: u32, data: &SlotData) {
    save_slot(id, data);

    // Update index entry
    let mut index = load_slot_index();
    if let Some(entry) = index.slots.iter_mut().find(|s| s.id == id) {
        entry.score = data.score;
        entry.section = data.section;
        entry.timestamp = data.timestamp.clone();
    }
    save_slot_index(&index);
}

/// Build a SlotData from current game state.
pub fn slot_data_from_checkpoint(checkpoint: &CheckpointData) -> SlotData {
    SlotData {
        score: checkpoint.last_checkpoint_score,
        checkpoint_x: checkpoint.checkpoint_x,
        checkpoint_y: checkpoint.checkpoint_y,
        section: checkpoint.section,
        timestamp: current_timestamp(),
    }
}

/// Backwards-compatible save function — saves settings + auto-saves to active slot.
pub fn save_to_disk(
    settings: &GameSettings,
    high_score: &HighScore,
    _checkpoint: &CheckpointData,
) {
    save_settings(settings, high_score);
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn load_slot_index() -> SlotIndex {
    let contents = read_file(SLOT_INDEX_KEY, &format!("{}/index.json", SAVES_DIR));
    match contents {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => SlotIndex::default(),
    }
}

fn save_slot_index(index: &SlotIndex) {
    match serde_json::to_string_pretty(index) {
        Ok(json) => write_file(SLOT_INDEX_KEY, &format!("{}/index.json", SAVES_DIR), &json),
        Err(e) => warn!("Failed to serialize slot index: {}", e),
    }
}

fn current_timestamp() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Local time (the previous hand-rolled math produced UTC, so saves
        // showed the wrong hour). Same "YYYY-MM-DD HH:MM" layout as before.
        chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()
    }
    #[cfg(target_arch = "wasm32")]
    {
        // Use js_sys Date for accurate timestamps in WASM
        let date = js_sys::Date::new_0();
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}",
            date.get_full_year(),
            date.get_month() + 1,
            date.get_date(),
            date.get_hours(),
            date.get_minutes()
        )
    }
}
