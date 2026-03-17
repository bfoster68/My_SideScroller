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

/// Boulder spawner — invisible entity that periodically drops boulders.
#[derive(Component)]
pub struct BoulderSpawner {
    pub timer: Timer,
    pub x: f32,
    pub boulder_image: Handle<Image>,
    pub warning_image: Handle<Image>,
}

/// Falling boulder — drops from above, damages on contact.
#[derive(Component)]
pub struct FallingBoulder {
    pub velocity_y: f32,
}

/// Warning indicator that appears before a boulder drops.
#[derive(Component)]
pub struct BoulderWarning {
    pub timer: Timer,
    pub x: f32,
    pub boulder_image: Handle<Image>,
}

/// Timed trap — spikes that appear and disappear on a cycle.
#[derive(Component)]
pub struct TimedTrap {
    pub timer: Timer,
    pub active: bool,
    pub on_duration: f32,
    pub off_duration: f32,
    #[allow(dead_code)]
    pub phase_offset: f32,
}

pub struct HazardsPlugin;

impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                saw_movement,
                saw_animate,
                lava_animate,
                boulder_spawner_system,
                boulder_warning_system,
                boulder_movement,
                timed_trap_system,
                spike_player_collision,
                saw_player_collision,
                lava_player_collision,
                boulder_player_collision,
                timed_trap_player_collision,
            )
                .after(PlayerMovementSet)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

// ---------------------------------------------------------------------------
// Spike
// ---------------------------------------------------------------------------

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

fn saw_animate(time: Res<Time>, mut query: Query<&mut Transform, With<Saw>>) {
    let dt = time.delta_secs();
    for mut tf in &mut query {
        tf.rotation *= Quat::from_rotation_z(-6.0 * dt);
    }
}

// ---------------------------------------------------------------------------
// Lava
// ---------------------------------------------------------------------------

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

fn lava_animate(time: Res<Time>, mut query: Query<&mut Sprite, With<Lava>>) {
    let t = time.elapsed_secs();
    let pulse = 0.85 + 0.15 * (t * 3.0).sin();
    for mut sprite in &mut query {
        sprite.color = Color::srgba(1.0, pulse, pulse, 1.0);
    }
}

// ---------------------------------------------------------------------------
// Falling Boulders
// ---------------------------------------------------------------------------

/// Spawn a boulder spawner entity at a platform position.
pub fn spawn_boulder_spawner(
    commands: &mut Commands,
    platform_x: f32,
    _platform_y: f32,
    boulder_image: Handle<Image>,
    warning_image: Handle<Image>,
) {
    commands.spawn((
        // Invisible entity — just a spawner marker
        Transform::from_xyz(platform_x, 0.0, 0.0),
        Visibility::Hidden,
        BoulderSpawner {
            timer: Timer::from_seconds(BOULDER_SPAWN_INTERVAL, TimerMode::Repeating),
            x: platform_x,
            boulder_image,
            warning_image,
        },
    ));
}

/// Boulder spawner ticks and creates warning indicators.
fn boulder_spawner_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<&mut BoulderSpawner>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    let Ok(camera_tf) = camera_query.single() else { return };
    let cam_x = camera_tf.translation.x;

    for mut spawner in &mut query {
        // Only spawn boulders for spawners near the camera
        if (spawner.x - cam_x).abs() > GENERATE_AHEAD {
            continue;
        }

        spawner.timer.tick(time.delta());

        if spawner.timer.just_finished() {
            // Spawn warning indicator first
            let warning_y = GROUND_Y + GROUND_HEIGHT;
            commands.spawn((
                Sprite {
                    image: spawner.warning_image.clone(),
                    custom_size: Some(Vec2::new(16.0, 16.0)),
                    color: Color::srgb(1.0, 0.3, 0.3),
                    ..default()
                },
                Transform::from_xyz(spawner.x, warning_y, BOULDER_Z + 0.1),
                BoulderWarning {
                    timer: Timer::from_seconds(BOULDER_WARNING_TIME, TimerMode::Once),
                    x: spawner.x,
                    boulder_image: spawner.boulder_image.clone(),
                },
            ));
        }
    }
}

/// Warning indicator counts down, then spawns the actual boulder.
fn boulder_warning_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut BoulderWarning, &mut Sprite)>,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    let Ok(camera_tf) = camera_query.single() else { return };
    let cam_y = camera_tf.translation.y;

    for (entity, mut warning, mut sprite) in &mut query {
        warning.timer.tick(time.delta());

        // Flash the warning indicator
        let t = warning.timer.elapsed_secs() / BOULDER_WARNING_TIME;
        let flash = if (t * 8.0) as u32 % 2 == 0 { 1.0 } else { 0.5 };
        sprite.color = Color::srgba(1.0, 0.3, 0.3, flash);

        if warning.timer.is_finished() {
            // Spawn the boulder above the camera
            let spawn_y = cam_y + 500.0;
            commands.spawn((
                Sprite {
                    image: warning.boulder_image.clone(),
                    custom_size: Some(Vec2::splat(BOULDER_SIZE)),
                    ..default()
                },
                Transform::from_xyz(warning.x, spawn_y, BOULDER_Z),
                FallingBoulder { velocity_y: 0.0 },
            ));

            commands.entity(entity).despawn();
        }
    }
}

