use bevy::prelude::*;
use rand::Rng;

use crate::constants::*;
use crate::level::{Platform, PlatformSize};
use crate::particles::spawn_burst;
use crate::player::{Grounded, Player, PlayerMovementSet, Score, Velocity};
use crate::state::GameState;

/// Marks a block as part of a breakable platform.
#[derive(Component)]
pub struct BreakableBlock {
    pub health: i32,
    pub max_health: i32,
    #[allow(dead_code)]
    pub group_id: u32, // reserved for future falling-block physics
}

/// A debris fragment spawned when a block is destroyed.
/// Falls with gravity, tumbles, shrinks, and fades out.
#[derive(Component)]
struct Debris {
    velocity: Vec2,
    angular_velocity: f32,
    lifetime: Timer,
}

/// Counter for generating unique group IDs.
#[derive(Resource, Default)]
pub struct BreakableGroupCounter(pub u32);

pub struct BreakablePlugin;

impl Plugin for BreakablePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BreakableGroupCounter>()
            .add_systems(
                Update,
                (
                    block_stomp_damage
                        .after(PlayerMovementSet),
                    update_debris,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Spawn a breakable platform as a row of individual block entities.
/// Each block is a standard `Platform` entity so existing collision works automatically.
/// Returns the total width of the breakable platform.
pub fn spawn_breakable_platform(
    commands: &mut Commands,
    center_x: f32,
    y: f32,
    num_blocks: usize,
    group_id: u32,
) -> f32 {
    let total_width = num_blocks as f32 * BLOCK_WIDTH;
    let start_x = center_x - total_width / 2.0 + BLOCK_WIDTH / 2.0;

    let mut rng = rand::thread_rng();

    for i in 0..num_blocks {
        let block_x = start_x + i as f32 * BLOCK_WIDTH;

        // Randomize starting health — some blocks are already weakened
        let health = if rng.gen_bool(BLOCK_WEAK_CHANCE) {
            rng.gen_range(1..BLOCK_MAX_HEALTH) // 1 or 2 HP
        } else {
            BLOCK_MAX_HEALTH // full health
        };

        let color = block_color_for_health(health, BLOCK_MAX_HEALTH);

        // Sprite slightly smaller than collision box for visible seams
        let sprite_width = BLOCK_WIDTH - 1.0;

        commands.spawn((
            Sprite::from_color(color, Vec2::new(sprite_width, BLOCK_HEIGHT)),
            Transform::from_xyz(block_x, y, 0.0),
            Platform,
            PlatformSize(Vec2::new(BLOCK_WIDTH, BLOCK_HEIGHT)),
            BreakableBlock {
                health,
                max_health: BLOCK_MAX_HEALTH,
                group_id,
            },
        ));
    }

    total_width
}

/// Compute block color based on remaining health — brick → orange → red.
fn block_color_for_health(health: i32, max_health: i32) -> Color {
    let t = health as f32 / max_health as f32;
    Color::srgb(
        0.7 + 0.2 * (1.0 - t),
        0.55 * t + 0.3 * (1.0 - t),
        0.3 * t,
    )
}

/// Spawn debris fragments when a block is destroyed.
/// Creates rectangular chunks that fly outward, tumble, fall with gravity, and fade.
fn spawn_debris(commands: &mut Commands, block_pos: Vec2, block_color: Color) {
    let mut rng = rand::thread_rng();
    let num_debris = DEBRIS_COUNT;

    for _ in 0..num_debris {
        // Random size — rectangular chunks, not squares
        let w = rng.gen_range(DEBRIS_MIN_SIZE..DEBRIS_MAX_SIZE);
        let h = rng.gen_range(DEBRIS_MIN_SIZE..DEBRIS_MAX_SIZE);

        // Random position within the block area
        let offset_x = rng.gen_range(-BLOCK_WIDTH / 2.0..BLOCK_WIDTH / 2.0);
        let offset_y = rng.gen_range(-BLOCK_HEIGHT / 2.0..BLOCK_HEIGHT / 2.0);

        // Velocity: explode outward and upward
        let angle = rng.gen_range(0.3..std::f32::consts::PI - 0.3); // mostly upward arc
        let speed = rng.gen_range(DEBRIS_MIN_SPEED..DEBRIS_MAX_SPEED);
        let vx = angle.cos() * speed * if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
        let vy = angle.sin() * speed;

        // Random spin
        let angular_vel = rng.gen_range(-DEBRIS_MAX_SPIN..DEBRIS_MAX_SPIN);

        // Slight color variation per fragment
        let color_shift = rng.gen_range(-0.08..0.08);
        let frag_color = match block_color {
            Color::Srgba(c) => Color::srgb(
                (c.red + color_shift).clamp(0.0, 1.0),
                (c.green + color_shift * 0.5).clamp(0.0, 1.0),
                (c.blue + color_shift * 0.3).clamp(0.0, 1.0),
            ),
            _ => block_color,
        };

        let lifetime = rng.gen_range(DEBRIS_LIFETIME_MIN..DEBRIS_LIFETIME_MAX);

        commands.spawn((
            Sprite::from_color(frag_color, Vec2::new(w, h)),
            Transform::from_xyz(
                block_pos.x + offset_x,
                block_pos.y + offset_y,
                0.9, // above platforms
            ),
            Debris {
                velocity: Vec2::new(vx, vy),
                angular_velocity: angular_vel,
                lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
            },
        ));
    }
}

/// Update debris: apply gravity, rotation, fade, shrink, and despawn.
fn update_debris(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Debris, &mut Sprite)>,
) {
    let dt = time.delta_secs();

    for (entity, mut tf, mut debris, mut sprite) in &mut query {
        debris.lifetime.tick(time.delta());

        // Apply gravity
        debris.velocity.y += DEBRIS_GRAVITY * dt;

        // Move
        tf.translation.x += debris.velocity.x * dt;
        tf.translation.y += debris.velocity.y * dt;

        // Tumble (rotate around Z axis)
        tf.rotate_z(debris.angular_velocity * dt);

        // Fade and shrink based on lifetime
        let remaining = debris.lifetime.fraction_remaining();
        sprite.color = sprite.color.with_alpha(remaining);
        let scale = 0.3 + remaining * 0.7;
        tf.scale = Vec3::splat(scale);

        if debris.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Detect when player lands on breakable blocks and apply damage.
fn block_stomp_damage(
    mut commands: Commands,
    player_query: Query<(&Transform, &Velocity, &Grounded), With<Player>>,
    mut block_query: Query<
        (Entity, &Transform, &PlatformSize, &mut BreakableBlock, &mut Sprite),
        Without<Player>,
    >,
    mut score: ResMut<Score>,
    mut prev_grounded: Local<bool>,
) {
    let Ok((player_tf, velocity, grounded)) = player_query.single() else {
        return;
    };

    // Detect the frame the player just landed (transition from airborne to grounded)
    let just_landed = grounded.on_ground && !*prev_grounded;
    *prev_grounded = grounded.on_ground;

    if !just_landed {
        return;
    }

    // Only damage if player was falling (moving downward)
    if velocity.0.y > 10.0 {
        return;
    }

    let player_x = player_tf.translation.x;
    let player_y = player_tf.translation.y;
    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (entity, block_tf, block_size, mut block, mut sprite) in &mut block_query {
        let block_x = block_tf.translation.x;
        let block_y = block_tf.translation.y;
        let block_half_w = block_size.0.x / 2.0;
        let block_half_h = block_size.0.y / 2.0;

        // AABB overlap check
        let overlap_x = (player_half_w + block_half_w) - (player_x - block_x).abs();
        let overlap_y = (player_half_h + block_half_h) - (player_y - block_y).abs();

        if overlap_x <= 0.0 || overlap_y < -1.0 {
            continue;
        }

        // Player must be above the block (landed on top)
        if player_y <= block_y {
            continue;
        }

        // Apply damage
        block.health -= BLOCK_DAMAGE_PER_STOMP;

        let block_pos = Vec2::new(block_x, block_y);

        if block.health <= 0 {
            // Destroy: debris explosion + dust particles + score
            let death_color = block_color_for_health(1, block.max_health);
            spawn_debris(&mut commands, block_pos, death_color);
            spawn_burst(
                &mut commands,
                block_pos,
                BLOCK_PARTICLE_COUNT,
                death_color,
                true,
            );
            score.value += BREAKABLE_SCORE_PER_BLOCK;
            commands.entity(entity).despawn();
        } else {
            // Damage: small dust burst + color change
            spawn_burst(
                &mut commands,
                block_pos,
                BLOCK_HIT_PARTICLE_COUNT,
                block_color_for_health(block.health, block.max_health),
                true,
            );
            let new_color = block_color_for_health(block.health, block.max_health);
            sprite.color = new_color;
        }
    }
}
