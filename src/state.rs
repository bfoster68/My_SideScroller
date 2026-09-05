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
    SaveMenu,
}

/// What the save menu is being used for.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveMenuMode {
    Load,
    Save,
}

/// Tracks which slot is selected in the save menu.
#[derive(Resource, Default)]
pub struct SaveMenuSelection {
    pub index: usize,
    pub confirm_delete: bool,
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

/// Tracks the previous game state so OnEnter systems can decide whether to reset.
#[derive(Resource, Default)]
pub struct PreviousGameState(pub Option<GameState>);

/// Marker resource that triggers deferred window settings on the first Update frame.
#[derive(Resource)]
struct ApplyWindowSettings;

/// Deferred window centering — applied next frame after a resolution change.
#[derive(Resource)]
struct DeferredRecenter;

pub struct StatePlugin;

impl Plugin for StatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_resource::<SettingsReturnState>()
            .init_resource::<MenuSelection>()
            .init_resource::<PauseSelection>()
            .init_resource::<SettingsSelection>()
            .init_resource::<PreviousGameState>()
            .init_resource::<SaveMenuSelection>()
            .insert_resource(ApplyWindowSettings)
            .add_systems(Update, (apply_saved_window_settings, apply_deferred_recenter, track_previous_state, handle_state_input).chain());
    }
}

/// Apply saved resolution/fullscreen settings on the first Update frame,
/// after the window has been fully created by the OS.
fn apply_saved_window_settings(
    mut commands: Commands,
    marker: Option<Res<ApplyWindowSettings>>,
    settings: Res<crate::audio::GameSettings>,
    mut window_query: Query<&mut Window>,
) {
    if marker.is_none() {
        return;
    }
    commands.remove_resource::<ApplyWindowSettings>();

    let Ok(mut window) = window_query.single_mut() else { return };
    let (w, h) = RESOLUTIONS[settings.resolution_index.min(RESOLUTIONS.len() - 1)];
    window.resolution = WindowResolution::new(w, h);
    window.position = WindowPosition::Centered(MonitorSelection::Current);
    if settings.fullscreen {
        window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Current);
    }
}

/// Applies window centering one frame after a resolution change,
/// giving the OS time to process the new window size.
fn apply_deferred_recenter(
    mut commands: Commands,
    marker: Option<Res<DeferredRecenter>>,
    mut window_query: Query<&mut Window>,
) {
    if marker.is_none() {
        return;
    }
    commands.remove_resource::<DeferredRecenter>();
    let Ok(mut window) = window_query.single_mut() else { return };
    window.position = WindowPosition::Centered(MonitorSelection::Current);
}

