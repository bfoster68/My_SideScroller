use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

use crate::checkpoint::CheckpointData;
use crate::enemies::{ChargingEnemy, Enemy, FlyingEnemy, FlyingRangedEnemy, Projectile, ShooterEnemy};
use crate::level::Difficulty;
use crate::particles::Particle;
use crate::player::{Player, Score, Velocity};

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
            .add_systems(Update, (toggle_debug_overlay, update_debug_text));
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
    // Player info
    player_query: Query<(&Transform, &Velocity), With<Player>>,
    // Game state
    difficulty: Res<Difficulty>,
    score: Res<Score>,
    checkpoint: Res<CheckpointData>,
    // Entity counts
    enemy_query: Query<(), With<Enemy>>,
    flying_query: Query<(), With<FlyingEnemy>>,
    shooter_query: Query<(), With<ShooterEnemy>>,
    charging_query: Query<(), With<ChargingEnemy>>,
    flying_ranged_query: Query<(), With<FlyingRangedEnemy>>,
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
    let (player_pos, player_vel) = if let Ok((tf, vel)) = player_query.single() {
        (
            format!("({:.0}, {:.0})", tf.translation.x, tf.translation.y),
            format!("({:.0}, {:.0})", vel.0.x, vel.0.y),
        )
    } else {
        ("--".to_string(), "--".to_string())
    };

    // Enemy counts
    let total_enemies = enemy_query.iter().count();
    let walkers = total_enemies
        - flying_query.iter().count()
        - shooter_query.iter().count()
        - charging_query.iter().count()
        - flying_ranged_query.iter().count();
    let flyers = flying_query.iter().count();
    let shooters = shooter_query.iter().count();
    let chargers = charging_query.iter().count();
    let ranged = flying_ranged_query.iter().count();
    let particles = particle_query.iter().count();
    let projectiles = projectile_query.iter().count();

    let debug_text = format!(
        "FPS: {:.0}  |  Entities: {}\n\
         Pos: {}  Vel: {}\n\
         Score: {}  |  Difficulty: {:.0}%\n\
         Section: {}  |  Checkpoint: {}\n\
         Enemies: {} (W:{} F:{} S:{} C:{} R:{})\n\
         Particles: {}/{}  |  Projectiles: {}",
        fps,
        entity_count,
        player_pos,
        player_vel,
        score.value,
        difficulty.value * 100.0,
        checkpoint.section,
        checkpoint.last_checkpoint_score,
        total_enemies, walkers, flyers, shooters, chargers, ranged,
        particles, MAX_PARTICLES,
        projectiles,
    );

    for mut text in &mut text_query {
        **text = debug_text.clone();
    }
}

use crate::constants::MAX_PARTICLES;
