use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::player::{Player, PlayerMovementSet, Score, Velocity};
use crate::state::GameState;

/// Marker component for enemy entities (all types share this for queries).
#[derive(Component)]
pub struct Enemy;

/// Defines the horizontal patrol bounds for a walking enemy.
#[derive(Component)]
pub struct Patrol {
    pub left_bound: f32,
    pub right_bound: f32,
    pub direction: f32, // -1.0 or 1.0
}

/// Flying enemy that oscillates on a sine wave.
#[derive(Component)]
pub struct FlyingEnemy {
    pub base_y: f32,
    pub amplitude: f32,
    pub frequency: f32,
}

/// Shooter enemy that fires projectiles at intervals.
#[derive(Component)]
pub struct ShooterEnemy;

/// Timer for shooter firing interval.
#[derive(Component)]
pub struct ShootTimer {
    pub timer: Timer,
}

/// Projectile fired by shooter enemies.
#[derive(Component)]
pub struct Projectile {
    pub velocity: Vec2,
    pub lifetime: Timer,
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                enemy_patrol,
                flying_enemy_movement,
                shooter_fire,
                projectile_update,
                enemy_player_collision,
                projectile_player_collision,
            )
                .chain()
                .after(PlayerMovementSet)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Walking Enemy (original)
// ---------------------------------------------------------------------------

/// Spawn an enemy on top of a platform. Called from level generation.
pub fn spawn_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
    platform_width: f32,
) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let left = platform_x - half_plat + enemy_half;
    let right = platform_x + half_plat - enemy_half;
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.85, 0.2, 0.15), // red/orange
            Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
        ),
        Transform::from_xyz(platform_x, spawn_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: left,
            right_bound: right,
            direction: 1.0,
        },
    ));
}

/// Spawn an enemy as a child of a moving platform (local coordinates).
pub fn spawn_enemy_on_moving(parent: &mut ChildSpawnerCommands, platform_width: f32) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let local_y = (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    parent.spawn((
        Sprite::from_color(
            Color::srgb(0.85, 0.2, 0.15),
            Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
        ),
        Transform::from_xyz(0.0, local_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: -half_plat + enemy_half,
            right_bound: half_plat - enemy_half,
            direction: 1.0,
        },
    ));
}

/// Move enemies back and forth within their patrol bounds.
fn enemy_patrol(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Patrol, &mut Sprite), (With<Enemy>, Without<FlyingEnemy>, Without<ShooterEnemy>)>,
) {
    for (mut transform, mut patrol, mut sprite) in &mut query {
        transform.translation.x += patrol.direction * ENEMY_SPEED * time.delta_secs();

        if transform.translation.x >= patrol.right_bound {
            transform.translation.x = patrol.right_bound;
            patrol.direction = -1.0;
        } else if transform.translation.x <= patrol.left_bound {
            transform.translation.x = patrol.left_bound;
            patrol.direction = 1.0;
        }

        sprite.flip_x = patrol.direction < 0.0;
    }
}

// ---------------------------------------------------------------------------
// Flying Enemy
// ---------------------------------------------------------------------------

/// Spawn a flying enemy above a platform position.
pub fn spawn_flying_enemy(commands: &mut Commands, x: f32, y: f32) {
    let hover_y = y + 80.0; // floats well above the platform

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.6, 0.3, 0.8), // purple
            Vec2::splat(FLYING_ENEMY_SIZE),
        ),
        Transform::from_xyz(x, hover_y, FLYING_ENEMY_Z),
        Enemy,
        FlyingEnemy {
            base_y: hover_y,
            amplitude: FLYING_ENEMY_AMPLITUDE,
            frequency: FLYING_ENEMY_FREQUENCY,
        },
    ));
}

/// Sine-wave oscillation for flying enemies.
fn flying_enemy_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &FlyingEnemy)>,
) {
    let t = time.elapsed_secs();
    for (mut tf, flying) in &mut query {
        tf.translation.y = flying.base_y + (t * flying.frequency).sin() * flying.amplitude;
    }
}

// ---------------------------------------------------------------------------
// Shooter Enemy
// ---------------------------------------------------------------------------

