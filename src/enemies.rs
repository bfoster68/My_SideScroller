use bevy::prelude::*;

use crate::audio::AudioHandles;
use crate::camera::HitFreeze;
use crate::constants::*;
use crate::health::{DamageEvent, Invincible};
use crate::particles::spawn_burst;
use crate::player::{Grounded, Player, PlayerMovementSet, Score, Velocity};
use crate::spatial::SpatialGrids;
use crate::state::GameState;

// Re-export types and spawn functions so existing `use crate::enemies::*` imports work.
pub use crate::enemy_types::{
    Projectile,
    spawn_enemy, spawn_enemy_on_moving, spawn_flying_enemy,
    spawn_shooter_enemy, spawn_charging_enemy, spawn_flying_ranged_enemy,
};

// ---------------------------------------------------------------------------
// Shared Components
// ---------------------------------------------------------------------------

/// Marker component for enemy entities (all types share this for queries).
#[derive(Component)]
pub struct Enemy;

/// Defines the horizontal patrol bounds for a walking enemy.
#[derive(Component)]
pub struct Patrol {
    pub left_bound: f32,
    pub right_bound: f32,
    pub direction: f32,
}

/// Floating score text that rises and fades in world space.
#[derive(Component)]
pub struct ScorePopup {
    pub timer: Timer,
}

