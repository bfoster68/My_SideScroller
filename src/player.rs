use bevy::prelude::*;

use crate::animation::{AnimationTimer, CurrentAnim, FacingDirection, PlayerAnimState, SpriteSheets};
use crate::checkpoint::CheckpointData;
use crate::constants::*;
use crate::enemies::Enemy;
use crate::hazards::{Lava, Saw, Spike};
use crate::health::{DeathTimer, Health, Invincible};
use crate::input::GameInput;
use crate::level::{
    ConveyorPlatform, IcePlatform, OneWayPlatform, Platform, PlatformSize, PlatformVelocity,
    SpringPlatform,
};
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
    /// Seconds remaining on a buffered Jump press (fires on the next landing).
    pub jump_buffer: f32,
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
            .init_resource::<StandingOnPlatform>()
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
                jump_buffer: 0.0,
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
    standing: Res<StandingOnPlatform>,
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
    // On ice: blend toward target speed slowly (sliding)
    let target_vx = game_input.move_x * PLAYER_SPEED * speed_mult * input_mult;
    if standing.on_ice && grounded.on_ground {
        let blend = ICE_FRICTION_MULTIPLIER * 3.0 * time.delta_secs();
        velocity.0.x = velocity.0.x + (target_vx - velocity.0.x) * blend.min(1.0);
    } else if knockback.is_some() {
        // Preserve the knockback impulse: ease toward the (dampened) input
        // instead of overwriting the pushback every frame, otherwise the hit
        // has no horizontal effect and the player re-touches the enemy.
        let blend = 6.0 * time.delta_secs();
        velocity.0.x += (target_vx - velocity.0.x) * blend.min(1.0);
    } else {
        velocity.0.x = target_vx;
    }

    // Determine max jumps (triple jump power-up)
    let max_jumps = if triple_jump.is_some() {
        TRIPLE_JUMP_MAX
    } else {
        MAX_JUMPS
    };
    // If a triple-jump power-up expired mid-air, don't keep the extra jump.
    jump_counter.jumps_remaining = jump_counter.jumps_remaining.min(max_jumps);

    // Jump buffering — the mirror of coyote time: remember a Jump press for a
    // short window so pressing a few frames before landing still jumps on
    // touchdown instead of being silently dropped. Holding Down doesn't
    // buffer (Down+Jump is the drop-through gesture).
    if game_input.jump_pressed && !game_input.down_held {
        grounded.jump_buffer = JUMP_BUFFER_TIME;
    } else {
        grounded.jump_buffer = (grounded.jump_buffer - time.delta_secs()).max(0.0);
    }

    // Coyote time — grace period after leaving a platform. This runs BEFORE
    // the spring check so that landing on a spring still refills your jumps.
    if grounded.on_ground {
        grounded.coyote_timer = COYOTE_TIME;
        jump_counter.jumps_remaining = max_jumps;
    } else {
        grounded.coyote_timer -= time.delta_secs();
        // Walked off a ledge without jumping: that counts as the first jump,
        // so coyote time can't grant a bonus jump on top of the double jump.
        if jump_counter.jumps_remaining == max_jumps {
            jump_counter.jumps_remaining = max_jumps.saturating_sub(1);
        }
    }

    // Spring bounce: auto-launch when landing on a spring platform. It's a
    // free bounce: it doesn't consume a jump, and it must NOT arm the
    // short-hop cut (that only applies to real button-driven jumps), or
    // releasing Jump afterwards would clip the bounce to a short hop.
    if standing.on_spring && grounded.on_ground && velocity.0.y <= 0.0 {
        velocity.0.y = JUMP_FORCE * standing.spring_multiplier;
        grounded.on_ground = false;
        jump_held.0 = false;
    }

    let can_jump =
        grounded.on_ground || grounded.coyote_timer > 0.0 || jump_counter.jumps_remaining > 0;

    // Jump initiation, driven by the buffer so an early press counts on
    // landing. Holding Down suppresses the jump so that Down+Jump can drop
    // through a one-way platform instead of launching upward.
    if can_jump && grounded.jump_buffer > 0.0 && !game_input.down_held {
        grounded.jump_buffer = 0.0; // consumed
        let is_ground_jump = grounded.on_ground || grounded.coyote_timer > 0.0;

        if is_ground_jump {
            // First jump: full force. Set the remaining count absolutely so a
            // coyote jump (where leaving the ledge already charged one jump)
            // still leaves the full set of air jumps — otherwise walking off
            // a ledge and jumping ate the double jump.
            velocity.0.y = JUMP_FORCE;
            jump_counter.jumps_remaining = max_jumps.saturating_sub(1);
        } else {
            // Air jump: momentum-based — stronger if used while still rising,
            // weaker if used while falling. Rewards good timing.
            let vy = velocity.0.y;
            // Map current velocity to a ratio: rising = max ratio, falling fast = min ratio
            // vy ranges roughly from JUMP_FORCE (just jumped) to -JUMP_FORCE (terminal fall)
            let t = ((vy / JUMP_FORCE) + 1.0).clamp(0.0, 1.0) * 0.5; // 0.0 (falling) to 0.5 (peak) to 1.0 (rising)
            let ratio = crate::constants::DOUBLE_JUMP_MIN_RATIO
                + t * (crate::constants::DOUBLE_JUMP_MAX_RATIO - crate::constants::DOUBLE_JUMP_MIN_RATIO);
            // Cancel downward momentum but don't boost upward momentum
            velocity.0.y = (JUMP_FORCE * ratio).max(0.0);
            // Consume one air jump
            jump_counter.jumps_remaining = jump_counter.jumps_remaining.saturating_sub(1);
        }

        grounded.coyote_timer = 0.0;
        grounded.on_ground = false;
        jump_held.0 = true;
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
        // Terminal velocity: without a cap a long fall moves far enough per
        // frame to tunnel through a platform or skip an enemy's stomp zone.
        velocity.0.y = velocity.0.y.max(-TERMINAL_VELOCITY);
    }
}

