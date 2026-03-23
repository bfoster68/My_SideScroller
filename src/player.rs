use bevy::prelude::*;

use crate::animation::{AnimationTimer, CurrentAnim, FacingDirection, PlayerAnimState, SpriteSheets};
use crate::checkpoint::CheckpointData;
use crate::constants::*;
use crate::enemies::Enemy;
use crate::hazards::{Lava, Saw, Spike};
use crate::health::{DeathTimer, Health, Invincible};
use crate::level::{Platform, PlatformSize, PlatformVelocity};
use crate::powerups::{Shield, SpeedBoost, TripleJump};
use crate::save::ResumeFromCheckpoint;
use crate::state::GameState;

/// System set for player movement — other modules can schedule `.after(PlayerMovementSet)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerMovementSet;

/// System set for the play-start reset — runs first so other OnEnter systems see correct position.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayResetSet;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Grounded {
    pub on_ground: bool,
    pub coyote_timer: f32,
}

/// Tracks jump count for double-jump support.
#[derive(Component)]
pub struct JumpCounter {
    pub jumps_remaining: u32,
}

/// Tracks whether the player is holding the jump button (for variable height).
#[derive(Component)]
pub struct JumpHeld(pub bool);

/// Score resource — global across the game session.
#[derive(Resource, Default)]
pub struct Score {
    pub value: u32,
}

/// Coins resource — global across the game session.
#[derive(Resource, Default)]
pub struct Coins {
    pub count: u32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<Coins>()
            .add_systems(
                Update,
                spawn_player_if_missing.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    player_input,
                    apply_gravity,
                    apply_velocity,
                    respawn_on_fall,
                )
                    .chain()
                    .in_set(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::GameOver), reset_player_on_game_over)
            .add_systems(OnEnter(GameState::Playing), (restore_player_visibility, reset_score_on_play).chain().in_set(PlayResetSet));
    }
}

