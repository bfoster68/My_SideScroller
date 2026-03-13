use bevy::prelude::*;

use crate::camera::ShakeEvent;
use crate::constants::*;
use crate::player::Player;
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

/// Brief timer that locks the player into the Hurt animation.
#[derive(Component)]
pub struct HurtTimer {
    pub timer: Timer,
}

/// Timer that plays the Faint animation before transitioning to GameOver.
#[derive(Component)]
pub struct DeathTimer {
    pub timer: Timer,
}

/// Message fired when the player takes damage.
#[derive(Message)]
pub struct DamageEvent {
    pub amount: i32,
}

/// Message fired when the player dies (health reaches 0).
#[derive(Message)]
pub struct PlayerDeathEvent;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageEvent>()
            .add_message::<PlayerDeathEvent>()
            .add_systems(
                Update,
                (
                    apply_damage,
                    tick_hurt_timer,
                    tick_death_timer,
                    tick_invincibility,
                    flash_invincible,
                )
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
    mut shake_events: MessageWriter<ShakeEvent>,
    mut query: Query<(Entity, &mut Health, Option<&Invincible>, Option<&DeathTimer>), With<Player>>,
) {
    let Ok((entity, mut health, invincible, death_timer)) = query.single_mut() else {
        return;
    };

    // Don't process damage during death animation
    if death_timer.is_some() {
        return;
    }

    for event in damage_events.read() {
        // Can't take damage while invincible
        if invincible.is_some() {
            continue;
        }

        health.current = (health.current - event.amount).max(0);

        // Screen shake on damage
        shake_events.write(ShakeEvent {
            intensity: SHAKE_INTENSITY_DAMAGE,
        });

        if health.current <= 0 {
            death_events.write(PlayerDeathEvent);
            // Start death animation timer instead of immediately going to GameOver
            commands.entity(entity).insert(DeathTimer {
                timer: Timer::from_seconds(DEATH_DURATION, TimerMode::Once),
            });
        } else {
            // Grant invincibility frames
            commands.entity(entity).insert(Invincible {
                timer: Timer::from_seconds(INVINCIBILITY_DURATION, TimerMode::Once),
            });
            // Play hurt animation briefly
            commands.entity(entity).insert(HurtTimer {
                timer: Timer::from_seconds(HURT_DURATION, TimerMode::Once),
            });
        }
    }
}

/// Count down the hurt timer and remove it when done.
fn tick_hurt_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut HurtTimer), With<Player>>,
) {
    let Ok((entity, mut hurt)) = query.single_mut() else {
        return;
    };

    hurt.timer.tick(time.delta());

    if hurt.timer.is_finished() {
        commands.entity(entity).remove::<HurtTimer>();
    }
}

/// Count down the death timer and transition to GameOver when done.
fn tick_death_timer(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DeathTimer), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((entity, mut death)) = query.single_mut() else {
        return;
    };

    death.timer.tick(time.delta());

    if death.timer.is_finished() {
        commands.entity(entity).remove::<DeathTimer>();
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