/// Apply gravity to falling boulders and despawn when below fall limit.
fn boulder_movement(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut FallingBoulder)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tf, mut boulder) in &mut query {
        boulder.velocity_y += BOULDER_GRAVITY * dt;
        tf.translation.y += boulder.velocity_y * dt;

        // Rotate for visual effect
        tf.rotation *= Quat::from_rotation_z(-3.0 * dt);

        if tf.translation.y < FALL_LIMIT - 100.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Boulder–player collision.
fn boulder_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    boulder_query: Query<(Entity, &Transform), With<FallingBoulder>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else { return };
    if invincible.is_some() { return; }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let boulder_half = BOULDER_SIZE / 2.0;

    for (entity, boulder_tf) in &boulder_query {
        let overlap_x = (player_half_w + boulder_half)
            - (player_tf.translation.x - boulder_tf.translation.x).abs();
        let overlap_y = (player_half_h + boulder_half)
            - (player_tf.translation.y - boulder_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            damage_events.write(DamageEvent { amount: BOULDER_DAMAGE });
            commands.entity(entity).despawn();

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Timed Traps (Disappearing Spikes)
// ---------------------------------------------------------------------------

/// Spawn a timed trap on a platform (replaces some spikes).
pub fn spawn_timed_trap(commands: &mut Commands, x: f32, platform_y: f32, image: Handle<Image>) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SPIKE_HEIGHT / 2.0);
    // Random phase offset to desynchronize traps
    let phase = rand::random::<f32>() * (TIMED_TRAP_ON_DURATION + TIMED_TRAP_OFF_DURATION);

    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(SPIKE_WIDTH, SPIKE_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(x, spawn_y, SPIKE_Z),
        TimedTrap {
            timer: Timer::from_seconds(
                if phase < TIMED_TRAP_ON_DURATION { TIMED_TRAP_ON_DURATION - phase } else { TIMED_TRAP_OFF_DURATION - (phase - TIMED_TRAP_ON_DURATION) },
                TimerMode::Once,
            ),
            active: phase < TIMED_TRAP_ON_DURATION,
            on_duration: TIMED_TRAP_ON_DURATION,
            off_duration: TIMED_TRAP_OFF_DURATION,
            phase_offset: phase,
        },
    ));
}

/// Cycle timed traps between active/inactive with visual feedback.
fn timed_trap_system(
    time: Res<Time>,
    mut query: Query<(&mut TimedTrap, &mut Sprite, &mut Visibility)>,
) {
    for (mut trap, mut sprite, mut vis) in &mut query {
        trap.timer.tick(time.delta());

        if trap.active {
            // Active — visible, check if about to deactivate
            *vis = Visibility::Inherited;
            let remaining = trap.timer.remaining_secs();
            if remaining < TIMED_TRAP_WARNING {
                // Flash before deactivating
                let flash = if (remaining * 12.0) as u32 % 2 == 0 { 1.0 } else { 0.4 };
                sprite.color = Color::srgba(1.0, 1.0, 1.0, flash);
            } else {
                sprite.color = Color::WHITE;
            }

            if trap.timer.is_finished() {
                trap.active = false;
                trap.timer = Timer::from_seconds(trap.off_duration, TimerMode::Once);
                *vis = Visibility::Hidden;
            }
        } else {
            // Inactive — hidden, check if about to activate
            let remaining = trap.timer.remaining_secs();
            if remaining < TIMED_TRAP_WARNING {
                // Show faintly as warning before becoming active
                *vis = Visibility::Inherited;
                let flash = if (remaining * 8.0) as u32 % 2 == 0 { 0.3 } else { 0.0 };
                sprite.color = Color::srgba(1.0, 0.5, 0.5, flash);
            }

            if trap.timer.is_finished() {
                trap.active = true;
                trap.timer = Timer::from_seconds(trap.on_duration, TimerMode::Once);
                *vis = Visibility::Inherited;
                sprite.color = Color::WHITE;
            }
        }
    }
}

/// Timed trap–player collision (only when active).
fn timed_trap_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    trap_query: Query<(&GlobalTransform, &TimedTrap)>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else { return };
    if invincible.is_some() { return; }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let spike_half_w = SPIKE_WIDTH / 2.0;
    let spike_half_h = SPIKE_HEIGHT / 2.0;

    for (trap_gtf, trap) in &trap_query {
        if !trap.active { continue; }

        let pos = trap_gtf.translation();
        let overlap_x = (player_half_w + spike_half_w)
            - (player_tf.translation.x - pos.x).abs();
        let overlap_y = (player_half_h + spike_half_h)
            - (player_tf.translation.y - pos.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            damage_events.write(DamageEvent { amount: TIMED_TRAP_DAMAGE });

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Existing collision systems
// ---------------------------------------------------------------------------

fn spike_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    spike_query: Query<&GlobalTransform, With<Spike>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else { return };
    if invincible.is_some() { return; }

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

fn saw_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    saw_query: Query<&GlobalTransform, With<Saw>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else { return };
    if invincible.is_some() { return; }

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

fn lava_player_collision(
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    lava_query: Query<(&Transform, &Sprite), (With<Lava>, Without<Player>)>,
    mut damage_events: MessageWriter<DamageEvent>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else { return };
    if invincible.is_some() { return; }

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