/// Tracks what kind of platform the player is standing on this frame.
#[derive(Resource, Default)]
pub struct StandingOnPlatform {
    pub on_ice: bool,
    pub conveyor_speed: f32,
    pub on_spring: bool,
    pub spring_multiplier: f32,
}

fn apply_velocity(
    time: Res<Time>,
    mut player_query: Query<
        (&mut Transform, &mut Velocity, &mut Grounded),
        With<Player>,
    >,
    platform_query: Query<
        (
            &Transform,
            &PlatformSize,
            Option<&PlatformVelocity>,
            Option<&OneWayPlatform>,
            Option<&IcePlatform>,
            Option<&ConveyorPlatform>,
            Option<&SpringPlatform>,
        ),
        (With<Platform>, Without<Player>),
    >,
    game_input: Res<GameInput>,
    mut standing: ResMut<StandingOnPlatform>,
) {
    let Ok((mut transform, mut velocity, mut grounded)) = player_query.single_mut() else {
        return;
    };

    // Reset standing-on info each frame
    *standing = StandingOnPlatform::default();

    // --- Horizontal pass ---
    transform.translation.x += velocity.0.x * time.delta_secs();

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (plat_tf, plat_size, _, one_way, _, _, _) in &platform_query {
        // One-way platforms have no horizontal collision
        if one_way.is_some() { continue; }

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

    // --- Vertical pass (swept) ---
    // Remember where the feet/head were BEFORE integrating so we can detect
    // crossing a surface this frame. Comparing only post-integration positions
    // meant a fast fall could skip the tiny landing window entirely (one-way
    // platforms had a 4px window, so they were nearly impossible to land on)
    // and a rising player could get snapped on TOP of a platform it was
    // passing through from below.
    let prev_bottom = transform.translation.y - player_half_h;
    let prev_top = transform.translation.y + player_half_h;
    transform.translation.y += velocity.0.y * time.delta_secs();

    grounded.on_ground = false;
    let mut riding_velocity = Vec2::ZERO;
    // Drop through a one-way platform by holding Down and pressing Jump.
    // (Uses the real down axis — the old check misread "moving left" as down.)
    let drop_through = game_input.down_held && game_input.jump_pressed;
    // Feet may sit slightly above the surface (a platform descending under
    // the player) or slightly below it (overshoot) and still count as "on it".
    const SURFACE_SLOP: f32 = 8.0;
    const GROUND_EPS: f32 = 6.0;

    for (plat_tf, plat_size, plat_vel, one_way, ice, conveyor, spring) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;
        let plat_top = plat_tf.translation.y + plat_half_h;
        let plat_bottom = plat_tf.translation.y - plat_half_h;

        let overlap_x = (player_half_w + plat_half_w)
            - (transform.translation.x - plat_tf.translation.x).abs();
        if overlap_x <= 0.0 {
            continue;
        }

        let bottom_now = transform.translation.y - player_half_h;
        let top_now = transform.translation.y + player_half_h;
        // Feet were at/above the surface last frame (slop covers a rising platform).
        let was_above = prev_bottom >= plat_top - SURFACE_SLOP;
        // Head was at/below the underside last frame.
        let was_below = prev_top <= plat_bottom + SURFACE_SLOP;

        // One-way platforms only catch you from above, and never while dropping through.
        if one_way.is_some() {
            if drop_through && was_above && bottom_now <= plat_top + GROUND_EPS {
                // Dropping through the platform we're standing on: push the
                // feet clearly below the surface so the swept check won't
                // re-catch us next frame, and make sure we head downward.
                transform.translation.y = plat_top - (SURFACE_SLOP + 1.0) + player_half_h;
                velocity.0.y = velocity.0.y.min(0.0);
                continue;
            }
            if !was_above || drop_through {
                continue;
            }
        }

        if velocity.0.y <= 0.0 && was_above && bottom_now <= plat_top + GROUND_EPS {
            // Landing / resting: always snap the feet to the surface and stop
            // falling. Snapping every frame is also what carries the player on a
            // moving platform (both rising and descending), so no extra vertical
            // "ride" offset is needed — adding one on top of the snap left the
            // player hovering a frame above a rising platform.
            transform.translation.y = plat_top + player_half_h;
            velocity.0.y = 0.0;
            grounded.on_ground = true;
            if let Some(pv) = plat_vel {
                riding_velocity = pv.0;
            }
            // Track special platform types
            if ice.is_some() { standing.on_ice = true; }
            if let Some(conv) = conveyor { standing.conveyor_speed = conv.speed; }
            if let Some(sp) = spring {
                standing.on_spring = true;
                standing.spring_multiplier = sp.force_multiplier;
            }
        } else if velocity.0.y > 0.0
            && was_below
            && top_now >= plat_bottom - GROUND_EPS
            && one_way.is_none()
        {
            // Head bonk on the underside of a solid platform (direction-gated so
            // a fast riser can't be resolved as a landing on top).
            transform.translation.y = plat_bottom - player_half_h;
            velocity.0.y = 0.0;
        }
    }

    // Carry the player horizontally with a moving platform / conveyor.
    // Vertical carry is handled by the landing snap above.
    if grounded.on_ground {
        let dt = time.delta_secs();
        if riding_velocity.x.abs() > 0.0 {
            transform.translation.x += riding_velocity.x * dt;
        }
        // Apply conveyor push
        if standing.conveyor_speed.abs() > 0.0 {
            transform.translation.x += standing.conveyor_speed * dt;
        }
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
    } else if !grounded.on_ground && velocity.0.y < 0.0 && transform.translation.y < GROUND_Y {
        // Only check for missing platforms when below ground level
        // to avoid false respawns during normal jump arcs over gaps
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
        // Single-pass: track best safe and best overall (fallback).
        let player_x = transform.translation.x;
        let danger_x = PLAYER_WIDTH / 2.0 + ENEMY_WIDTH / 2.0 + 20.0;
        let danger_y = PLAYER_HEIGHT / 2.0 + ENEMY_HEIGHT / 2.0;

        // Collect hazard positions once (small vec, only nearby hazards)
        let hazards: Vec<Vec2> = enemy_query.iter()
            .map(|t| t.translation().truncate())
            .chain(spike_query.iter().map(|t| t.translation().truncate()))
            .chain(saw_query.iter().map(|t| t.translation().truncate()))
            .filter(|h| (h.x - player_x).abs() < GENERATE_AHEAD)
            .collect();

        let mut best_safe: Option<(f32, f32, f32)> = None; // (score, x, top_y)
        let mut best_any: Option<(f32, f32, f32)> = None;

        for (plat_tf, plat_size) in &platform_query {
            let px = plat_tf.translation.x;
            let top_y = plat_tf.translation.y + plat_size.0.y / 2.0;
            let raw_dist = (px - player_x).abs();
            let score = if px >= player_x - 100.0 { raw_dist } else { raw_dist + 5000.0 };

            if best_any.is_none() || score < best_any.unwrap().0 {
                best_any = Some((score, px, top_y));
            }

            if best_safe.is_none() || score < best_safe.unwrap().0 {
                let is_safe = !hazards.iter().any(|h| {
                    (h.x - px).abs() < danger_x && (h.y - top_y).abs() < danger_y
                });
                if is_safe {
                    best_safe = Some((score, px, top_y));
                }
            }
        }

        let best_platform = best_safe.or(best_any).map(|(_, x, y)| (x, y));

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

        // NOTE: the ResumeFromCheckpoint marker is deliberately NOT consumed
        // here. Other OnEnter(Playing) systems (checkpoint + level reset) run
        // after this one and also need to see it; removing it here made those
        // see a fresh start (regenerating terrain from spawn on every Load).
        // It's cleared on OnExit(Playing) in level.rs instead.
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