fn spawn_player_if_missing(
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
    sheets: Option<Res<SpriteSheets>>,
) {
    if !query.is_empty() {
        return;
    }

    let Some(sheets) = sheets else { return };

    commands
        .spawn((
            Sprite {
                image: sheets.idle.image.clone(),
                custom_size: Some(Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT)),
                texture_atlas: Some(TextureAtlas {
                    layout: sheets.idle.layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            Transform::from_xyz(SPAWN_X, SPAWN_Y, 1.0),
            Player,
            Velocity(Vec2::ZERO),
            Grounded {
                on_ground: true,
                coyote_timer: 0.0,
            },
            JumpCounter {
                jumps_remaining: MAX_JUMPS,
            },
            JumpHeld(false),
            Health::default(),
            PlayerAnimState::default(),
            FacingDirection::default(),
        ))
        .insert((
            CurrentAnim::default(),
            AnimationTimer {
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            },
        ));
}

fn player_input(
    game_input: Res<crate::input::GameInput>,
    time: Res<Time>,
    mut query: Query<
        (
            &mut Velocity,
            &mut Grounded,
            &mut JumpCounter,
            &mut JumpHeld,
            Option<&DeathTimer>,
            Option<&SpeedBoost>,
            Option<&TripleJump>,
            Option<&crate::health::Knockback>,
        ),
        With<Player>,
    >,
) {
    let Ok((
        mut velocity,
        mut grounded,
        mut jump_counter,
        mut jump_held,
        death_timer,
        speed_boost,
        triple_jump,
        knockback,
    )) = query.single_mut()
    else {
        return;
    };

    // No input during death animation
    if death_timer.is_some() {
        velocity.0.x = 0.0;
        return;
    }

    // Determine speed multiplier from active power-ups
    let speed_mult = speed_boost.map_or(1.0, |b| b.multiplier);

    // Reduce input control during knockback
    let input_mult = if knockback.is_some() { 0.3 } else { 1.0 };

    // Horizontal movement (supports analog from gamepad)
    velocity.0.x = game_input.move_x * PLAYER_SPEED * speed_mult * input_mult;

    // Determine max jumps (triple jump power-up)
    let max_jumps = if triple_jump.is_some() {
        TRIPLE_JUMP_MAX
    } else {
        MAX_JUMPS
    };

    // Coyote time — grace period after leaving a platform
    if grounded.on_ground {
        grounded.coyote_timer = COYOTE_TIME;
        jump_counter.jumps_remaining = max_jumps;
    } else {
        grounded.coyote_timer -= time.delta_secs();
    }

    let can_jump =
        grounded.on_ground || grounded.coyote_timer > 0.0 || jump_counter.jumps_remaining > 0;

    // Jump initiation
    if can_jump && game_input.jump_pressed {
        velocity.0.y = JUMP_FORCE;
        grounded.coyote_timer = 0.0;
        grounded.on_ground = false;
        jump_held.0 = true;

        // Consume a jump
        jump_counter.jumps_remaining = jump_counter.jumps_remaining.saturating_sub(1);
    }

    // Variable jump height — release early for a short hop
    if game_input.jump_released && jump_held.0 {
        jump_held.0 = false;
        if velocity.0.y > JUMP_FORCE_MIN {
            velocity.0.y = JUMP_FORCE_MIN;
        }
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut query: Query<(&mut Velocity, &Grounded, Option<&DeathTimer>), With<Player>>,
) {
    let Ok((mut velocity, grounded, death)) = query.single_mut() else {
        return;
    };
    if death.is_some() {
        return;
    }

    if !grounded.on_ground {
        velocity.0.y += GRAVITY * time.delta_secs();
    }
}

fn apply_velocity(
    time: Res<Time>,
    mut player_query: Query<
        (&mut Transform, &mut Velocity, &mut Grounded),
        With<Player>,
    >,
    platform_query: Query<
        (&Transform, &PlatformSize, Option<&PlatformVelocity>),
        (With<Platform>, Without<Player>),
    >,
) {
    let Ok((mut transform, mut velocity, mut grounded)) = player_query.single_mut() else {
        return;
    };

    // --- Horizontal pass ---
    transform.translation.x += velocity.0.x * time.delta_secs();

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (plat_tf, plat_size, _) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w)
            - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h)
            - (transform.translation.y - plat_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 && overlap_x < overlap_y {
            if transform.translation.x < plat_tf.translation.x {
                transform.translation.x =
                    plat_tf.translation.x - plat_half_w - player_half_w;
            } else {
                transform.translation.x =
                    plat_tf.translation.x + plat_half_w + player_half_w;
            }
            velocity.0.x = 0.0;
        }
    }

    // --- Vertical pass ---
    transform.translation.y += velocity.0.y * time.delta_secs();

    grounded.on_ground = false;
    let mut riding_delta_y = 0.0_f32;

    for (plat_tf, plat_size, plat_vel) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w)
            - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h)
            - (transform.translation.y - plat_tf.translation.y).abs();

        // Use a small epsilon so the player stays grounded when sitting
        // exactly on top of a platform (overlap_y == 0.0 after snap).
        if overlap_x > 0.0 && overlap_y >= -0.5 {
            if overlap_y <= 0.0 && transform.translation.y > plat_tf.translation.y {
                // Resting exactly on top — just mark grounded, no position correction.
                grounded.on_ground = true;
                if let Some(pv) = plat_vel {
                    riding_delta_y = pv.0.y;
                }
            } else if overlap_y > 0.0 && transform.translation.y > plat_tf.translation.y {
                // Landing on top — snap already places player at platform's
                // current position, so no riding delta needed here.
                transform.translation.y =
                    plat_tf.translation.y + plat_half_h + player_half_h;
                velocity.0.y = 0.0;
                grounded.on_ground = true;
            } else if overlap_y > 0.0 {
                // Bonking head on bottom
                transform.translation.y =
                    plat_tf.translation.y - plat_half_h - player_half_h;
                velocity.0.y = 0.0;
            }
        }
    }

    // Carry the player along with the moving platform
    if grounded.on_ground && riding_delta_y.abs() > 0.0 {
        transform.translation.y += riding_delta_y;
    }
}