/// Record the current state each frame so OnEnter systems know where we came from.
fn track_previous_state(
    current: Res<State<GameState>>,
    next: Res<NextState<GameState>>,
    mut prev: ResMut<PreviousGameState>,
) {
    // Only update when a transition is NOT pending — once NextState is set,
    // we want to freeze the "previous" value until the transition completes.
    if matches!(*next, NextState::Unchanged) {
        prev.0 = Some(*current.get());
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
    mut save_menu_sel: ResMut<SaveMenuSelection>,
    mut settings_return: ResMut<SettingsReturnState>,
    mut settings: ResMut<crate::audio::GameSettings>,
    checkpoint: Res<CheckpointData>,
    mut window_query: Query<&mut Window>,
    high_score: Res<crate::highscore::HighScore>,
    // Grouped into a tuple to stay within Bevy's 16-param system limit.
    (save_menu_mode, mut save_cache): (Option<Res<SaveMenuMode>>, ResMut<crate::save::SaveSlotCache>),
) {
    // Don't process input while a transition is active
    if transition.is_some() {
        return;
    }

    match current_state.get() {
        GameState::Menu => {
            #[cfg(target_arch = "wasm32")]
            let has_saves = false;
            // Read the cached index rather than hitting disk every frame.
            #[cfg(not(target_arch = "wasm32"))]
            let has_saves = !save_cache.slots.is_empty();
            let count = if has_saves { 4 } else { 3 }; // Load/New/Settings/Quit or New/Settings/Quit
            if game_input.up_pressed {
                menu_sel.index = if menu_sel.index == 0 { count - 1 } else { menu_sel.index - 1 };
            }
            if game_input.down_pressed {
                menu_sel.index = (menu_sel.index + 1) % count;
            }
            if game_input.confirm_pressed {
                let action = if has_saves {
                    menu_sel.index // 0=Load, 1=New Game, 2=Settings, 3=Quit
                } else {
                    menu_sel.index + 1 // offset: 1=New Game, 2=Settings, 3=Quit
                };
                match action {
                    0 => {
                        // Open save menu in Load mode
                        commands.insert_resource(SaveMenuMode::Load);
                        save_menu_sel.index = 0;
                        save_menu_sel.confirm_delete = false;
                        next_state.set(GameState::SaveMenu);
                    }
                    1 => {
                        // New Game — create a fresh active slot
                        commands.insert_resource(crate::save::ActiveSlot(None));
                        start_transition(&mut commands, GameState::Playing);
                    }
                    2 => {
                        settings_return.0 = Some(GameState::Menu);
                        settings_sel.index = 0;
                        next_state.set(GameState::Settings);
                    }
                    3 => {
                        #[cfg(not(target_arch = "wasm32"))]
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
            // WASM: Resume, Settings, Quit to Menu (3 items)
            // Native: Resume, Save Game, Settings, Quit to Menu, Quit Game (5 items)
            #[cfg(target_arch = "wasm32")]
            let count = 3;
            #[cfg(not(target_arch = "wasm32"))]
            let count = 5;

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
                // Map index to action — different on WASM (no Save/Quit Game)
                #[cfg(target_arch = "wasm32")]
                let action = match pause_sel.index {
                    0 => 0, // Resume
                    1 => 2, // Settings
                    2 => 3, // Quit to Menu
                    _ => 99,
                };
                #[cfg(not(target_arch = "wasm32"))]
                let action = pause_sel.index;

                match action {
                    0 => next_state.set(GameState::Playing),
                    1 => {
                        // Save Game (native only)
                        commands.insert_resource(SaveMenuMode::Save);
                        save_menu_sel.index = 0;
                        save_menu_sel.confirm_delete = false;
                        next_state.set(GameState::SaveMenu);
                    }
                    2 => {
                        settings_return.0 = Some(GameState::Paused);
                        settings_sel.index = 0;
                        next_state.set(GameState::Settings);
                    }
                    3 => {
                        start_transition(&mut commands, GameState::Menu);
                    }
                    4 => {
                        #[cfg(not(target_arch = "wasm32"))]
                        std::process::exit(0);
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
                        // Only apply resolution in windowed mode — fullscreen uses native resolution
                        if !settings.fullscreen {
                            if let Ok(mut window) = window_query.single_mut() {
                                let (w, h) = RESOLUTIONS[settings.resolution_index];
                                window.resolution = WindowResolution::new(w, h);
                                commands.insert_resource(DeferredRecenter);
                            }
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
                                // Restore saved resolution when exiting fullscreen
                                let (w, h) = RESOLUTIONS[settings.resolution_index];
                                window.resolution = WindowResolution::new(w, h);
                                commands.insert_resource(DeferredRecenter);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Back: Escape/gamepad Start (pause_pressed) or confirm on "Back" item
            let back_index = count - 1; // 5
            if game_input.pause_pressed
                || (game_input.confirm_pressed && settings_sel.index == back_index)
            {
                // Save settings when leaving
                crate::save::save_settings(&settings, &high_score);

                if let Some(return_state) = settings_return.0 {
                    next_state.set(return_state);
                } else {
                    next_state.set(GameState::Menu);
                }
            }
        }
        GameState::SaveMenu => {
            // Cached index (no per-frame disk I/O). Cloned so the cache can be
            // refreshed below after a mutation; it holds at most MAX_SAVE_SLOTS entries.
            let slots = save_cache.slots.clone();
            let mode = save_menu_mode.as_deref().copied().unwrap_or(SaveMenuMode::Load);
            // In Save mode, add one extra entry for "New Save"
            let count = if mode == SaveMenuMode::Save { slots.len() + 1 } else { slots.len() }.max(1);

            // Back: Escape or gamepad Start
            if game_input.pause_pressed {
                if save_menu_sel.confirm_delete {
                    save_menu_sel.confirm_delete = false;
                } else {
                    // Return to where we came from
                    commands.remove_resource::<SaveMenuMode>();
                    match mode {
                        SaveMenuMode::Load => next_state.set(GameState::Menu),
                        SaveMenuMode::Save => next_state.set(GameState::Paused),
                    }
                }
            }

            if game_input.up_pressed && !save_menu_sel.confirm_delete {
                save_menu_sel.index = if save_menu_sel.index == 0 { count - 1 } else { save_menu_sel.index - 1 };
            }
            if game_input.down_pressed && !save_menu_sel.confirm_delete {
                save_menu_sel.index = (save_menu_sel.index + 1) % count;
            }

            // Delete key in Load mode
            if mode == SaveMenuMode::Load
                && keyboard.just_pressed(KeyCode::Delete)
                && save_menu_sel.index < slots.len()
            {
                save_menu_sel.confirm_delete = true;
            }
            // Also support Backspace for delete on Mac
            if mode == SaveMenuMode::Load
                && keyboard.just_pressed(KeyCode::Backspace)
                && save_menu_sel.index < slots.len()
            {
                save_menu_sel.confirm_delete = true;
            }

            if game_input.confirm_pressed {
                if save_menu_sel.confirm_delete {
                    // Confirm delete
                    if let Some(slot) = slots.get(save_menu_sel.index) {
                        crate::save::delete_slot(slot.id);
                        save_menu_sel.confirm_delete = false;
                        // Refresh the cache so the menu reflects the deletion immediately
                        save_cache.refresh();
                        // Clamp index
                        let new_slots = &save_cache.slots;
                        if save_menu_sel.index >= new_slots.len() && !new_slots.is_empty() {
                            save_menu_sel.index = new_slots.len() - 1;
                        } else if new_slots.is_empty() {
                            // No more slots — return to menu
                            commands.remove_resource::<SaveMenuMode>();
                            next_state.set(GameState::Menu);
                        }
                    }
                } else {
                    match mode {
                        SaveMenuMode::Load => {
                            if let Some(slot_entry) = slots.get(save_menu_sel.index) {
                                if let Some(data) = crate::save::load_slot(slot_entry.id) {
                                    // Load slot into checkpoint data
                                    commands.insert_resource(CheckpointData {
                                        last_checkpoint_score: data.score,
                                        checkpoint_x: data.checkpoint_x,
                                        checkpoint_y: data.checkpoint_y,
                                        section: data.section,
                                        initialized: false,
                                    });
                                    commands.insert_resource(crate::save::ActiveSlot(Some(slot_entry.id)));
                                    commands.insert_resource(ResumeFromCheckpoint);
                                    commands.remove_resource::<SaveMenuMode>();
                                    start_transition(&mut commands, GameState::Playing);
                                }
                            }
                        }
                        SaveMenuMode::Save => {
                            let slot_data = crate::save::slot_data_from_checkpoint(&checkpoint);
                            if save_menu_sel.index < slots.len() {
                                // Overwrite existing slot
                                let id = slots[save_menu_sel.index].id;
                                crate::save::update_slot(id, &slot_data);
                                commands.insert_resource(crate::save::ActiveSlot(Some(id)));
                            } else {
                                // New save
                                let id = crate::save::create_slot(&slot_data);
                                commands.insert_resource(crate::save::ActiveSlot(Some(id)));
                            }
                            // Keep the cache in sync with the slot we just wrote
                            save_cache.refresh();
                            commands.remove_resource::<SaveMenuMode>();
                            next_state.set(GameState::Paused);
                        }
                    }
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
