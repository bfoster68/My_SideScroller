use bevy::prelude::*;

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
                (apply_damage, tick_invincibility, flash_invincible)
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
    mut query: Query<(Entity, &mut Health, Option<&Invincible>), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    let Ok((entity, mut health, invincible)) = query.single_mut() else {
        return;
    };

    for event in damage_events.read() {
        // Can't take damage while invincible
        if invincible.is_some() {
            continue;
        }

        health.current = (health.current - event.amount).max(0);

        if health.current <= 0 {
            death_events.write(PlayerDeathEvent);
            next_state.set(GameState::GameOver);
        } else {
            // Grant invincibility frames
            commands.entity(entity).insert(Invincible {
                timer: Timer::from_seconds(INVINCIBILITY_DURATION, TimerMode::Once),
            });
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
