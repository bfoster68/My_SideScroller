use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::player::{Player, PlayerMovementSet};
use crate::state::GameState;

/// Marker for spike hazard entities.
#[derive(Component)]
pub struct Spike;

/// Moving saw that patrols back and forth on a platform.
#[derive(Component)]
pub struct Saw {
    pub left_bound: f32,
    pub right_bound: f32,
    pub direction: f32,
}

/// Lava pool in a ground gap — instant kill.
#[derive(Component)]
pub struct Lava;

pub struct HazardsPlugin;

impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                saw_movement,
                saw_animate,
                lava_animate,
                spike_player_collision,
                saw_player_collision,
                lava_player_collision,
            )
                .after(PlayerMovementSet)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Spike
// ---------------------------------------------------------------------------

/// Spawn a spike on top of a platform. Called from level generation.
pub fn spawn_spike(commands: &mut Commands, x: f32, platform_y: f32, image: Handle<Image>) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SPIKE_HEIGHT / 2.0);

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(SPIKE_WIDTH, SPIKE_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(x, spawn_y, SPIKE_Z),
        Spike,
    ));
}

/// Spawn a spike as a child of a moving platform (local coordinates).
pub fn spawn_spike_on_moving(parent: &mut ChildSpawnerCommands, image: Handle<Image>) {
    let local_y = (PLATFORM_HEIGHT / 2.0) + (SPIKE_HEIGHT / 2.0);

    parent.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(SPIKE_WIDTH, SPIKE_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, local_y, SPIKE_Z),
        Spike,
    ));
}

// ---------------------------------------------------------------------------
// Moving Saw
// ---------------------------------------------------------------------------

/// Spawn a saw that patrols on top of a platform.
pub fn spawn_saw(commands: &mut Commands, x: f32, platform_y: f32, platform_width: f32, image: Handle<Image>) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SAW_SIZE / 2.0);
    let half_plat = platform_width / 2.0;
    let saw_half = SAW_SIZE / 2.0;

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::splat(SAW_SIZE)),
            ..default()
        },
        Transform::from_xyz(x, spawn_y, SAW_Z),
        Saw {
            left_bound: x - half_plat + saw_half,
            right_bound: x + half_plat - saw_half,
            direction: 1.0,
        },
    ));
}

/// Spawn a saw as a child of a moving platform (local coordinates).
pub fn spawn_saw_on_moving(parent: &mut ChildSpawnerCommands, platform_width: f32, image: Handle<Image>) {
    let local_y = (PLATFORM_HEIGHT / 2.0) + (SAW_SIZE / 2.0);
    let half_plat = platform_width / 2.0;
    let saw_half = SAW_SIZE / 2.0;

    parent.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::splat(SAW_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, local_y, SAW_Z),
        Saw {
            left_bound: -half_plat + saw_half,
            right_bound: half_plat - saw_half,
            direction: 1.0,
        },
    ));
}

/// Move saws back and forth within their patrol bounds.
fn saw_movement(time: Res<Time>, mut query: Query<(&mut Transform, &mut Saw)>) {
    let dt = time.delta_secs();
    for (mut tf, mut saw) in &mut query {
        tf.translation.x += SAW_SPEED * saw.direction * dt;

        if tf.translation.x >= saw.right_bound {
            tf.translation.x = saw.right_bound;
            saw.direction = -1.0;
        } else if tf.translation.x <= saw.left_bound {
            tf.translation.x = saw.left_bound;
            saw.direction = 1.0;
        }
    }
}

/// Spin the saw by rotating the transform.
fn saw_animate(time: Res<Time>, mut query: Query<&mut Transform, With<Saw>>) {
    let dt = time.delta_secs();
    for mut tf in &mut query {
        tf.rotation *= Quat::from_rotation_z(-6.0 * dt); // fast spin
    }
}

// ---------------------------------------------------------------------------
// Lava
// ---------------------------------------------------------------------------

/// Spawn a lava pool at a ground gap position.
pub fn spawn_lava(commands: &mut Commands, x: f32, width: f32, image: Handle<Image>) {
    let y = GROUND_Y - (GROUND_HEIGHT / 2.0) + (LAVA_HEIGHT / 2.0) - 5.0;

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(width, LAVA_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(x, y, LAVA_Z),
        Lava,
    ));
}

/// Pulsing glow for lava — oscillate brightness via tint.
fn lava_animate(time: Res<Time>, mut query: Query<&mut Sprite, With<Lava>>) {
    let t = time.elapsed_secs();
    let pulse = 0.85 + 0.15 * (t * 3.0).sin();
    for mut sprite in &mut query {
        sprite.color = Color::srgba(1.0, pulse, pulse, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Collision systems
// ---------------------------------------------------------------------------

/// Damage the player when they touch a spike.
fn spike_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    spike_query: Query<&GlobalTransform, With<Spike>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else {
        return;
    };

    if invincible.is_some() {
        return;
    }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let spike_half_w = SPIKE_WIDTH / 2.0;
    let spike_half_h = SPIKE_HEIGHT / 2.0;

    for spike_gtf in &spike_query {
        let spike_pos = spike_gtf.translation();
        let overlap_x = (player_half_w + spike_half_w)
            - (player_tf.translation.x - spike_pos.x).abs();
        let overlap_y = (player_half_h + spike_half_h)
            - (player_tf.translation.y - spike_pos.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            damage_events.write(DamageEvent { amount: SPIKE_DAMAGE });

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}

/// Damage the player when they touch a saw (no stomp — always hurts).
fn saw_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    saw_query: Query<&GlobalTransform, With<Saw>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else {
        return;
    };

    if invincible.is_some() {
        return;
    }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let saw_half = SAW_SIZE / 2.0;

    for saw_gtf in &saw_query {
        let saw_pos = saw_gtf.translation();
        let overlap_x =
            (player_half_w + saw_half) - (player_tf.translation.x - saw_pos.x).abs();
        let overlap_y =
            (player_half_h + saw_half) - (player_tf.translation.y - saw_pos.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            damage_events.write(DamageEvent { amount: SAW_DAMAGE });

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}

/// Damage the player when they touch lava (instant kill).
fn lava_player_collision(
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    lava_query: Query<(&Transform, &Sprite), (With<Lava>, Without<Player>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else {
        return;
    };

    if invincible.is_some() {
        return;
    }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (lava_tf, lava_sprite) in &lava_query {
        let lava_size = lava_sprite.custom_size.unwrap_or(Vec2::new(GROUND_SEGMENT_WIDTH, LAVA_HEIGHT));
        let lava_half_w = lava_size.x / 2.0;
        let lava_half_h = lava_size.y / 2.0;

        let overlap_x = (player_half_w + lava_half_w)
            - (player_tf.translation.x - lava_tf.translation.x).abs();
        let overlap_y = (player_half_h + lava_half_h)
            - (player_tf.translation.y - lava_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            damage_events.write(DamageEvent { amount: LAVA_DAMAGE });
            break;
        }
    }
}