/// Tracks consecutive stomps without landing for combo multiplier.
#[derive(Resource, Default)]
pub struct ComboTracker {
    pub count: u32,
    pub display_timer: Timer,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct EnemiesPlugin;

impl Plugin for EnemiesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ComboTracker>()
            .add_systems(
                Update,
                (
                    crate::enemy_types::enemy_patrol,
                    crate::enemy_types::flying_enemy_movement,
                    crate::enemy_types::charging_enemy_behavior,
                    crate::enemy_types::flying_ranged_fire,
                    crate::enemy_types::shooter_fire,
                    crate::enemy_types::projectile_update,
                    enemy_player_collision,
                    projectile_player_collision,
                    update_score_popups,
                    reset_combo_on_land,
                )
                    .chain()
                    .after(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Collision
// ---------------------------------------------------------------------------

/// Check player–enemy collisions: stomp from above kills enemy, side contact deals damage.
fn enemy_player_collision(
    mut commands: Commands,
    mut player_query: Query<
        (&Transform, &mut Velocity, Option<&Invincible>, Option<&crate::health::DeathTimer>),
        With<Player>,
    >,
    enemy_query: Query<(&GlobalTransform, &Sprite), (With<Enemy>, Without<Projectile>)>,
    grids: Res<SpatialGrids>,
    mut damage_events: MessageWriter<DamageEvent>,
    mut score: ResMut<Score>,
    mut combo: ResMut<ComboTracker>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, mut player_vel, invincible, death)) = player_query.single_mut() else {
        return;
    };
    if death.is_some() {
        return;
    }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let check_radius = player_half_w + ENEMY_WIDTH;

    for &(enemy_entity, _) in grids.enemies.query_nearby(player_tf.translation.x, check_radius) {
        let Ok((enemy_gtf, enemy_sprite)) = enemy_query.get(enemy_entity) else { continue; };
        let enemy_pos = enemy_gtf.translation();
        let enemy_size = enemy_sprite.custom_size.unwrap_or(Vec2::new(ENEMY_WIDTH, ENEMY_HEIGHT));
        let (enemy_half_w, enemy_half_h) = (enemy_size.x / 2.0, enemy_size.y / 2.0);
        let overlap_x = (player_half_w + enemy_half_w)
            - (player_tf.translation.x - enemy_pos.x).abs();
        let overlap_y = (player_half_h + enemy_half_h)
            - (player_tf.translation.y - enemy_pos.y).abs();

        if overlap_x <= 0.0 || overlap_y <= 0.0 {
            continue;
        }

        let player_bottom = player_tf.translation.y - player_half_h;
        let stomp_zone =
            enemy_pos.y + enemy_half_h * (1.0 - 2.0 * ENEMY_STOMP_THRESHOLD);

        let is_stomp = player_vel.0.y < 0.0 && player_bottom >= stomp_zone;

        if is_stomp {
            let death_pos = Vec2::new(enemy_pos.x, enemy_pos.y);

            let multiplier = 2u32.pow(combo.count.min(MAX_COMBO_POWER));
            let kill_score = ENEMY_KILL_SCORE * multiplier;
            combo.count += 1;
            combo.display_timer = Timer::from_seconds(COMBO_DISPLAY_DURATION, TimerMode::Once);

            commands.entity(enemy_entity).despawn();
            player_vel.0.y = ENEMY_STOMP_BOUNCE;
            score.value += kill_score;

            spawn_burst(
                &mut commands,
                death_pos,
                ENEMY_DEATH_PARTICLE_COUNT,
                Color::srgb(0.9, 0.4, 0.1),
                true,
            );

            commands.spawn((
                ScorePopup {
                    timer: Timer::from_seconds(SCORE_POPUP_DURATION, TimerMode::Once),
                },
                Text2d::new(format!("+{}", kill_score)),
                TextFont { font_size: FontSize::Px(20.0), ..default() },
                TextColor(Color::srgb(1.0, 1.0, 0.3)),
                Transform::from_xyz(death_pos.x, death_pos.y + 20.0, 5.0),
            ));

            commands.insert_resource(HitFreeze {
                timer: Timer::from_seconds(STOMP_FREEZE_DURATION, TimerMode::Once),
                time_scale: STOMP_FREEZE_SCALE,
            });
        } else if invincible.is_none() {
            let enemy_pos_2d = Vec2::new(enemy_pos.x, enemy_pos.y);
            damage_events.write(DamageEvent {
                amount: 1,
                source_pos: Some(enemy_pos_2d),
            });
            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            commands.insert_resource(HitFreeze {
                timer: Timer::from_seconds(DAMAGE_FREEZE_DURATION, TimerMode::Once),
                time_scale: DAMAGE_FREEZE_SCALE,
            });
            break;
        }
    }
}

/// Update floating score popups — rise, fade, and despawn.
fn update_score_popups(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut ScorePopup, &mut TextColor)>,
) {
    for (entity, mut tf, mut popup, mut color) in &mut query {
        popup.timer.tick(time.delta());
        tf.translation.y += SCORE_POPUP_RISE_SPEED * time.delta_secs();
        let alpha = popup.timer.fraction_remaining();
        color.0 = color.0.with_alpha(alpha);
        if popup.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Reset combo when player touches the ground.
fn reset_combo_on_land(
    mut commands: Commands,
    mut combo: ResMut<ComboTracker>,
    player_query: Query<(&Transform, &Grounded), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((tf, grounded)) = player_query.single() {
        if grounded.on_ground && combo.count >= 2 {
            let combo_power = (combo.count.saturating_sub(1)).min(MAX_COMBO_POWER);
            let multiplier = 1u32 << combo_power;
            commands.spawn((
                Text2d::new(format!("x{} COMBO", multiplier)),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::srgb(1.0, 0.4, 0.2)),
                Transform::from_xyz(
                    tf.translation.x,
                    tf.translation.y + PLAYER_HEIGHT / 2.0 + 10.0,
                    10.0,
                ),
                ScorePopup {
                    timer: Timer::from_seconds(SCORE_POPUP_DURATION, TimerMode::Once),
                },
            ));
            combo.count = 0;
        } else if grounded.on_ground {
            combo.count = 0;
        }
    }
    combo.display_timer.tick(time.delta());
}

/// Projectile–player collision.
fn projectile_player_collision(
    mut commands: Commands,
    player_query: Query<(&Transform, Option<&Invincible>, Option<&crate::health::DeathTimer>), With<Player>>,
    projectile_query: Query<&Transform, With<Projectile>>,
    grids: Res<SpatialGrids>,
    mut damage_events: MessageWriter<DamageEvent>,
    audio_handles: Option<Res<AudioHandles>>,
) {
    let Ok((player_tf, invincible, death)) = player_query.single() else {
        return;
    };

    if death.is_some() || invincible.is_some() {
        return;
    }

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;
    let proj_half = PROJECTILE_SIZE / 2.0;
    let check_radius = player_half_w + proj_half + 50.0;

    for &(entity, _) in grids.projectiles.query_nearby(player_tf.translation.x, check_radius) {
        let Ok(proj_tf) = projectile_query.get(entity) else { continue; };
        let overlap_x = (player_half_w + proj_half)
            - (player_tf.translation.x - proj_tf.translation.x).abs();
        let overlap_y = (player_half_h + proj_half)
            - (player_tf.translation.y - proj_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            let impact_pos = Vec2::new(proj_tf.translation.x, proj_tf.translation.y);
            commands.entity(entity).despawn();
            damage_events.write(DamageEvent {
                amount: PROJECTILE_DAMAGE,
                source_pos: Some(impact_pos),
            });

            crate::particles::spawn_burst(
                &mut commands,
                impact_pos,
                6,
                Color::srgb(1.0, 0.3, 0.3),
                false,
            );

            if let Some(ref handles) = audio_handles {
                if let Some(ref handle) = handles.hit {
                    crate::audio::spawn_sfx(&mut commands, handle);
                }
            }

            break;
        }
    }
}