fn respawn_on_fall(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Transform, &mut Velocity, &mut Grounded, &mut JumpCounter, Option<&DeathTimer>),
        With<Player>,
    >,
    camera_query: Query<&Transform, (With<Camera2d>, Without<Player>)>,
    platform_query: Query<
        (&Transform, &PlatformSize),
        (With<Platform>, Without<Player>, Without<Camera2d>),
    >,
    enemy_query: Query<&GlobalTransform, (With<Enemy>, Without<Player>, Without<Camera2d>, Without<Platform>)>,
    spike_query: Query<&GlobalTransform, (With<Spike>, Without<Player>, Without<Camera2d>, Without<Platform>)>,
    saw_query: Query<&GlobalTransform, (With<Saw>, Without<Player>, Without<Camera2d>, Without<Platform>)>,
    lava_query: Query<&Transform, (With<Lava>, Without<Player>, Without<Camera2d>, Without<Platform>)>,
) {
    let Ok((entity, mut transform, mut velocity, mut grounded, mut jump_counter, death)) =
        query.single_mut()
    else {
        return;
    };

    // Don't respawn during death animation
    if death.is_some() {
        return;
    }

    // Trigger respawn either when hitting the hard limit OR when
    // the player is falling and there is no platform beneath them.
    let needs_respawn = if transform.translation.y < FALL_LIMIT {
        true
    } else if !grounded.on_ground && velocity.0.y < 0.0 {
        // Check if ANY platform exists below the player within reach
        let player_x = transform.translation.x;
        let player_y = transform.translation.y;
        let has_platform_below = platform_query.iter().any(|(plat_tf, plat_size)| {
            let plat_half_w = plat_size.0.x / 2.0;
            let plat_top = plat_tf.translation.y + plat_size.0.y / 2.0;
            // Platform is below the player and horizontally reachable
            plat_top < player_y
                && plat_top > FALL_LIMIT
                && player_x > plat_tf.translation.x - plat_half_w - PLAYER_WIDTH
                && player_x < plat_tf.translation.x + plat_half_w + PLAYER_WIDTH
        });
        // Also check if lava is below — if so, let the player fall into it
        let has_lava_below = lava_query.iter().any(|lava_tf| {
            let lava_half_w = GROUND_SEGMENT_WIDTH / 2.0;
            lava_tf.translation.y < player_y
                && player_x > lava_tf.translation.x - lava_half_w
                && player_x < lava_tf.translation.x + lava_half_w
        });
        !has_platform_below && !has_lava_below
    } else {
        false
    };

    if needs_respawn {
        let camera_x = camera_query
            .single()
            .map(|c| c.translation.x)
            .unwrap_or(SPAWN_X);

        // Find the nearest safe platform, preferring platforms AHEAD of the
        // player so they don't get stuck in a backward-respawn loop.
        let player_x = transform.translation.x;
        let mut candidates: Vec<(f32, f32, f32)> = Vec::new(); // (score, x, top_y)
        for (plat_tf, plat_size) in &platform_query {
            let px = plat_tf.translation.x;
            let top_y = plat_tf.translation.y + plat_size.0.y / 2.0;
            let raw_dist = (px - player_x).abs();
            let score = if px >= player_x - 100.0 { raw_dist } else { raw_dist + 5000.0 };
            candidates.push((score, px, top_y));
        }
        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        let danger_radius = PLAYER_WIDTH + ENEMY_WIDTH;
        let mut best_platform: Option<(f32, f32)> = None;
        for (_dist, plat_x, top_y) in &candidates {
            // Check hazards via iterators (no Vec allocation)
            let is_safe = !enemy_query.iter()
                .map(|t| t.translation().truncate())
                .chain(spike_query.iter().map(|t| t.translation().truncate()))
                .chain(saw_query.iter().map(|t| t.translation().truncate()))
                .any(|h| {
                    (h.x - plat_x).abs() < danger_radius
                        && (h.y - top_y).abs() < ENEMY_HEIGHT + SPIKE_HEIGHT
                });
            if is_safe {
                best_platform = Some((*plat_x, *top_y));
                break;
            }
        }

        // If no safe platform, fall back to the closest one anyway
        if best_platform.is_none() {
            if let Some((_dist, plat_x, top_y)) = candidates.first() {
                best_platform = Some((*plat_x, *top_y));
            }
        }

        if let Some((plat_x, top_y)) = best_platform {
            transform.translation.x = plat_x;
            transform.translation.y = top_y + PLAYER_HEIGHT / 2.0 + 1.0;
        } else {
            // Fallback if no platforms exist
            transform.translation.x = camera_x;
            transform.translation.y = SPAWN_Y;
        }

        velocity.0 = Vec2::ZERO;
        grounded.on_ground = true;
        grounded.coyote_timer = 0.0;
        jump_counter.jumps_remaining = MAX_JUMPS;

        // Grant brief invincibility after respawn as a safety net
        commands.entity(entity).insert(Invincible {
            timer: Timer::from_seconds(INVINCIBILITY_DURATION, TimerMode::Once),
        });

        // Snap camera to new position
        commands.insert_resource(crate::camera::NeedsCameraSnap);
    }
}

