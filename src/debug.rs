use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::checkpoint::CheckpointData;
use crate::constants::MAX_HEALTH;
use crate::enemies::{ComboTracker, Enemy, Projectile};
use crate::health::{Health, Invincible};
use crate::level::{ChunkTracker, Difficulty, Platform};
use crate::particles::Particle;
use crate::player::{Grounded, Player, Score, Velocity};
use crate::powerups::{SpeedBoost, TripleJump, Shield};
use crate::state::GameState;

/// When active, player is permanently invincible (god mode).
#[derive(Resource, Default)]
pub struct GodMode(pub bool);

/// Marker for debug overlay root node.
#[derive(Component)]
struct DebugRoot;

/// Marker for the debug text element.
#[derive(Component)]
struct DebugText;

/// Whether the debug overlay is currently visible.
#[derive(Resource, Default)]
struct DebugVisible(bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .init_resource::<DebugVisible>()
            .init_resource::<GodMode>()
            .add_systems(Update, (toggle_debug_overlay, update_debug_text, debug_cheats));
    }
}

/// Toggle the debug overlay with backtick (`) key.
fn toggle_debug_overlay(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<DebugVisible>,
    overlay_query: Query<Entity, With<DebugRoot>>,
) {
    if !keyboard.just_pressed(KeyCode::Backquote) {
        return;
    }

    visible.0 = !visible.0;

    if visible.0 {
        commands
            .spawn((
                DebugRoot,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(48.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                ZIndex(99),
            ))
            .with_children(|parent| {
                parent.spawn((
                    DebugText,
                    Text::new("Loading..."),
                    TextFont { font_size: 20.0, ..default() },
                    TextColor(Color::srgb(0.0, 1.0, 0.0)),
                ));
            });
    } else {
        for entity in &overlay_query {
            commands.entity(entity).despawn();
        }
    }
}

/// Update debug text with comprehensive game metrics.
fn update_debug_text(
    visible: Res<DebugVisible>,
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<&mut Text, With<DebugText>>,
    god_mode: Res<GodMode>,
    // Player info
    player_query: Query<
        (&Transform, &Velocity, &Grounded, &Health, Option<&Invincible>, Option<&SpeedBoost>, Option<&TripleJump>, Option<&Shield>),
        With<Player>,
    >,
    // Game state
    difficulty: Res<Difficulty>,
    score: Res<Score>,
    checkpoint: Res<CheckpointData>,
    combo: Res<ComboTracker>,
    chunk_tracker: Res<ChunkTracker>,
    // Entity counts (combined into fewer queries)
    platform_query: Query<(), With<Platform>>,
    enemy_query: Query<(), With<Enemy>>,
    particle_query: Query<(), With<Particle>>,
    projectile_query: Query<(), With<Projectile>>,
) {
    if !visible.0 {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as u32;

    // Player state
    let (player_pos, player_vel, player_state, health_str, powerups_str) =
        if let Ok((tf, vel, grounded, health, invincible, speed, triple, shield)) = player_query.single() {
            let ground_str = if grounded.on_ground { "GND" } else { "AIR" };
            let inv_str = if invincible.is_some() { " INV" } else { "" };
            let mut pups = Vec::new();
            if speed.is_some() { pups.push("SPD"); }
            if triple.is_some() { pups.push("3JMP"); }
            if shield.is_some() { pups.push("SHD"); }
            let pup_str = if pups.is_empty() { "none".to_string() } else { pups.join(" ") };
            (
                format!("({:.0}, {:.0})", tf.translation.x, tf.translation.y),
                format!("({:.0}, {:.0})", vel.0.x, vel.0.y),
                format!("{}{}", ground_str, inv_str),
                format!("{}/{}", health.current, health.max),
                pup_str,
            )
        } else {
            ("--".into(), "--".into(), "--".into(), "--".into(), "--".into())
        };

    // Entity counts
    let total_enemies = enemy_query.iter().count();
    let particles = particle_query.iter().count();
    let projectiles = projectile_query.iter().count();
    let platforms = platform_query.iter().count();

    let god = if god_mode.0 { " [GOD]" } else { "" };
    let combo_str = if combo.count > 0 {
        format!("  Combo: {}x", 2u32.pow(combo.count.min(4)))
    } else {
        String::new()
    };

    let debug_text = format!(
        "FPS: {:.0}  |  Entities: {}{}\n\
         Pos: {}  Vel: {}\n\
         State: {}  |  HP: {}  |  Powerups: {}\n\
         Score: {}  |  Diff: {:.0}%{}\n\
         Section: {}  |  Checkpoint: {}\n\
         Enemies: {}  |  Platforms: {}  |  Proj: {}\n\
         Particles: {}  |  Gen: {:.0}  |  Gnd: {:.0}\n\
         0:God  9:Heal",
        fps,
        entity_count,
        god,
        player_pos,
        player_vel,
        player_state,
        health_str,
        powerups_str,
        score.value,
        difficulty.value * 100.0,
        combo_str,
        checkpoint.section,
        checkpoint.last_checkpoint_score,
        total_enemies,
        platforms,
        projectiles,
        particles,
        chunk_tracker.rightmost_platform_x,
        chunk_tracker.rightmost_ground_x,
    );

    for mut text in &mut text_query {
        **text = debug_text.clone();
    }
}

/// Debug cheats: 0 = toggle god mode, 9 = refill health.
fn debug_cheats(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut god_mode: ResMut<GodMode>,
    mut player_query: Query<(Entity, &mut Health), With<Player>>,
    state: Res<State<GameState>>,
) {
    if *state.get() != GameState::Playing {
        return;
    }

    // 0: Toggle god mode (permanent invincibility)
    if keyboard.just_pressed(KeyCode::Digit0) {
        god_mode.0 = !god_mode.0;
        if let Ok((entity, _)) = player_query.single_mut() {
            if god_mode.0 {
                // Insert a very long invincibility
                commands.entity(entity).insert(Invincible {
                    timer: Timer::from_seconds(999999.0, TimerMode::Once),
                });
                info!("GOD MODE: ON");
            } else {
                commands.entity(entity).remove::<Invincible>();
                info!("GOD MODE: OFF");
            }
        }
    }

    // 9: Refill health to max
    if keyboard.just_pressed(KeyCode::Digit9) {
        if let Ok((_, mut health)) = player_query.single_mut() {
            health.current = MAX_HEALTH;
            info!("HEALTH REFILLED");
        }
    }

    // Re-apply god mode invincibility if it expired or was removed
    if god_mode.0 {
        if let Ok((entity, _)) = player_query.single_mut() {
            commands.entity(entity).insert(Invincible {
                timer: Timer::from_seconds(999999.0, TimerMode::Once),
            });
        }
    }
}
