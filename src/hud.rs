use bevy::prelude::*;

use crate::audio::GameSettings;
use crate::checkpoint::CheckpointData;
use crate::constants::*;
use crate::enemies::ComboTracker;
use crate::health::Health;
use crate::highscore::{HighScore, HighScoreSet, NewHighScoreFlag};
use crate::player::{Coins, Player, Score};
use crate::powerups::{Shield, SpeedBoost, TripleJump};
use crate::state::{GameState, MenuSelection, PauseSelection, SettingsSelection};

/// Marker for the HUD root node so we can despawn it cleanly.
#[derive(Component)]
struct HudRoot;

/// Marker for the health text element.
#[derive(Component)]
struct HealthText;

/// Marker for the score text element.
#[derive(Component)]
struct ScoreText;

/// Marker for the coins text element.
#[derive(Component)]
struct CoinsText;

/// Marker for menu overlay.
#[derive(Component)]
struct MenuOverlay;

/// Marker for individual menu items.
#[derive(Component)]
struct MenuItem(usize);

/// Marker for game-over overlay.
#[derive(Component)]
struct GameOverOverlay;

/// Marker for pause overlay.
#[derive(Component)]
struct PauseOverlay;

/// Marker for pause menu items.
#[derive(Component)]
struct PauseMenuItem(usize);

/// Marker for settings overlay.
#[derive(Component)]
struct SettingsOverlay;

/// Marker for save menu overlay.
#[derive(Component)]
struct SaveMenuOverlay;

/// Marker for save menu slot items.
#[derive(Component)]
struct SaveMenuItem(usize);

/// Marker for settings rows.
#[derive(Component)]
struct SettingsItem(usize);

/// Marker for section banner UI.
#[derive(Component)]
pub struct SectionBannerUi;

/// Marker for combo display text.
#[derive(Component)]
struct ComboText;

/// Marker for powerup bar container.
#[derive(Component)]
struct PowerupBarRoot;

/// Marker for individual powerup bar fill nodes.
#[derive(Component)]
struct PowerupBarFill(usize); // 0=speed, 1=jump, 2=shield

/// Marker for powerup bar label text.
#[derive(Component)]
#[allow(dead_code)]
struct PowerupBarLabel(usize);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app
            // Menu
            .add_systems(OnEnter(GameState::Menu), spawn_menu_overlay)
            .add_systems(OnExit(GameState::Menu), despawn_all::<MenuOverlay>)
            .add_systems(Update, update_menu_highlight.run_if(in_state(GameState::Menu)))
            // In-game HUD
            .add_systems(OnEnter(GameState::Playing), spawn_hud)
            .add_systems(OnExit(GameState::Playing), despawn_all::<HudRoot>)
            .add_systems(Update, (update_hud, update_combo_text, update_powerup_bars).run_if(in_state(GameState::Playing)))
            // Pause
            .add_systems(OnEnter(GameState::Paused), spawn_pause_overlay)
            .add_systems(OnExit(GameState::Paused), despawn_all::<PauseOverlay>)
            .add_systems(Update, update_pause_highlight.run_if(in_state(GameState::Paused)))
            // Settings
            .add_systems(OnEnter(GameState::Settings), spawn_settings_overlay)
            .add_systems(OnExit(GameState::Settings), despawn_all::<SettingsOverlay>)
            .add_systems(Update, update_settings_display.run_if(in_state(GameState::Settings)))
            // Save Menu
            .add_systems(OnEnter(GameState::SaveMenu), spawn_save_menu_overlay)
            .add_systems(OnExit(GameState::SaveMenu), despawn_all::<SaveMenuOverlay>)
            .add_systems(Update, update_save_menu_display.run_if(in_state(GameState::SaveMenu)))
            // Game Over
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over_overlay.after(HighScoreSet))
            .add_systems(OnExit(GameState::GameOver), despawn_all::<GameOverOverlay>);
    }
}

// ---------------------------------------------------------------------------
// Colors
// ---------------------------------------------------------------------------

