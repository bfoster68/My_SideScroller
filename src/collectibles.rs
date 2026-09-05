use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::level::Difficulty;
use crate::player::{Coins, Player, PlayerMovementSet, Score};
use crate::spatial::SpatialGrids;
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

/// Gameplay-only clock driving coin/powerup bob animations.
///
/// Fix: `Time::elapsed_secs()` keeps advancing while paused (only the systems are
/// gated), so on resume every bobbing pickup teleported to a new phase. This clock
/// only ticks while Playing, so pausing freezes the bob in place.
#[derive(Resource, Default)]
pub struct BobClock(pub f32);

fn tick_bob_clock(time: Res<Time>, mut clock: ResMut<BobClock>) {
    clock.0 += time.delta_secs();
}

pub struct CollectiblesPlugin;

impl Plugin for CollectiblesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BobClock>().add_systems(
            Update,
            (
                (tick_bob_clock, coin_animate).chain(),
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
    clock: Res<BobClock>,
    mut query: Query<(&mut Transform, &CoinBob), With<Coin>>,
) {
    let t = clock.0;
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
    coin_query: Query<&GlobalTransform, With<Coin>>,
    grids: Res<SpatialGrids>,
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
    let check_radius = player_half_w + coin_half + 50.0;

    for &(entity, _) in grids.coins.query_nearby(player_tf.translation.x, check_radius) {
        let Ok(coin_gtf) = coin_query.get(entity) else { continue; };
        let coin_pos = coin_gtf.translation();
        let overlap_x = (player_half_w + coin_half)
            - (player_tf.translation.x - coin_pos.x).abs();
        let overlap_y = (player_half_h + coin_half)
            - (player_tf.translation.y - coin_pos.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            commands.entity(entity).despawn();
            coins.count += 1;
            // Coins scale in value with difficulty: 10 at d=0, 50 at d=1, more at extreme
            let d = difficulty.value;
            let bonus = if d <= 1.0 {
                d * 40.0
            } else {
                let t = ((d - 1.0) / 1.5).min(1.0);
                40.0 + (EXTREME_COIN_BONUS - 40.0) * t
            };
            let coin_value = COIN_SCORE + bonus as u32;
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
