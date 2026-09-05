use bevy::prelude::*;

use crate::animation::PlayerAnimState;
use crate::audio::AudioHandles;
use crate::constants::*;
use crate::player::{Player, Velocity};
use crate::powerups::Shield;
use crate::state::GameState;

/// Tracks player health and lives.
#[derive(Component)]
pub struct Health {
    pub current: i32,
    pub max: i32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: MAX_HEALTH,
            max: MAX_HEALTH,
        }
    }
}

/// Present on the player while they are temporarily invincible after a hit.
#[derive(Component)]
pub struct Invincible {
    pub timer: Timer,
}

/// Message fired when the player takes damage.
#[derive(Message)]
pub struct DamageEvent {
    pub amount: i32,
    pub source_pos: Option<Vec2>,
}

/// Applied to the player during knockback — reduces input control.
#[derive(Component)]
pub struct Knockback {
    pub timer: Timer,
}

// PlayerDeathEvent removed — was written but never consumed by any system.

/// Attached to the player during the death animation to delay the GameOver transition.
#[derive(Component)]
pub struct DeathTimer {
    pub timer: Timer,
    pub shattered: bool,
}

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEvent>()
            .add_systems(
                Update,
                (apply_damage, tick_knockback, tick_death_animation, update_death_fragments, tick_invincibility, flash_invincible)
                    .chain()
                    // Must run AFTER player movement: apply_damage writes the
                    // knockback impulse to Velocity but inserts the Knockback
                    // marker via deferred commands. If it ran before player_input
                    // in the same frame, player_input saw no Knockback yet and
                    // overwrote the impulse to zero — so hits had no pushback.
                    .after(crate::player::PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Listen for damage messages, reduce health, grant invincibility.
fn apply_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut query: Query<
        (Entity, &Transform, &mut Health, &mut Velocity, Option<&Invincible>, Option<&DeathTimer>, Option<&mut Shield>),
        With<Player>,
    >,
    god_mode: Res<crate::debug::GodMode>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((entity, player_tf, mut health, mut velocity, invincible, death_timer, mut shield)) = query.single_mut()
    else {
        return;
    };

    // Already dying — ignore further damage
    if death_timer.is_some() {
        for _ in damage_events.read() {}
        return;
    }

    // `Invincible` is inserted via deferred Commands, so it isn't visible until
    // next frame. Track locally so only ONE hit lands per frame even when several
    // DamageEvents are queued at once (e.g. enemy body + projectile overlap).
    let mut hit_this_frame = invincible.is_some();

    for event in damage_events.read() {
        // Can't take damage while invincible, already hit this frame, or in god mode
        if hit_this_frame || god_mode.0 {
            continue;
        }

        // Shield absorbs the hit
        if let Some(ref mut s) = shield {
            hit_this_frame = true;
            // Clamp so a second event can't drive this negative and re-fire the break burst
            s.hits_remaining = (s.hits_remaining - 1).max(0);
            if s.hits_remaining <= 0 {
                commands.entity(entity).remove::<Shield>();
                // Gold particle burst when shield breaks
                crate::particles::spawn_burst(
                    &mut commands,
                    player_tf.translation.truncate(),
                    10,
                    Color::srgb(1.0, 0.85, 0.2),
                    true,
                );
            }
            continue;
        }

        health.current = (health.current - event.amount).max(0);
        hit_this_frame = true;

        if health.current <= 0 {
            // Play death SFX (quieter than other effects)
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.death {
                    crate::audio::spawn_sfx_at_volume(&mut commands, handle, 0.35);
                }
            }
            // Clear any active hit-freeze so the death animation isn't
            // stuck in slow-motion (HitFreeze slows virtual time to ~2%).
            commands.remove_resource::<crate::camera::HitFreeze>();
            // Start death animation instead of immediate GameOver
            commands.entity(entity).insert((
                DeathTimer {
                    timer: Timer::from_seconds(DEATH_ANIM_DURATION, TimerMode::Once),
                    shattered: false,
                },
                PlayerAnimState::Death,
            ));
        } else {
            // Grant invincibility frames
            commands.entity(entity).insert(Invincible {
                timer: Timer::from_seconds(INVINCIBILITY_DURATION, TimerMode::Once),
            });

            // Apply knockback away from damage source
            if let Some(src) = event.source_pos {
                let player_pos = Vec2::new(player_tf.translation.x, player_tf.translation.y);
                let dir = (player_pos - src).normalize_or_zero();
                velocity.0.x = dir.x * KNOCKBACK_FORCE;
                velocity.0.y = KNOCKBACK_LIFT;
                commands.entity(entity).insert(Knockback {
                    timer: Timer::from_seconds(KNOCKBACK_DURATION, TimerMode::Once),
                });
            }
        }
    }
}