const COLOR_SELECTED: Color = Color::srgb(0.2, 1.0, 0.4);
const COLOR_UNSELECTED: Color = Color::srgb(0.5, 0.5, 0.5);
const COLOR_TITLE: Color = Color::srgb(0.2, 0.8, 0.2);

// ---------------------------------------------------------------------------
// HUD (in-game)
// ---------------------------------------------------------------------------

fn spawn_hud(mut commands: Commands) {
    commands
        .spawn((
            HudRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Auto,
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Top row: Health, Score, Coins
            parent.spawn(Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            }).with_children(|row| {
                row.spawn((
                    HealthText,
                    Text::new("Health: 3"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.3, 0.3)),
                ));
                row.spawn((
                    ScoreText,
                    Text::new("Score: 0"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(1.0, 1.0, 1.0)),
                ));
                row.spawn((
                    CoinsText,
                    Text::new("Coins: 0"),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.85, 0.0)),
                ));
            });

            // Combo text (centered, below top row)
            parent.spawn((
                ComboText,
                Text::new(""),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(1.0, 0.9, 0.2)),
                Node {
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            ));

            // Powerup bar container
            parent.spawn((
                PowerupBarRoot,
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(12.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
            )).with_children(|bar_root| {
                // 3 powerup bar slots: Speed (blue), Jump (green), Shield (gold)
                let bars = [
                    (0, "SPEED", Color::srgb(0.3, 0.5, 1.0)),
                    (1, "JUMP", Color::srgb(0.2, 0.9, 0.3)),
                    (2, "SHIELD", Color::srgb(1.0, 0.85, 0.0)),
                ];
                for (idx, label, color) in bars {
                    bar_root.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        display: Display::None, // hidden by default
                        ..default()
                    }).with_children(|slot| {
                        // Label
                        slot.spawn((
                            PowerupBarLabel(idx),
                            Text::new(label),
                            TextFont { font_size: 11.0, ..default() },
                            TextColor(color),
                        ));
                        // Bar background
                        slot.spawn(Node {
                            width: Val::Px(POWERUP_BAR_WIDTH),
                            height: Val::Px(POWERUP_BAR_HEIGHT),
                            ..default()
                        }).insert(BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.6)))
                        .with_children(|bg| {
                            // Fill bar
                            bg.spawn((
                                PowerupBarFill(idx),
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(color),
                            ));
                        });
                    });
                }
            });
        });
}

fn update_hud(
    player_query: Query<&Health, With<Player>>,
    score: Res<Score>,
    coins: Res<Coins>,
    mut health_query: Query<&mut Text, (With<HealthText>, Without<ScoreText>, Without<CoinsText>)>,
    mut score_query: Query<&mut Text, (With<ScoreText>, Without<HealthText>, Without<CoinsText>)>,
    mut coins_query: Query<&mut Text, (With<CoinsText>, Without<HealthText>, Without<ScoreText>)>,
) {
    if let Ok(health) = player_query.single() {
        for mut text in &mut health_query {
            **text = format!("Health: {}", health.current);
        }
    }
    for mut text in &mut score_query {
        **text = format!("Score: {}", score.value);
    }
    for mut text in &mut coins_query {
        **text = format!("Coins: {}", coins.count);
    }
}

fn update_combo_text(
    combo: Res<ComboTracker>,
    mut query: Query<(&mut Text, &mut TextColor), With<ComboText>>,
) {
    let Ok((mut text, mut color)) = query.single_mut() else {
        return;
    };

    if combo.count > 0 && !combo.display_timer.is_finished() {
        let multiplier = 2u32.pow(combo.count.min(MAX_COMBO_POWER));
        **text = format!("x{}!", multiplier);
        // Fade out as timer progresses
        let alpha = combo.display_timer.fraction_remaining();
        color.0 = Color::srgba(1.0, 0.9, 0.2, alpha);
    } else {
        **text = String::new();
    }
}

