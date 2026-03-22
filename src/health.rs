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

/// Message fired when the player dies (health reaches 0).
#[derive(Message)]
pub struct PlayerDeathEvent;

/// Attached to the player during the death animation to delay the GameOver transition.
#[derive(Component)]
pub struct DeathTimer {
    pub timer: Timer,
}

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEvent>()
            .add_message::<PlayerDeathEvent>()
            .add_systems(
                Update,
                (apply_damage, tick_knockback, tick_death_animation, tick_invincibility, flash_invincible)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Listen for damage messages, reduce health, grant invincibility.
fn apply_damage(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut death_events: MessageWriter<PlayerDeathEvent>,
    mut query: Query<
        (Entity, &Transform, &mut Health, &mut Velocity, Option<&Invincible>, Option<&DeathTimer>, Option<&mut Shield>),
        With<Player>,
    >,
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

    for event in damage_events.read() {
        // Can't take damage while invincible
        if invincible.is_some() {
            continue;
        }

        // Shield absorbs the hit
        if let Some(ref mut s) = shield {
            s.hits_remaining -= 1;
            if s.hits_remaining <= 0 {
                commands.entity(entity).remove::<Shield>();
            }
            continue;
        }

        health.current = (health.current - event.amount).max(0);

        if health.current <= 0 {
            death_events.write(PlayerDeathEvent);
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
fn tick_death_animation(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeathTimer, &mut Sprite), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((entity, mut death, mut sprite)) = query.single_mut() else {
        return;
    };

    death.timer.tick(time.delta());

    // Fade out over the death duration
    let alpha = death.timer.fraction_remaining();
    sprite.color = sprite.color.with_alpha(alpha);

    if death.timer.is_finished() {
        commands.entity(entity).remove::<DeathTimer>();
        sprite.color = sprite.color.with_alpha(1.0);
        next_state.set(GameState::GameOver);
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
