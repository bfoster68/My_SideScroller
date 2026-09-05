use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::enemies::{Enemy, Patrol};
use crate::level::Difficulty;
use crate::player::Player;
use crate::sprites::GameSprites;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

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

/// Charging enemy — patrols normally until player is in range, then charges.
#[derive(Component)]
pub struct ChargingEnemy {
    pub state: ChargeState,
    pub charge_direction: f32,
    pub state_timer: Timer,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ChargeState {
    Idle,
    WindingUp,
    Charging,
    Recovering,
}

/// Flying ranged enemy — hovers and fires downward projectiles.
#[derive(Component)]
pub struct FlyingRangedEnemy;

/// Seconds a charging enemy waits after recovering before it can detect the player again.
/// (Local to this module so constants.rs doesn't need to change.)
const CHARGING_COOLDOWN: f32 = 1.0;

// ---------------------------------------------------------------------------
// Spawn functions
// ---------------------------------------------------------------------------

/// Spawn an enemy on top of a platform.
pub fn spawn_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
    platform_width: f32,
    image: Handle<Image>,
) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let left = platform_x - half_plat + enemy_half;
    let right = platform_x + half_plat - enemy_half;
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT)),
            ..default()
        },
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
pub fn spawn_enemy_on_moving(parent: &mut ChildSpawnerCommands, platform_width: f32, image: Handle<Image>) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let local_y = (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    parent.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, local_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: -half_plat + enemy_half,
            right_bound: half_plat - enemy_half,
            direction: 1.0,
        },
    ));
}

/// Spawn a flying enemy above a platform position.
pub fn spawn_flying_enemy(commands: &mut Commands, x: f32, y: f32, image: Handle<Image>) {
    let hover_y = y + 80.0;

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::splat(FLYING_ENEMY_SIZE)),
            ..default()
        },
        Transform::from_xyz(x, hover_y, FLYING_ENEMY_Z),
        Enemy,
        FlyingEnemy {
            base_y: hover_y,
            amplitude: FLYING_ENEMY_AMPLITUDE,
            frequency: FLYING_ENEMY_FREQUENCY,
        },
    ));
}

/// Spawn a shooter enemy on top of a platform.
pub fn spawn_shooter_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
    image: Handle<Image>,
) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SHOOTER_HEIGHT / 2.0);

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(SHOOTER_WIDTH, SHOOTER_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(platform_x, spawn_y, SHOOTER_Z),
        Enemy,
        ShooterEnemy,
        ShootTimer {
            timer: Timer::from_seconds(SHOOTER_FIRE_INTERVAL_MAX, TimerMode::Repeating),
        },
    ));
}

/// Spawn a charging enemy on a platform.
pub fn spawn_charging_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
    platform_width: f32,
    image: Handle<Image>,
) {
    let half_plat = platform_width / 2.0;
    let enemy_half = CHARGING_ENEMY_WIDTH / 2.0;
    let left = platform_x - half_plat + enemy_half;
    let right = platform_x + half_plat - enemy_half;
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (CHARGING_ENEMY_HEIGHT / 2.0);

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(CHARGING_ENEMY_WIDTH, CHARGING_ENEMY_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(platform_x, spawn_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: left,
            right_bound: right,
            direction: 1.0,
        },
        ChargingEnemy {
            state: ChargeState::Idle,
            charge_direction: 1.0,
            state_timer: Timer::from_seconds(0.0, TimerMode::Once),
        },
    ));
}

/// Spawn a flying ranged enemy that hovers and fires downward.
pub fn spawn_flying_ranged_enemy(commands: &mut Commands, x: f32, y: f32, image: Handle<Image>) {
    let hover_y = y + 100.0;

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::splat(FLYING_ENEMY_SIZE)),
            ..default()
        },
        Transform::from_xyz(x, hover_y, FLYING_ENEMY_Z),
        Enemy,
        FlyingEnemy {
            base_y: hover_y,
            amplitude: FLYING_ENEMY_AMPLITUDE * 0.7,
            frequency: FLYING_ENEMY_FREQUENCY * 0.8,
        },
        FlyingRangedEnemy,
        ShootTimer {
            timer: Timer::from_seconds(FLYING_RANGED_FIRE_INTERVAL, TimerMode::Repeating),
        },
    ));
}

// ---------------------------------------------------------------------------
// AI Systems
// ---------------------------------------------------------------------------

