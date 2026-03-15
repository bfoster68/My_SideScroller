use bevy::prelude::*;

use crate::health::Health;
use crate::highscore::{HighScore, HighScoreSet, NewHighScoreFlag};
use crate::player::{Coins, Player, Score};
use crate::state::GameState;

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

/// Marker for menu overlay text.
#[derive(Component)]
struct MenuOverlay;

/// Marker for game-over overlay text.
#[derive(Component)]
struct GameOverOverlay;

/// Marker for pause overlay text.
#[derive(Component)]
struct PauseOverlay;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app
            // Spawn / despawn overlays on state transitions
            .add_systems(OnEnter(GameState::Menu), spawn_menu_overlay)
            .add_systems(OnExit(GameState::Menu), despawn_all::<MenuOverlay>)
            .add_systems(OnEnter(GameState::Playing), spawn_hud)
            .add_systems(OnExit(GameState::Playing), despawn_all::<HudRoot>)
            .add_systems(OnEnter(GameState::Paused), spawn_pause_overlay)
            .add_systems(OnExit(GameState::Paused), despawn_all::<PauseOverlay>)
            .add_systems(OnEnter(GameState::GameOver), spawn_game_over_overlay.after(HighScoreSet))
            .add_systems(OnExit(GameState::GameOver), despawn_all::<GameOverOverlay>)
            // Update HUD every frame while playing
            .add_systems(
                Update,
                update_hud.run_if(in_state(GameState::Playing)),
            );
    }
}

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
                justify_content: JustifyContent::SpaceBetween,
                position_type: PositionType::Absolute,
                ..default()
            },
        ))
        .with_children(|parent| {
            // Health (left)
            parent.spawn((
                HealthText,
                Text::new("Health: 3"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.3, 0.3)),
            ));

            // Score (center)
            parent.spawn((
                ScoreText,
                Text::new("Score: 0"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));

            // Coins (right)
            parent.spawn((
                CoinsText,
                Text::new("Coins: 0"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
            ));
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

// ---------------------------------------------------------------------------
// Overlay screens
// ---------------------------------------------------------------------------

fn spawn_menu_overlay(mut commands: Commands, high_score: Res<HighScore>) {
    commands
        .spawn((
            MenuOverlay,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("My Side-Scroller"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(0.2, 0.8, 0.2)),
            ));

            // Show high score if one exists
            if high_score.value > 0 {
                parent.spawn((
                    Text::new(format!("High Score: {}", high_score.value)),
                    TextFont {
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.85, 0.0)),
                ));
            }

            parent.spawn((
                Text::new("Press SPACE to Play"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
            parent.spawn((
                Text::new("A/D: Move  |  Space: Jump  |  ESC: Pause"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
            ));
        });
}

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
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("PAUSED"),
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));
            parent.spawn((
                Text::new("Press ESC to resume"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

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
                TextFont {
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.2, 0.2)),
            ));

            // Final score
            parent.spawn((
                Text::new(format!("Score: {}", score.value)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 1.0)),
            ));

            // High score
            parent.spawn((
                Text::new(format!("High Score: {}", high_score.value)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.85, 0.0)),
            ));

            // NEW HIGH SCORE callout
            if new_high_score.is_some() {
                parent.spawn((
                    Text::new("NEW HIGH SCORE!"),
                    TextFont {
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 1.0, 0.0)),
                ));
            }

            parent.spawn((
                Text::new("Press SPACE to Retry"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
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