fn update_powerup_bars(
    player_query: Query<(Option<&SpeedBoost>, Option<&TripleJump>, Option<&Shield>), With<Player>>,
    mut bar_fill_query: Query<(&PowerupBarFill, &mut Node), Without<PowerupBarRoot>>,
    bar_root_query: Query<&Children, With<PowerupBarRoot>>,
    mut slot_query: Query<&mut Node, (Without<PowerupBarFill>, Without<PowerupBarRoot>)>,
) {
    let Ok((speed, jump, shield)) = player_query.single() else {
        return;
    };

    // Collect active states: (slot_index, fill_fraction)
    let states: [(usize, Option<f32>); 3] = [
        (0, speed.map(|s| s.timer.fraction_remaining())),
        (1, jump.map(|j| j.timer.fraction_remaining())),
        (2, shield.map(|s| Some(s.hits_remaining as f32 / SHIELD_HITS as f32)).unwrap_or(None)),
    ];

    // Update fill bar widths
    for (fill, mut node) in &mut bar_fill_query {
        if let Some((_, active)) = states.iter().find(|(idx, _)| *idx == fill.0) {
            if let Some(frac) = active {
                node.width = Val::Percent(frac * 100.0);
            }
        }
    }

    // Show/hide slot containers
    if let Ok(children) = bar_root_query.single() {
        for (i, child) in children.iter().enumerate() {
            if i < 3 {
                if let Ok(mut node) = slot_query.get_mut(child) {
                    node.display = if states[i].1.is_some() {
                        Display::Flex
                    } else {
                        Display::None
                    };
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main Menu
// ---------------------------------------------------------------------------

fn spawn_menu_overlay(
    mut commands: Commands,
    high_score: Res<HighScore>,
    mut menu_sel: ResMut<MenuSelection>,
) {
    menu_sel.index = 0;
    let has_saves = !crate::save::list_slots().is_empty();

    commands
        .spawn((
            MenuOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("My Side-Scroller"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(COLOR_TITLE),
            ));

            if high_score.value > 0 {
                parent.spawn((
                    Text::new(format!("High Score: {}", high_score.value)),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.85, 0.0)),
                ));
            }

            // Spacer
            parent.spawn(Node { height: Val::Px(10.0), ..default() });

            // Menu items — conditionally include Load Game
            let mut idx = 0;
            if has_saves {
                parent.spawn((
                    MenuItem(idx),
                    Text::new("Load Game"),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(COLOR_SELECTED),
                ));
                idx += 1;
            }

            let items = ["New Game", "Settings", "Quit"];
            for label in &items {
                parent.spawn((
                    MenuItem(idx),
                    Text::new(*label),
                    TextFont { font_size: 28.0, ..default() },
                    TextColor(if idx == 0 { COLOR_SELECTED } else { COLOR_UNSELECTED }),
                ));
                idx += 1;
            }

            // Spacer
            parent.spawn(Node { height: Val::Px(10.0), ..default() });

            // Controls hint
            parent.spawn((
                Text::new("A/D: Move  |  Space: Jump  |  ESC: Pause"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
            ));
        });
}

fn update_menu_highlight(
    menu_sel: Res<MenuSelection>,
    mut query: Query<(&MenuItem, &mut TextColor)>,
) {
    for (item, mut color) in &mut query {
        color.0 = if item.0 == menu_sel.index { COLOR_SELECTED } else { COLOR_UNSELECTED };
    }
}

// ---------------------------------------------------------------------------
// Pause Menu
// ---------------------------------------------------------------------------

fn spawn_pause_overlay(mut commands: Commands) {
    commands
        .spawn((
            PauseOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));

            parent.spawn(Node { height: Val::Px(10.0), ..default() });

            let items = ["Resume", "Save Game", "Settings", "Quit to Menu", "Quit Game"];
            for (i, label) in items.iter().enumerate() {
                parent.spawn((
                    PauseMenuItem(i),
                    Text::new(*label),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(if i == 0 { COLOR_SELECTED } else { COLOR_UNSELECTED }),
                ));
            }
        });
}

fn update_pause_highlight(
    pause_sel: Res<PauseSelection>,
    mut query: Query<(&PauseMenuItem, &mut TextColor)>,
) {
    for (item, mut color) in &mut query {
        color.0 = if item.0 == pause_sel.index { COLOR_SELECTED } else { COLOR_UNSELECTED };
    }
}

// ---------------------------------------------------------------------------
// Save Menu Screen
// ---------------------------------------------------------------------------

fn spawn_save_menu_overlay(
    mut commands: Commands,
    mode: Option<Res<crate::state::SaveMenuMode>>,
) {
    let mode = mode.map(|m| *m).unwrap_or(crate::state::SaveMenuMode::Load);
    let title = match mode {
        crate::state::SaveMenuMode::Load => "LOAD GAME",
        crate::state::SaveMenuMode::Save => "SAVE GAME",
    };

    let slots = crate::save::list_slots();

    commands
        .spawn((
            SaveMenuOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextFont { font_size: 40.0, ..default() },
                TextColor(COLOR_TITLE),
            ));

            parent.spawn(Node { height: Val::Px(8.0), ..default() });

            if slots.is_empty() && mode == crate::state::SaveMenuMode::Load {
                parent.spawn((
                    Text::new("No saves found"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(Color::srgb(0.5, 0.5, 0.5)),
                ));
            }

            for (i, slot) in slots.iter().enumerate() {
                let label = format!(
                    "Save {} — Section {} — Score {} — {}",
                    slot.id + 1,
                    slot.section + 1,
                    slot.score,
                    slot.timestamp
                );
                parent.spawn((
                    SaveMenuItem(i),
                    Text::new(label),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(if i == 0 { COLOR_SELECTED } else { COLOR_UNSELECTED }),
                ));
            }

            // "New Save" option in Save mode
            if mode == crate::state::SaveMenuMode::Save {
                let idx = slots.len();
                parent.spawn((
                    SaveMenuItem(idx),
                    Text::new("+ New Save"),
                    TextFont { font_size: 22.0, ..default() },
                    TextColor(if idx == 0 { COLOR_SELECTED } else { COLOR_UNSELECTED }),
                ));
            }

            parent.spawn(Node { height: Val::Px(8.0), ..default() });

            let hint = match mode {
                crate::state::SaveMenuMode::Load => "Enter: Load  |  Backspace: Delete  |  ESC: Back",
                crate::state::SaveMenuMode::Save => "Enter: Save  |  ESC: Back",
            };
            parent.spawn((
                Text::new(hint),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
            ));
        });
}