/// Move walking enemies back and forth within their patrol bounds.
pub fn enemy_patrol(
    time: Res<Time>,
    difficulty: Res<Difficulty>,
    mut query: Query<
        (&mut Transform, &mut Patrol, &mut Sprite),
        (With<Enemy>, Without<FlyingEnemy>, Without<ShooterEnemy>, Without<ChargingEnemy>, Without<FlyingRangedEnemy>),
    >,
) {
    let d = difficulty.value;
    let speed = if d > 1.0 {
        let t = ((d - 1.0) / 1.5).min(1.0);
        ENEMY_SPEED + (EXTREME_ENEMY_SPEED - ENEMY_SPEED) * t
    } else {
        ENEMY_SPEED
    };
    for (mut transform, mut patrol, mut sprite) in &mut query {
        transform.translation.x += patrol.direction * speed * time.delta_secs();

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

/// Sine-wave oscillation for flying enemies (including flying ranged).
pub fn flying_enemy_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &FlyingEnemy)>,
) {
    let t = time.elapsed_secs();
    for (mut tf, flying) in &mut query {
        tf.translation.y = flying.base_y + (t * flying.frequency).sin() * flying.amplitude;
    }
}

/// Shooter fires projectiles toward the player at intervals.
pub fn shooter_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&GlobalTransform, &mut ShootTimer), (With<ShooterEnemy>, Without<FlyingRangedEnemy>)>,
    player_query: Query<&Transform, With<Player>>,
    difficulty: Res<Difficulty>,
    game_sprites: Res<GameSprites>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    let d = difficulty.value;
    let fire_interval = if d <= 1.0 {
        SHOOTER_FIRE_INTERVAL_MAX + (SHOOTER_FIRE_INTERVAL_MIN - SHOOTER_FIRE_INTERVAL_MAX) * d
    } else {
        let t = ((d - 1.0) / 1.5).min(1.0);
        SHOOTER_FIRE_INTERVAL_MIN + (SHOOTER_FIRE_INTERVAL_EXTREME - SHOOTER_FIRE_INTERVAL_MIN) * t
    };
    let proj_speed = if d <= 1.0 {
        PROJECTILE_SPEED_MIN + (PROJECTILE_SPEED_MAX - PROJECTILE_SPEED_MIN) * d
    } else {
        let t = ((d - 1.0) / 1.5).min(1.0);
        PROJECTILE_SPEED_MAX + (PROJECTILE_SPEED_EXTREME - PROJECTILE_SPEED_MAX) * t
    };

    for (shooter_gtf, mut shoot_timer) in &mut query {
        shoot_timer.timer.tick(time.delta());
        shoot_timer.timer.set_duration(std::time::Duration::from_secs_f32(fire_interval));

        if shoot_timer.timer.just_finished() {
            let shooter_pos = shooter_gtf.translation();

            let to_player = Vec2::new(
                player_tf.translation.x - shooter_pos.x,
                player_tf.translation.y - shooter_pos.y,
            );
            if to_player.length() > SHOOTER_RANGE {
                continue;
            }

            let dir = to_player.normalize_or_zero();

            commands.spawn((
                Sprite {
                    image: game_sprites.projectile.clone(),
                    custom_size: Some(Vec2::splat(PROJECTILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(shooter_pos.x, shooter_pos.y, PROJECTILE_Z),
                Projectile {
                    velocity: dir * proj_speed,
                    lifetime: Timer::from_seconds(PROJECTILE_LIFETIME, TimerMode::Once),
                },
            ));

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.shoot {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }
        }
    }
}

/// Charging enemy AI: patrol normally, detect player, wind up, charge, recover.
pub fn charging_enemy_behavior(
    time: Res<Time>,
    player_query: Query<&Transform, With<Player>>,
    mut query: Query<(&mut Transform, &mut Patrol, &mut ChargingEnemy, &mut Sprite), (With<Enemy>, Without<Player>)>,
) {
    let Ok(player_tf) = player_query.single() else { return };
    let dt = time.delta_secs();

    for (mut tf, mut patrol, mut charger, mut sprite) in &mut query {
        charger.state_timer.tick(time.delta());

        match charger.state {
            ChargeState::Idle => {
                tf.translation.x += patrol.direction * ENEMY_SPEED * dt;
                if tf.translation.x >= patrol.right_bound {
                    tf.translation.x = patrol.right_bound;
                    patrol.direction = -1.0;
                } else if tf.translation.x <= patrol.left_bound {
                    tf.translation.x = patrol.left_bound;
                    patrol.direction = 1.0;
                }
                sprite.flip_x = patrol.direction < 0.0;

                // Post-recovery cooldown: state_timer is set when leaving Recovering;
                // the spawn-time 0s Once timer finishes on the first tick so initial
                // detection is unaffected.
                if !charger.state_timer.is_finished() {
                    continue;
                }

                let dx = player_tf.translation.x - tf.translation.x;
                let dy = (player_tf.translation.y - tf.translation.y).abs();
                // Only wind up if there is meaningful room to charge toward the player
                // within the patrol bounds — otherwise the charge would be zero-length
                // (enemies have no physics, so they must stay on the platform).
                let room = if dx > 0.0 {
                    patrol.right_bound - tf.translation.x
                } else {
                    tf.translation.x - patrol.left_bound
                };
                if dx.abs() < CHARGING_DETECT_RANGE
                    && dy < CHARGING_ENEMY_HEIGHT * 2.0
                    && room > CHARGING_ENEMY_WIDTH
                {
                    charger.state = ChargeState::WindingUp;
                    charger.charge_direction = dx.signum();
                    charger.state_timer = Timer::from_seconds(CHARGING_WIND_TIME, TimerMode::Once);
                    sprite.color = Color::srgb(1.0, 0.5, 0.5);
                }
            }
            ChargeState::WindingUp => {
                if charger.state_timer.is_finished() {
                    charger.state = ChargeState::Charging;
                    charger.state_timer = Timer::from_seconds(CHARGING_DURATION, TimerMode::Once);
                    sprite.color = Color::srgb(1.0, 0.3, 0.3);
                }
            }
            ChargeState::Charging => {
                tf.translation.x += charger.charge_direction * CHARGING_ENEMY_SPEED * dt;
                sprite.flip_x = charger.charge_direction < 0.0;

                if tf.translation.x >= patrol.right_bound
                    || tf.translation.x <= patrol.left_bound
                    || charger.state_timer.is_finished()
                {
                    tf.translation.x = tf.translation.x.clamp(patrol.left_bound, patrol.right_bound);
                    charger.state = ChargeState::Recovering;
                    charger.state_timer = Timer::from_seconds(CHARGING_RECOVERY, TimerMode::Once);
                }
            }
            ChargeState::Recovering => {
                sprite.color = Color::WHITE;
                if charger.state_timer.is_finished() {
                    charger.state = ChargeState::Idle;
                    // Cooldown before re-detecting so it doesn't immediately wind up again.
                    charger.state_timer = Timer::from_seconds(CHARGING_COOLDOWN, TimerMode::Once);
                }
            }
        }
    }
}

/// Flying ranged enemy fires downward projectiles.
pub fn flying_ranged_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&GlobalTransform, &mut ShootTimer), With<FlyingRangedEnemy>>,
    game_sprites: Res<GameSprites>,
    audio_handles: Option<Res<AudioHandles>>,
    player_query: Query<&Transform, With<Player>>,
) {
    let Ok(player_tf) = player_query.single() else { return };

    for (gtf, mut timer) in &mut query {
        timer.timer.tick(time.delta());

        if timer.timer.just_finished() {
            let pos = gtf.translation();

            // Range gate (same as shooter_fire): don't fire from off-screen.
            let to_player = Vec2::new(
                player_tf.translation.x - pos.x,
                player_tf.translation.y - pos.y,
            );
            if to_player.length() > SHOOTER_RANGE {
                continue;
            }

            let dx = (player_tf.translation.x - pos.x).clamp(-50.0, 50.0);
            let dir = Vec2::new(dx, -1.0).normalize();

            commands.spawn((
                Sprite {
                    image: game_sprites.projectile.clone(),
                    custom_size: Some(Vec2::splat(PROJECTILE_SIZE)),
                    ..default()
                },
                Transform::from_xyz(pos.x, pos.y, PROJECTILE_Z),
                Projectile {
                    velocity: dir * FLYING_RANGED_PROJ_SPEED,
                    lifetime: Timer::from_seconds(PROJECTILE_LIFETIME, TimerMode::Once),
                },
            ));

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.shoot {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }
        }
    }
}

/// Move projectiles and despawn when lifetime expires.
pub fn projectile_update(
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
