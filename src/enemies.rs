use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::camera::HitFreeze;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::level::Difficulty;
use crate::particles::spawn_burst;
use crate::player::{Grounded, Player, PlayerMovementSet, Score, Velocity};
use crate::sprites::GameSprites;
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

/// Floating score text that rises and fades in world space.
#[derive(Component)]
pub struct ScorePopup {
    pub timer: Timer,
}

/// Tracks consecutive stomps without landing for combo multiplier.
#[derive(Resource, Default)]
pub struct ComboTracker {
    pub count: u32,
    pub display_timer: Timer,
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComboTracker>()
            .add_systems(
                Update,
                (
                    enemy_patrol,
                    flying_enemy_movement,
                    charging_enemy_behavior,
                    flying_ranged_fire,
                    shooter_fire,
                    projectile_update,
                    enemy_player_collision,
                    projectile_player_collision,
                    update_score_popups,
                    reset_combo_on_land,
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

/// Move walking enemies back and forth within their patrol bounds.
/// Excludes charging enemies (they have their own behavior system).
fn enemy_patrol(
    time: Res<Time>,
    mut query: Query<
        (&mut Transform, &mut Patrol, &mut Sprite),
        (With<Enemy>, Without<FlyingEnemy>, Without<ShooterEnemy>, Without<ChargingEnemy>, Without<FlyingRangedEnemy>),
    >,
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

/// Sine-wave oscillation for flying enemies (including flying ranged).
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

/// Shooter fires projectiles toward the player at intervals.
/// Fire rate and projectile speed scale with difficulty. Only fires when
/// the player is within SHOOTER_RANGE.
fn shooter_fire(
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
    // Fire interval decreases (faster) with difficulty
    let fire_interval = SHOOTER_FIRE_INTERVAL_MAX
        + (SHOOTER_FIRE_INTERVAL_MIN - SHOOTER_FIRE_INTERVAL_MAX) * d;
    // Projectile speed increases with difficulty
    let proj_speed = PROJECTILE_SPEED_MIN
        + (PROJECTILE_SPEED_MAX - PROJECTILE_SPEED_MIN) * d;

    for (shooter_gtf, mut shoot_timer) in &mut query {
        // Dynamically adjust fire rate based on current difficulty
        shoot_timer.timer.set_duration(std::time::Duration::from_secs_f32(fire_interval));
        shoot_timer.timer.tick(time.delta());

        if shoot_timer.timer.just_finished() {
            let shooter_pos = shooter_gtf.translation();

            // Only fire when player is within range
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

// ---------------------------------------------------------------------------
// Charging Enemy
// ---------------------------------------------------------------------------

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

/// Charging enemy AI: patrol normally, detect player, wind up, charge, recover.
fn charging_enemy_behavior(
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
                // Normal patrol
                tf.translation.x += patrol.direction * ENEMY_SPEED * dt;
                if tf.translation.x >= patrol.right_bound {
                    tf.translation.x = patrol.right_bound;
                    patrol.direction = -1.0;
                } else if tf.translation.x <= patrol.left_bound {
                    tf.translation.x = patrol.left_bound;
                    patrol.direction = 1.0;
                }
                sprite.flip_x = patrol.direction < 0.0;

                // Check for player in detect range
                let dx = player_tf.translation.x - tf.translation.x;
                let dy = (player_tf.translation.y - tf.translation.y).abs();
                if dx.abs() < CHARGING_DETECT_RANGE && dy < CHARGING_ENEMY_HEIGHT * 2.0 {
                    charger.state = ChargeState::WindingUp;
                    charger.charge_direction = dx.signum();
                    charger.state_timer = Timer::from_seconds(CHARGING_WIND_TIME, TimerMode::Once);
                    // Visual cue: tint red during wind-up
                    sprite.color = Color::srgb(1.0, 0.5, 0.5);
                }
            }
            ChargeState::WindingUp => {
                if charger.state_timer.is_finished() {
                    charger.state = ChargeState::Charging;
                    charger.state_timer = Timer::from_seconds(CHARGING_DURATION, TimerMode::Once);
                    sprite.color = Color::srgb(1.0, 0.3, 0.3); // brighter red during charge
                }
            }
            ChargeState::Charging => {
                tf.translation.x += charger.charge_direction * CHARGING_ENEMY_SPEED * dt;
                sprite.flip_x = charger.charge_direction < 0.0;

                // Stop at platform edges
                if tf.translation.x >= patrol.right_bound {
                    tf.translation.x = patrol.right_bound;
                    charger.state = ChargeState::Recovering;
                    charger.state_timer = Timer::from_seconds(CHARGING_RECOVERY, TimerMode::Once);
                } else if tf.translation.x <= patrol.left_bound {
                    tf.translation.x = patrol.left_bound;
                    charger.state = ChargeState::Recovering;
                    charger.state_timer = Timer::from_seconds(CHARGING_RECOVERY, TimerMode::Once);
                }

                if charger.state_timer.is_finished() {
                    charger.state = ChargeState::Recovering;
                    charger.state_timer = Timer::from_seconds(CHARGING_RECOVERY, TimerMode::Once);
                }
            }
            ChargeState::Recovering => {
                // Pause, then return to idle
                sprite.color = Color::WHITE; // reset tint
                if charger.state_timer.is_finished() {
                    charger.state = ChargeState::Idle;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Flying Ranged Enemy
// ---------------------------------------------------------------------------

/// Spawn a flying ranged enemy that hovers and fires downward.
pub fn spawn_flying_ranged_enemy(commands: &mut Commands, x: f32, y: f32, image: Handle<Image>) {
    let hover_y = y + 100.0; // hovers higher than regular flying

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

/// Flying ranged enemy fires downward projectiles.
fn flying_ranged_fire(
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
            // Fire slightly toward the player but mostly downward
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
fn enemy_player_collision(
    mut commands: Commands,
    mut player_query: Query<
        (&Transform, &mut Velocity, Option<&Invincible>, Option<&crate::health::DeathTimer>),
        With<Player>,
    >,
    enemy_query: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<Projectile>)>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut score: ResMut<Score>,
    mut combo: ResMut<ComboTracker>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, mut player_vel, invincible, death)) = player_query.single_mut() else {
        return;
    };
    if death.is_some() {
        return;
    }

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
            let death_pos = Vec2::new(enemy_pos.x, enemy_pos.y);

            // Combo multiplier: 1x, 2x, 4x, 8x, 16x
            let multiplier = 2u32.pow(combo.count.min(MAX_COMBO_POWER));
            let kill_score = ENEMY_KILL_SCORE * multiplier;
            combo.count += 1;
            combo.display_timer = Timer::from_seconds(COMBO_DISPLAY_DURATION, TimerMode::Once);

            commands.entity(enemy_entity).despawn();
            player_vel.0.y = ENEMY_STOMP_BOUNCE;
            score.value += kill_score;

            // Death particles (orange/red burst)
            spawn_burst(
                &mut commands,
                death_pos,
                ENEMY_DEATH_PARTICLE_COUNT,
                Color::srgb(0.9, 0.4, 0.1),
                true,
            );

            // Floating score popup
            commands.spawn((
                ScorePopup {
                    timer: Timer::from_seconds(SCORE_POPUP_DURATION, TimerMode::Once),
                },
                Text2d::new(format!("+{}", kill_score)),
                TextFont { font_size: 20.0, ..default() },
                TextColor(Color::srgb(1.0, 1.0, 0.3)),
                Transform::from_xyz(death_pos.x, death_pos.y + 20.0, 5.0),
            ));

            // Hit freeze for stomp impact
            commands.insert_resource(HitFreeze {
                timer: Timer::from_seconds(STOMP_FREEZE_DURATION, TimerMode::Once),
                time_scale: STOMP_FREEZE_SCALE,
            });
        } else if invincible.is_none() {
            let enemy_pos_2d = Vec2::new(enemy_pos.x, enemy_pos.y);
            damage_events.write(DamageEvent {
                amount: 1,
                source_pos: Some(enemy_pos_2d),
            });
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            // Hit freeze for damage impact
            commands.insert_resource(HitFreeze {
                timer: Timer::from_seconds(DAMAGE_FREEZE_DURATION, TimerMode::Once),
                time_scale: DAMAGE_FREEZE_SCALE,
            });
            break;
        }
    }
}

/// Update floating score popups — rise, fade, and despawn.
fn update_score_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ScorePopup, &mut TextColor)>,
) {
    for (entity, mut tf, mut popup, mut color) in &mut query {
        popup.timer.tick(time.delta());
        tf.translation.y += SCORE_POPUP_RISE_SPEED * time.delta_secs();
        let alpha = popup.timer.fraction_remaining();
        color.0 = color.0.with_alpha(alpha);
        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Reset combo when player touches the ground.
fn reset_combo_on_land(
    mut combo: ResMut<ComboTracker>,
    player_query: Query<&Grounded, With<Player>>,
    time: Res<Time>,
) {
    if let Ok(grounded) = player_query.single() {
        if grounded.on_ground {
            combo.count = 0;
        }
    }
    combo.display_timer.tick(time.delta());
}

/// Projectile–player collision.
fn projectile_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>, Option<&crate::health::DeathTimer>), With<Player>>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible, death)) = player_query.single() else {
        return;
    };

    if death.is_some() || invincible.is_some() {
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
                source_pos: Some(Vec2::new(proj_tf.translation.x, proj_tf.translation.y)),
            });

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}