fn update_save_menu_display(
    save_sel: Res<crate::state::SaveMenuSelection>,
    mut query: Query<(&SaveMenuItem, &mut TextColor, &mut Text)>,
) {
    let slots = crate::save::list_slots();
    for (item, mut color, mut text) in &mut query {
        let is_selected = item.0 == save_sel.index;

        if save_sel.confirm_delete && is_selected && item.0 < slots.len() {
            // Show delete confirmation in red
            color.0 = Color::srgb(1.0, 0.3, 0.3);
            if let Some(slot) = slots.get(item.0) {
                **text = format!("DELETE Save {}? Press Enter to confirm", slot.id + 1);
            }
        } else {
            color.0 = if is_selected { COLOR_SELECTED } else { COLOR_UNSELECTED };
            // Restore normal text if not in delete mode
            if item.0 < slots.len() {
                if let Some(slot) = slots.get(item.0) {
                    **text = format!(
                        "Save {} — Section {} — Score {} — {}",
                        slot.id + 1, slot.section + 1, slot.score, slot.timestamp
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Settings Screen
// ---------------------------------------------------------------------------

fn spawn_settings_overlay(mut commands: Commands, settings: Res<GameSettings>) {
    commands
        .spawn((
            SettingsOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(16.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SETTINGS"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(COLOR_TITLE),
            ));

            parent.spawn(Node { height: Val::Px(10.0), ..default() });

            // Volume sliders
            let labels = [
                ("Master Volume", settings.master_volume),
                ("SFX Volume", settings.sfx_volume),
                ("Music Volume", settings.music_volume),
            ];
            for (i, (label, val)) in labels.iter().enumerate() {
                parent.spawn((
                    SettingsItem(i),
                    Text::new(format!("{}: {}", label, volume_bar(*val))),
                    TextFont { font_size: 24.0, ..default() },
                    TextColor(if i == 0 { COLOR_SELECTED } else { COLOR_UNSELECTED }),
                ));
            }

            // Resolution
            let res_label = RESOLUTION_LABELS[settings.resolution_index.min(RESOLUTION_LABELS.len() - 1)];
            parent.spawn((
                SettingsItem(3),
                Text::new(format!("Resolution: {}", res_label)),
                TextFont { font_size: 24.0, ..default() },
                TextColor(COLOR_UNSELECTED),
            ));

            // Fullscreen
            parent.spawn((
                SettingsItem(4),
                Text::new(format!("Fullscreen: {}", if settings.fullscreen { "ON" } else { "OFF" })),
                TextFont { font_size: 24.0, ..default() },
                TextColor(COLOR_UNSELECTED),
            ));

            // Back option
            parent.spawn((
                SettingsItem(5),
                Text::new("Back"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(COLOR_UNSELECTED),
            ));

            parent.spawn(Node { height: Val::Px(10.0), ..default() });

            parent.spawn((
                Text::new("Up/Down: Select  |  Left/Right: Adjust  |  ESC: Back"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::srgb(0.4, 0.4, 0.4)),
            ));
        });
}

fn update_settings_display(
    settings: Res<GameSettings>,
    settings_sel: Res<SettingsSelection>,
    mut query: Query<(&SettingsItem, &mut Text, &mut TextColor)>,
) {
    let vol_labels = ["Master Volume", "SFX Volume", "Music Volume"];
    let vol_values = [settings.master_volume, settings.sfx_volume, settings.music_volume];

    for (item, mut text, mut color) in &mut query {
        let is_selected = item.0 == settings_sel.index;
        color.0 = if is_selected { COLOR_SELECTED } else { COLOR_UNSELECTED };

        let arrow_l = if is_selected { "< " } else { "  " };
        let arrow_r = if is_selected { " >" } else { "  " };

        match item.0 {
            0..=2 => {
                **text = format!("{}: {}{}{}", vol_labels[item.0], arrow_l, volume_bar(vol_values[item.0]), arrow_r);
            }
            3 => {
                let res_label = RESOLUTION_LABELS[settings.resolution_index.min(RESOLUTION_LABELS.len() - 1)];
                **text = format!("Resolution: {}{}{}", arrow_l, res_label, arrow_r);
            }
            4 => {
                let val = if settings.fullscreen { "ON" } else { "OFF" };
                **text = format!("Fullscreen: {}{}{}", arrow_l, val, arrow_r);
            }
            _ => {} // Back — no dynamic content
        }
    }
}

/// Build a visual volume bar like `[========--] 80%`
fn volume_bar(val: f32) -> String {
    let pct = (val * 100.0).round() as u32;
    let filled = (val * 10.0).round() as usize;
    let empty = 10 - filled;
    format!("[{}{}] {}%", "=".repeat(filled), "-".repeat(empty), pct)
}

// ---------------------------------------------------------------------------
// Game Over
// ---------------------------------------------------------------------------

fn spawn_game_over_overlay(
    mut commands: Commands,
    score: Res<Score>,
    high_score: Res<HighScore>,
    new_high_score: Option<Res<NewHighScoreFlag>>,
) {
    commands
        .spawn((
            GameOverOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.0, 0.0, 0.8)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GAME OVER"),
                TextFont { font_size: 48.0, ..default() },
                TextColor(Color::srgb(1.0, 0.2, 0.2)),
            ));
            parent.spawn((
                Text::new(format!("Score: {}", score.value)),
                TextFont { font_size: 28.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
            parent.spawn((
                Text::new(format!("High Score: {}", high_score.value)),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
            ));
            if new_high_score.is_some() {
                parent.spawn((
                    Text::new("NEW HIGH SCORE!"),
                    TextFont { font_size: 32.0, ..default() },
                    TextColor(Color::srgb(1.0, 1.0, 0.0)),
                ));
            }
            parent.spawn((
                Text::new("Press SPACE to Retry"),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn despawn_all<T: Component>(mut commands: Commands, query: Query<Entity, With<T>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