fn reset_player_on_game_over(
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Health, &mut Sprite, &mut PlayerAnimState, &mut CurrentAnim),
        With<Player>,
    >,
) {
    let Ok((entity, mut health, mut sprite, mut anim_state, mut current_anim)) =
        query.single_mut()
    else {
        return;
    };
    // Only reset health here so the player stops taking damage.
    // Everything else resets on OnEnter(Playing) so the level is ready.
    health.current = health.max;

    // Reset animation from Death back to Idle
    *anim_state = PlayerAnimState::Idle;
    current_anim.0 = PlayerAnimState::Death; // Force swap_sprite_sheet to detect a change

    // Clean up death animation and power-up state
    commands
        .entity(entity)
        .remove::<DeathTimer>()
        .remove::<SpeedBoost>()
        .remove::<TripleJump>()
        .remove::<Shield>();
    // Keep player hidden during GameOver — restored in reset_score_on_play
    sprite.color = sprite.color.with_alpha(0.0);
}

/// Restore player sprite visibility when entering Playing (hidden during GameOver).
fn restore_player_visibility(mut query: Query<&mut Sprite, With<Player>>) {
    if let Ok(mut sprite) = query.single_mut() {
        sprite.color = sprite.color.with_alpha(1.0);
    }
}

/// Reset score, coins, and player position when starting a new game.
/// If ResumeFromCheckpoint is present, restore from checkpoint instead of resetting.
/// Level cleanup (chunk_tracker, difficulty, entity despawn) is handled
/// by reset_level_if_needed in level.rs.
fn reset_score_on_play(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut coins: ResMut<Coins>,
    checkpoint: Res<CheckpointData>,
    resume: Option<Res<ResumeFromCheckpoint>>,
    prev_state: Res<crate::state::PreviousGameState>,
    mut query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Grounded,
            &mut JumpCounter,
        ),
        With<Player>,
    >,
) {
    // Coming back from Pause or Settings — nothing to reset.
    if matches!(
        prev_state.0,
        Some(crate::state::GameState::Paused) | Some(crate::state::GameState::Settings)
    ) {
        return;
    }

    if resume.is_some() {
        // Resume from checkpoint — restore score and position
        score.value = checkpoint.last_checkpoint_score;
        coins.count = 0;

        if let Ok((mut transform, mut velocity, mut grounded, mut jump_counter)) =
            query.single_mut()
        {
            transform.translation.x = checkpoint.checkpoint_x;
            transform.translation.y = checkpoint.checkpoint_y;
            velocity.0 = Vec2::ZERO;
            grounded.on_ground = true;
            grounded.coyote_timer = 0.0;
            jump_counter.jumps_remaining = MAX_JUMPS;
        }

        // Consume the resume marker
        commands.remove_resource::<ResumeFromCheckpoint>();
    } else {
        // Fresh start
        score.value = 0;
        coins.count = 0;

        if let Ok((mut transform, mut velocity, mut grounded, mut jump_counter)) =
            query.single_mut()
        {
            transform.translation.x = SPAWN_X;
            transform.translation.y = SPAWN_Y;
            velocity.0 = Vec2::ZERO;
            grounded.on_ground = true;
            grounded.coyote_timer = 0.0;
            jump_counter.jumps_remaining = MAX_JUMPS;
        }
    }
}
