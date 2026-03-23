use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::level::Difficulty;
use crate::player::{Coins, Player, PlayerMovementSet, Score};
use crate::state::GameState;

/// Marker for coin entities.
#[derive(Component)]
pub struct Coin;

/// Tracks the base Y position and phase offset for the bobbing animation.
#[derive(Component)]
pub struct CoinBob {
    pub base_y: f32,
    pub phase: f32,
}

pub struct CollectiblesPlugin;

impl Plugin for CollectiblesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                coin_animate,
                coin_player_collision.after(PlayerMovementSet),
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Spawn a coin floating above a platform. Called from level generation.
pub fn spawn_coin(commands: &mut Commands, x: f32, platform_y: f32, image: Handle<Image>) {
    let y = platform_y + (PLATFORM_HEIGHT / 2.0) + COIN_FLOAT_HEIGHT;
    spawn_coin_at(commands, x, y, image);
}

/// Spawn a coin at an exact world position (for mid-air coins).
pub fn spawn_coin_at(commands: &mut Commands, x: f32, y: f32, image: Handle<Image>) {
    commands.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(COIN_SIZE, COIN_SIZE)),
            ..default()
        },
        Transform::from_xyz(x, y, COIN_Z),
        Coin,
        CoinBob {
            base_y: y,
            phase: x * 0.1, // deterministic variety from position
        },
    ));
}

/// Spawn a coin as a child of a moving platform (local coordinates).
pub fn spawn_coin_on_moving(parent: &mut ChildSpawnerCommands, image: Handle<Image>) {
    let local_y = (PLATFORM_HEIGHT / 2.0) + COIN_FLOAT_HEIGHT;

    parent.spawn((
        Sprite {
            image,
            custom_size: Some(Vec2::new(COIN_SIZE, COIN_SIZE)),
            ..default()
        },
        Transform::from_xyz(0.0, local_y, COIN_Z),
        Coin,
        CoinBob {
            base_y: local_y,
            phase: 0.0,
        },
    ));
}

/// Bob coins up/down and simulate spinning via X-scale oscillation.
fn coin_animate(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &CoinBob), With<Coin>>,
) {
    let t = time.elapsed_secs();
    for (mut transform, bob) in &mut query {
        // Vertical bob
        transform.translation.y =
            bob.base_y + (t * COIN_BOB_SPEED + bob.phase).sin() * COIN_BOB_AMPLITUDE;

        // Spin effect via X-scale oscillation
        let scale_x = (t * COIN_SPIN_SPEED + bob.phase).cos().abs().max(0.3);
        transform.scale.x = scale_x;
    }
}

/// Collect coins on player overlap.
fn coin_player_collision(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    coin_query: Query<(Entity, &GlobalTransform), With<Coin>>,
    mut coins: ResMut<Coins>,
    mut score: ResMut<Score>,
    difficulty: Res<Difficulty>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let coin_half = COIN_SIZE / 2.0;

    for (entity, coin_gtf) in &coin_query {
        let coin_pos = coin_gtf.translation();
        let overlap_x = (player_half_w + coin_half)
            - (player_tf.translation.x - coin_pos.x).abs();
        let overlap_y = (player_half_h + coin_half)
            - (player_tf.translation.y - coin_pos.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            commands.entity(entity).despawn();
            coins.count += 1;
            // Coins scale in value with difficulty: 10 at d=0, up to 50 at d=1
            let coin_value = COIN_SCORE + (difficulty.value * 40.0) as u32;
            score.value = score.value.saturating_add(coin_value);

            // Play collect SFX
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.collect {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }
        }
    }
}
