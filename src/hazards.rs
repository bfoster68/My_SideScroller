use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::player::{Player, PlayerMovementSet};
use crate::state::GameState;

/// Marker for spike hazard entities.
#[derive(Component)]
pub struct Spike;

pub struct HazardsPlugin;

impl Plugin for HazardsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            spike_player_collision
                .after(PlayerMovementSet)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Spawn a spike on top of a platform. Called from level generation.
pub fn spawn_spike(commands: &mut Commands, x: f32, platform_y: f32) {
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (SPIKE_HEIGHT / 2.0);

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.6, 0.1, 0.1), // dark red
            Vec2::new(SPIKE_WIDTH, SPIKE_HEIGHT),
        ),
        Transform::from_xyz(x, spawn_y, SPIKE_Z),
        Spike,
    ));
}

/// Spawn a spike as a child of a moving platform (local coordinates).
pub fn spawn_spike_on_moving(parent: &mut ChildSpawnerCommands) {
    let local_y = (PLATFORM_HEIGHT / 2.0) + (SPIKE_HEIGHT / 2.0);

    parent.spawn((
        Sprite::from_color(
            Color::srgb(0.6, 0.1, 0.1),
            Vec2::new(SPIKE_WIDTH, SPIKE_HEIGHT),
        ),
        Transform::from_xyz(0.0, local_y, SPIKE_Z),
        Spike,
    ));
}

/// Damage the player when they touch a spike.
fn spike_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>), With<Player>>,
    spike_query: Query<&GlobalTransform, With<Spike>>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible)) = player_query.single() else {
        return;
    };

    // Early out: no point checking spikes while invincible
    if invincible.is_some() {
        return;
    }

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

            // Play hit SFX
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    commands.spawn(AudioPlayer::new(handle.clone()));
                }
            }

            break; // Only take one hit per frame
        }
    }
}
