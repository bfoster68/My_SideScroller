use bevy::prelude::*;
use rand::Rng;

use crate::constants::*;
use crate::player::{Grounded, Player, Velocity};
use crate::state::GameState;

/// A single particle with velocity and remaining lifetime.
#[derive(Component)]
struct Particle {
    velocity: Vec2,
    lifetime: Timer,
}

/// Tracks whether the player was grounded last frame (for detecting landing).
#[derive(Resource, Default)]
struct PrevGrounded(bool);

/// Tracks whether the player was in the air last frame (for detecting jump start).
#[derive(Resource, Default)]
struct PrevAirborne(bool);

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PrevGrounded>()
            .init_resource::<PrevAirborne>()
            .add_systems(
                Update,
                (
                    detect_jump_particles,
                    detect_land_particles,
                    run_dust_while_running,
                    update_particles,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Spawn dust particles when the player jumps.
fn detect_jump_particles(
    mut commands: Commands,
    mut prev_airborne: ResMut<PrevAirborne>,
    query: Query<(&Transform, &Grounded, &Velocity), With<Player>>,
) {
    let Ok((tf, grounded, velocity)) = query.single() else {
        return;
    };

    let is_airborne = !grounded.on_ground;

    // Just became airborne and moving upward = jump
    if is_airborne && !prev_airborne.0 && velocity.0.y > 0.0 {
        spawn_burst(
            &mut commands,
            Vec2::new(tf.translation.x, tf.translation.y - PLAYER_HEIGHT / 2.0),
            PARTICLE_COUNT_JUMP,
            Color::srgb(0.6, 0.5, 0.4), // dusty brown
            true,                         // upward bias
        );
    }

    prev_airborne.0 = is_airborne;
}

/// Spawn dust particles when the player lands.
fn detect_land_particles(
    mut commands: Commands,
    mut prev_grounded: ResMut<PrevGrounded>,
    query: Query<(&Transform, &Grounded), With<Player>>,
) {
    let Ok((tf, grounded)) = query.single() else {
        return;
    };

    // Just became grounded = landing
    if grounded.on_ground && !prev_grounded.0 {
        spawn_burst(
            &mut commands,
            Vec2::new(tf.translation.x, tf.translation.y - PLAYER_HEIGHT / 2.0),
            PARTICLE_COUNT_LAND,
            Color::srgb(0.5, 0.45, 0.35), // dusty brown
            false,                          // horizontal spread
        );
    }

    prev_grounded.0 = grounded.on_ground;
}

/// Spawn small running dust while the player moves on the ground.
fn run_dust_while_running(
    mut commands: Commands,
    query: Query<(&Transform, &Velocity, &Grounded), With<Player>>,
    time: Res<Time>,
) {
    let Ok((tf, velocity, grounded)) = query.single() else {
        return;
    };

    if !grounded.on_ground || velocity.0.x.abs() < 50.0 {
        return;
    }

    // Spawn a particle every few frames (use time-based probability)
    let mut rng = rand::thread_rng();
    if rng.gen_bool((time.delta_secs() * 8.0).min(1.0) as f64) {
        let foot_y = tf.translation.y - PLAYER_HEIGHT / 2.0;
        let behind_x = if velocity.0.x > 0.0 {
            tf.translation.x - PLAYER_WIDTH / 2.0
        } else {
            tf.translation.x + PLAYER_WIDTH / 2.0
        };

        let vx = rng.gen_range(-20.0..20.0) - velocity.0.x * 0.1;
        let vy = rng.gen_range(20.0..60.0);

        commands.spawn((
            Sprite::from_color(
                Color::srgba(0.5, 0.45, 0.35, 0.6),
                Vec2::splat(PARTICLE_SIZE * 0.7),
            ),
            Transform::from_xyz(behind_x, foot_y, 0.5),
            Particle {
                velocity: Vec2::new(vx, vy),
                lifetime: Timer::from_seconds(PARTICLE_LIFETIME * 0.5, TimerMode::Once),
            },
        ));
    }
}

fn spawn_burst(
    commands: &mut Commands,
    origin: Vec2,
    count: usize,
    color: Color,
    upward_bias: bool,
) {
    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let angle = if upward_bias {
            // Mostly upward: between 30° and 150°
            rng.gen_range(std::f32::consts::FRAC_PI_6..std::f32::consts::PI * 5.0 / 6.0)
        } else {
            // Mostly horizontal: between -30° and 30° on both sides
            let base = rng.gen_range(-std::f32::consts::FRAC_PI_6..std::f32::consts::FRAC_PI_6);
            if rng.gen_bool(0.5) {
                std::f32::consts::PI - base
            } else {
                base
            }
        };

        let speed = rng.gen_range(PARTICLE_SPEED * 0.3..PARTICLE_SPEED);
        let vx = angle.cos() * speed;
        let vy = angle.sin() * speed;
        let size = rng.gen_range(PARTICLE_SIZE * 0.5..PARTICLE_SIZE * 1.5);
        let lifetime = rng.gen_range(PARTICLE_LIFETIME * 0.5..PARTICLE_LIFETIME);

        commands.spawn((
            Sprite::from_color(color, Vec2::splat(size)),
            Transform::from_xyz(origin.x + rng.gen_range(-4.0..4.0), origin.y, 0.5),
            Particle {
                velocity: Vec2::new(vx, vy),
                lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
            },
        ));
    }
}

/// Move particles, fade them out, and despawn when lifetime expires.
fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Particle, &mut Sprite)>,
) {
    let dt = time.delta_secs();

    for (entity, mut tf, mut particle, mut sprite) in &mut query {
        particle.lifetime.tick(time.delta());

        // Apply velocity with gravity
        particle.velocity.y += PARTICLE_GRAVITY * dt;
        tf.translation.x += particle.velocity.x * dt;
        tf.translation.y += particle.velocity.y * dt;

        // Fade out based on remaining lifetime
        let fraction_remaining = particle.lifetime.fraction_remaining();
        sprite.color = sprite.color.with_alpha(fraction_remaining);

        // Shrink slightly
        let scale = 0.3 + fraction_remaining * 0.7;
        tf.scale = Vec3::splat(scale);

        if particle.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
