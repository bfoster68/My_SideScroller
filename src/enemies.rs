use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::player::{Player, PlayerMovementSet, Score, Velocity};
use crate::state::GameState;

/// Marker component for enemy entities.
#[derive(Component)]
pub struct Enemy;

/// Defines the horizontal patrol bounds for an enemy.
#[derive(Component)]
pub struct Patrol {
    pub left_bound: f32,
    pub right_bound: f32,
    pub direction: f32, // -1.0 or 1.0
}

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (enemy_patrol, enemy_player_collision)
                .chain()
                .after(PlayerMovementSet)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Spawn an enemy on top of a platform. Called from level generation.
pub fn spawn_enemy(
    commands: &mut Commands,
    platform_x: f32,
    platform_y: f32,
    platform_width: f32,
) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let left = platform_x - half_plat + enemy_half;
    let right = platform_x + half_plat - enemy_half;
    let spawn_y = platform_y + (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.85, 0.2, 0.15), // red/orange
            Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
        ),
        Transform::from_xyz(platform_x, spawn_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: left,
            right_bound: right,
            direction: 1.0,
        },
    ));
}

/// Spawn an enemy as a child of a moving platform (local coordinates).
pub fn spawn_enemy_on_moving(parent: &mut ChildSpawnerCommands, platform_width: f32) {
    let half_plat = platform_width / 2.0;
    let enemy_half = ENEMY_WIDTH / 2.0;
    let local_y = (PLATFORM_HEIGHT / 2.0) + (ENEMY_HEIGHT / 2.0);

    parent.spawn((
        Sprite::from_color(
            Color::srgb(0.85, 0.2, 0.15),
            Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT),
        ),
        Transform::from_xyz(0.0, local_y, ENEMY_Z),
        Enemy,
        Patrol {
            left_bound: -half_plat + enemy_half,
            right_bound: half_plat - enemy_half,
            direction: 1.0,
        },
    ));
}

/// Move enemies back and forth within their patrol bounds.
fn enemy_patrol(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Patrol, &mut Sprite), With<Enemy>>,
) {
    for (mut transform, mut patrol, mut sprite) in &mut query {
        transform.translation.x += patrol.direction * ENEMY_SPEED * time.delta_secs();

        // Reverse at bounds
        if transform.translation.x >= patrol.right_bound {
            transform.translation.x = patrol.right_bound;
            patrol.direction = -1.0;
        } else if transform.translation.x <= patrol.left_bound {
            transform.translation.x = patrol.left_bound;
            patrol.direction = 1.0;
        }

        // Flip sprite to face movement direction
        sprite.flip_x = patrol.direction < 0.0;
    }
}

/// Check player–enemy collisions: stomp from above kills enemy, side contact deals damage.
fn enemy_player_collision(
    mut commands: Commands,
    mut player_query: Query<
        (&Transform, &mut Velocity, Option<&Invincible>),
        With<Player>,
    >,
    enemy_query: Query<(Entity, &GlobalTransform), With<Enemy>>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut score: ResMut<Score>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, mut player_vel, invincible)) = player_query.single_mut() else {
        return;
    };

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let enemy_half_w = ENEMY_WIDTH / 2.0;
    let enemy_half_h = ENEMY_HEIGHT / 2.0;

    for (enemy_entity, enemy_gtf) in &enemy_query {
        let enemy_pos = enemy_gtf.translation();
        // AABB overlap test
        let overlap_x = (player_half_w + enemy_half_w)
            - (player_tf.translation.x - enemy_pos.x).abs();
        let overlap_y = (player_half_h + enemy_half_h)
            - (player_tf.translation.y - enemy_pos.y).abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            continue;
        }

        // Determine stomp vs contact damage
        let player_bottom = player_tf.translation.y - player_half_h;
        let stomp_zone =
            enemy_pos.y + enemy_half_h * (1.0 - 2.0 * ENEMY_STOMP_THRESHOLD);

        let is_stomp = player_vel.0.y < 0.0 && player_bottom >= stomp_zone;

        if is_stomp {
            // Kill enemy
            commands.entity(enemy_entity).despawn();
            // Bounce player upward
            player_vel.0.y = ENEMY_STOMP_BOUNCE;
            // Award score
            score.value += ENEMY_KILL_SCORE;
        } else if invincible.is_none() {
            // Contact damage
            damage_events.write(DamageEvent { amount: 1 });
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
