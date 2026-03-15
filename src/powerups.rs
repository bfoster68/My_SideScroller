use bevy::prelude::*;
use rand::Rng;

use crate::constants::*;
use crate::player::Player;
use crate::state::GameState;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Which type of power-up this pickup represents.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerupKind {
    SpeedBoost,
    TripleJump,
    Shield,
}

/// Bobbing animation data for power-up pickups.
#[derive(Component)]
pub struct PowerupBob {
    pub base_y: f32,
    pub phase: f32,
}

/// Active speed boost on the player.
#[derive(Component)]
pub struct SpeedBoost {
    pub timer: Timer,
    pub multiplier: f32,
}

/// Active triple-jump on the player.
#[derive(Component)]
pub struct TripleJump {
    pub timer: Timer,
}

/// Active shield on the player — absorbs hits.
#[derive(Component)]
pub struct Shield {
    pub hits_remaining: i32,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct PowerupsPlugin;

impl Plugin for PowerupsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                powerup_bob_animate,
                powerup_pickup_collision,
                tick_speed_boost,
                tick_triple_jump,
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Spawning
// ---------------------------------------------------------------------------

/// Spawn a random power-up pickup above a platform.
pub fn spawn_powerup(commands: &mut Commands, x: f32, platform_y: f32) {
    let mut rng = rand::thread_rng();
    let kind = match rng.gen_range(0..3) {
        0 => PowerupKind::SpeedBoost,
        1 => PowerupKind::TripleJump,
        _ => PowerupKind::Shield,
    };

    let color = match kind {
        PowerupKind::SpeedBoost => Color::srgb(0.3, 0.5, 1.0),  // blue
        PowerupKind::TripleJump => Color::srgb(0.2, 0.9, 0.3),  // green
        PowerupKind::Shield => Color::srgb(1.0, 0.85, 0.0),     // gold
    };

    let y = platform_y + (PLATFORM_HEIGHT / 2.0) + COIN_FLOAT_HEIGHT + 10.0;

    commands.spawn((
        Sprite::from_color(color, Vec2::splat(POWERUP_SIZE)),
        Transform::from_xyz(x, y, POWERUP_Z),
        kind,
        PowerupBob {
            base_y: y,
            phase: x * 0.1,
        },
    ));
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Bob power-ups up and down (like coins).
fn powerup_bob_animate(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &PowerupBob), With<PowerupKind>>,
) {
    let t = time.elapsed_secs();
    for (mut tf, bob) in &mut query {
        tf.translation.y =
            bob.base_y + (t * POWERUP_BOB_SPEED + bob.phase).sin() * POWERUP_BOB_AMPLITUDE;
    }
}

/// Player picks up power-ups on overlap.
fn powerup_pickup_collision(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    powerup_query: Query<(Entity, &Transform, &PowerupKind)>,
) {
    let Ok((player_entity, player_tf)) = player_query.single() else {
        return;
    };

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let pu_half = POWERUP_SIZE / 2.0;

    for (pu_entity, pu_tf, kind) in &powerup_query {
        let overlap_x = (player_half_w + pu_half)
            - (player_tf.translation.x - pu_tf.translation.x).abs();
        let overlap_y = (player_half_h + pu_half)
            - (player_tf.translation.y - pu_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            commands.entity(pu_entity).despawn();

            match kind {
                PowerupKind::SpeedBoost => {
                    commands.entity(player_entity).insert(SpeedBoost {
                        timer: Timer::from_seconds(SPEED_BOOST_DURATION, TimerMode::Once),
                        multiplier: SPEED_BOOST_MULTIPLIER,
                    });
                }
                PowerupKind::TripleJump => {
                    commands.entity(player_entity).insert(TripleJump {
                        timer: Timer::from_seconds(TRIPLE_JUMP_DURATION, TimerMode::Once),
                    });
                }
                PowerupKind::Shield => {
                    commands.entity(player_entity).insert(Shield {
                        hits_remaining: SHIELD_HITS,
                    });
                }
            }
        }
    }
}

/// Tick speed boost timer and remove when expired.
fn tick_speed_boost(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SpeedBoost), With<Player>>,
) {
    let Ok((entity, mut boost)) = query.single_mut() else {
        return;
    };

    boost.timer.tick(time.delta());
    if boost.timer.is_finished() {
        commands.entity(entity).remove::<SpeedBoost>();
    }
}

/// Tick triple-jump timer and remove when expired.
fn tick_triple_jump(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TripleJump), With<Player>>,
) {
    let Ok((entity, mut tj)) = query.single_mut() else {
        return;
    };

    tj.timer.tick(time.delta());
    if tj.timer.is_finished() {
        commands.entity(entity).remove::<TripleJump>();
    }
}