/// Tick knockback timer and remove when expired.
fn tick_knockback(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Knockback), With<Player>>,
) {
    let Ok((entity, mut kb)) = query.single_mut() else {
        return;
    };
    kb.timer.tick(time.delta());
    if kb.timer.is_finished() {
        commands.entity(entity).remove::<Knockback>();
    }
}

/// Tick the death timer, fade the player out, then transition to GameOver.
/// A fragment of the player sprite that flies outward on death.
#[derive(Component)]
struct DeathFragment {
    velocity: Vec2,
    angular_velocity: f32,
    lifetime: Timer,
}

fn tick_death_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeathTimer, &mut Transform, &mut Sprite), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((entity, mut death, transform, mut sprite)) = query.single_mut() else {
        return;
    };

    death.timer.tick(time.delta());

    if !death.shattered {
        // First frame: hide player and spawn shatter fragments
        death.shattered = true;
        sprite.color = sprite.color.with_alpha(0.0);

        let pos = transform.translation.truncate();
        spawn_death_fragments(&mut commands, pos);
    }

    if death.timer.is_finished() {
        commands.entity(entity).remove::<DeathTimer>();
        // Keep player hidden — sprite alpha restored in reset_player_on_game_over
        next_state.set(GameState::GameOver);
    }
}

fn spawn_death_fragments(commands: &mut Commands, pos: Vec2) {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    // Player body colors for fragments
    let colors = [
        Color::srgb(0.9, 0.75, 0.6),  // skin tone
        Color::srgb(0.3, 0.5, 0.8),   // blue clothing
        Color::srgb(0.2, 0.4, 0.7),   // darker blue
        Color::srgb(0.8, 0.3, 0.3),   // red accent
        Color::srgb(0.95, 0.85, 0.7), // light skin
        Color::srgb(0.4, 0.3, 0.2),   // hair/dark
    ];

    let fragment_count = 16;
    for i in 0..fragment_count {
        let w = rng.gen_range(6.0..16.0);
        let h = rng.gen_range(6.0..14.0);

        // Spread starting position across the player area
        let offset_x = rng.gen_range(-PLAYER_WIDTH / 2.0..PLAYER_WIDTH / 2.0);
        let offset_y = rng.gen_range(-PLAYER_HEIGHT / 2.0..PLAYER_HEIGHT / 2.0);

        // Explode outward from center
        let angle = (i as f32 / fragment_count as f32) * std::f32::consts::TAU
            + rng.gen_range(-0.3..0.3);
        let speed = rng.gen_range(120.0..320.0);
        let vx = angle.cos() * speed;
        let vy = angle.sin() * speed + rng.gen_range(50.0..150.0); // bias upward

        let color = colors[rng.gen_range(0..colors.len())];
        let lifetime = rng.gen_range(0.5..1.0);

        commands.spawn((
            Sprite::from_color(color, Vec2::new(w, h)),
            Transform::from_xyz(pos.x + offset_x, pos.y + offset_y, 5.0),
            DeathFragment {
                velocity: Vec2::new(vx, vy),
                angular_velocity: rng.gen_range(-15.0..15.0),
                lifetime: Timer::from_seconds(lifetime, TimerMode::Once),
            },
        ));
    }
}

fn update_death_fragments(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut DeathFragment, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (entity, mut tf, mut frag, mut sprite) in &mut query {
        frag.lifetime.tick(time.delta());

        // Gravity
        frag.velocity.y += GRAVITY * 0.6 * dt;

        // Move
        tf.translation.x += frag.velocity.x * dt;
        tf.translation.y += frag.velocity.y * dt;

        // Spin
        tf.rotation = Quat::from_rotation_z(
            tf.rotation.to_euler(EulerRot::ZYX).0 + frag.angular_velocity * dt,
        );

        // Fade + shrink
        let remaining = frag.lifetime.fraction_remaining();
        sprite.color = sprite.color.with_alpha(remaining);
        let scale = 0.3 + 0.7 * remaining;
        tf.scale = Vec3::splat(scale);

        if frag.lifetime.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Count down the invincibility timer and remove the component when done.
fn tick_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Invincible, &mut Sprite), With<Player>>,
) {
    let Ok((entity, mut inv, mut sprite)) = query.single_mut() else {
        return;
    };

    inv.timer.tick(time.delta());

    if inv.timer.is_finished() {
        commands.entity(entity).remove::<Invincible>();
        // Restore full visibility
        sprite.color = sprite.color.with_alpha(1.0);
    }
}

/// Flash the player sprite while invincible.
fn flash_invincible(
    mut query: Query<(&Invincible, &mut Sprite), With<Player>>,
) {
    let Ok((inv, mut sprite)) = query.single_mut() else {
        return;
    };

    let elapsed = inv.timer.elapsed_secs();
    let flash = (elapsed * INVINCIBILITY_FLASH_RATE).sin();
    let alpha = if flash > 0.0 { 1.0 } else { 0.3 };
    sprite.color = sprite.color.with_alpha(alpha);
}
