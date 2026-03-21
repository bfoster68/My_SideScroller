use bevy::prelude::*;
use bevy::camera::ScalingMode;
use rand::Rng;

use crate::constants::*;
use crate::health::DamageEvent;
use crate::player::Player;
use crate::state::GameState;

/// Tracks screen-shake intensity. Trauma decays over time; offset = trauma² * max_offset.
#[derive(Resource, Default)]
pub struct ScreenShake {
    pub trauma: f32,
}

/// Brief time-scale reduction for impact feel on stomp/damage.
#[derive(Resource)]
pub struct HitFreeze {
    pub timer: Timer,
    pub time_scale: f32,
}

/// Marker resource: camera needs to snap to player on next frame.
#[derive(Resource)]
struct NeedsCameraSnap;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenShake>()
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(GameState::Playing), request_camera_snap)
            .add_systems(
                Update,
                (manage_hit_freeze, damage_triggers_shake, try_snap_camera, camera_follow)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Request a camera snap when entering Playing state.
fn request_camera_snap(mut commands: Commands) {
    commands.insert_resource(NeedsCameraSnap);
}

/// On the first Update frame where the player exists, snap the camera instantly.
/// This handles both fresh spawns and checkpoint resumes.
fn try_snap_camera(
    mut commands: Commands,
    snap: Option<Res<NeedsCameraSnap>>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    if snap.is_none() {
        return;
    }
    let Ok(player_tf) = player_query.single() else { return };
    let Ok(mut camera_tf) = camera_query.single_mut() else { return };

    camera_tf.translation.x = player_tf.translation.x;
    camera_tf.translation.y = player_tf.translation.y + CAMERA_Y_OFFSET;
    commands.remove_resource::<NeedsCameraSnap>();
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: 720.0,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Manage hit freeze: slow virtual time, tick with real time, restore when done.
fn manage_hit_freeze(
    mut commands: Commands,
    freeze: Option<ResMut<HitFreeze>>,
    mut time: ResMut<Time<Virtual>>,
    real_time: Res<Time<Real>>,
) {
    if let Some(mut freeze) = freeze {
        freeze.timer.tick(real_time.delta());
        if freeze.timer.is_finished() {
            time.set_relative_speed(1.0);
            commands.remove_resource::<HitFreeze>();
        } else {
            time.set_relative_speed(freeze.time_scale);
        }
    } else if time.relative_speed() < 1.0 {
        // Restore speed if HitFreeze was removed externally (e.g. on death)
        time.set_relative_speed(1.0);
    }
}

/// On any damage event, add trauma for screen shake.
fn damage_triggers_shake(
    mut shake: ResMut<ScreenShake>,
    mut damage_events: MessageReader<DamageEvent>,
) {
    for _event in damage_events.read() {
        shake.trauma = (shake.trauma + SHAKE_TRAUMA_ON_HIT).min(1.0);
    }
}

fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    mut shake: ResMut<ScreenShake>,
    time: Res<Time>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_query.single_mut() else {
        return;
    };

    // Smooth follow
    let target_x = player_tf.translation.x;
    let target_y = player_tf.translation.y + CAMERA_Y_OFFSET;

    camera_tf.translation.x += (target_x - camera_tf.translation.x) * CAMERA_LERP_SPEED;
    camera_tf.translation.y += (target_y - camera_tf.translation.y) * CAMERA_LERP_SPEED;

    // Apply screen shake offset (trauma² for nice falloff)
    if shake.trauma > 0.0 {
        let intensity = shake.trauma * shake.trauma;
        let mut rng = rand::thread_rng();
        let offset_x = rng.gen_range(-1.0..1.0) * SHAKE_MAX_OFFSET * intensity;
        let offset_y = rng.gen_range(-1.0..1.0) * SHAKE_MAX_OFFSET * intensity;

        camera_tf.translation.x += offset_x;
        camera_tf.translation.y += offset_y;

        // Decay trauma
        shake.trauma = (shake.trauma - SHAKE_DECAY * time.delta_secs()).max(0.0);
    }
}