/// Spawn a shooter enemy on top of a platform.
pub fn spawn_shooter_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SHOOTER_HEIGHT / 2.0);

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.2, 0.7, 0.3), // green
            Vec2::new(SHOOTER_WIDTH, SHOOTER_HEIGHT),
        ),
        Transform::from_xyz(platform_x, spawn_y, SHOOTER_Z),
        Enemy,
        ShooterEnemy,
        ShootTimer {
            timer: Timer::from_seconds(SHOOTER_FIRE_INTERVAL, TimerMode::Repeating),
        },
    ));
}

/// Shooter fires projectiles toward the player at intervals.
fn shooter_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&GlobalTransform, &mut ShootTimer), With<ShooterEnemy>>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    for (shooter_gtf, mut shoot_timer) in &mut query {
        shoot_timer.timer.tick(time.delta());

        if shoot_timer.timer.just_finished() {
            let shooter_pos = shooter_gtf.translation();
            // Direction toward the player
            let dir = Vec2::new(
                player_tf.translation.x - shooter_pos.x,
                player_tf.translation.y - shooter_pos.y,
            )
            .normalize_or_zero();

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(0.9, 1.0, 0.3), // yellow-green bullet
                    Vec2::splat(PROJECTILE_SIZE),
                ),
                Transform::from_xyz(shooter_pos.x, shooter_pos.y, PROJECTILE_Z),
                Projectile {
                    velocity: dir * PROJECTILE_SPEED,
                    lifetime: Timer::from_seconds(PROJECTILE_LIFETIME, TimerMode::Once),
                },
            ));
        }
    }
}

/// Move projectiles and despawn when lifetime expires.
fn projectile_update(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Projectile)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tf, mut proj) in &mut query {
        proj.lifetime.tick(time.delta());
        tf.translation.x += proj.velocity.x * dt;
        tf.translation.y += proj.velocity.y * dt;

        if proj.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Collision
// ---------------------------------------------------------------------------

/// Check player–enemy collisions: stomp from above kills enemy, side contact deals damage.
/// Works for walking, flying, and shooter enemies (all have Enemy marker).
fn enemy_player_collision(
    mut commands: Commands,
    mut player_query: Query<
        (&Transform, &mut Velocity, Option<&Invincible>),
        With<Player>,
    >,
    enemy_query: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<Projectile>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut score: ResMut<Score>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, mut player_vel, invincible)) = player_query.single_mut() else {
        return;
    };

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let enemy_half_w = ENEMY_WIDTH / 2.0;
    let enemy_half_h = ENEMY_HEIGHT / 2.0;

    for (enemy_entity, enemy_gtf) in &enemy_query {
        let enemy_pos = enemy_gtf.translation();
        let overlap_x = (player_half_w + enemy_half_w)
            - (player_tf.translation.x - enemy_pos.x).abs();
        let overlap_y = (player_half_h + enemy_half_h)
            - (player_tf.translation.y - enemy_pos.y).abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            continue;
        }

        let player_bottom = player_tf.translation.y - player_half_h;
        let stomp_zone =
            enemy_pos.y + enemy_half_h * (1.0 - 2.0 * ENEMY_STOMP_THRESHOLD);

        let is_stomp = player_vel.0.y < 0.0 && player_bottom >= stomp_zone;

        if is_stomp {
            commands.entity(enemy_entity).despawn();
            player_vel.0.y = ENEMY_STOMP_BOUNCE;
            score.value += ENEMY_KILL_SCORE;
        } else if invincible.is_none() {
            damage_events.write(DamageEvent { amount: 1 });
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    commands.spawn(AudioPlayer::new(handle.clone()));
                }
            }
            break;
        }
    }
}

/// Projectile–player collision.
fn projectile_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
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
    let proj_half = PROJECTILE_SIZE / 2.0;

    for (entity, proj_tf) in &projectile_query {
        let overlap_x = (player_half_w + proj_half)
            - (player_tf.translation.x - proj_tf.translation.x).abs();
        let overlap_y = (player_half_h + proj_half)
            - (player_tf.translation.y - proj_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            commands.entity(entity).despawn();
            damage_events.write(DamageEvent {
                amount: PROJECTILE_DAMAGE,
            });

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    commands.spawn(AudioPlayer::new(handle.clone()));
                }
            }

            break;
        }
    }
}
