use bevy::prelude::*;
use rand::Rng;

use crate::constants::*;
use crate::player::Player;
use crate::state::GameState;

/// Resource that drives camera shake. Write to this to trigger a shake.
#[derive(Resource, Default)]
pub struct ScreenShake {
    pub timer: Timer,
    pub intensity: f32,
}

/// Message to trigger a screen shake from other systems.
#[derive(Message)]
pub struct ShakeEvent {
    pub intensity: f32,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScreenShake>()
            .add_message::<ShakeEvent>()
            .add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                (receive_shake_events, camera_follow, apply_screen_shake)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn receive_shake_events(
    mut shake: ResMut<ScreenShake>,
    mut events: MessageReader<ShakeEvent>,
) {
    for event in events.read() {
        // Only override if this shake is stronger than the current one
        if event.intensity > shake.intensity || shake.timer.is_finished() {
            shake.intensity = event.intensity;
            shake.timer = Timer::from_seconds(SHAKE_DURATION, TimerMode::Once);
        }
    }
}

fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let Ok(player_tf) = player_query.single() else {
        return;
    };
    let Ok(mut camera_tf) = camera_query.single_mut() else {
        return;
    };

    let target_x = player_tf.translation.x;
    let target_y = player_tf.translation.y + CAMERA_Y_OFFSET;

    camera_tf.translation.x += (target_x - camera_tf.translation.x) * CAMERA_LERP_SPEED;
    camera_tf.translation.y += (target_y - camera_tf.translation.y) * CAMERA_LERP_SPEED;
}

fn apply_screen_shake(
    time: Res<Time>,
    mut shake: ResMut<ScreenShake>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    shake.timer.tick(time.delta());

    if shake.timer.is_finished() {
        shake.intensity = 0.0;
        return;
    }

    let Ok(mut camera_tf) = camera_query.single_mut() else {
        return;
    };

    // Decay intensity over the shake duration
    let remaining = shake.timer.fraction_remaining();
    let current_intensity = shake.intensity * remaining;

    let mut rng = rand::thread_rng();
    camera_tf.translation.x += rng.gen_range(-current_intensity..current_intensity);
    camera_tf.translation.y += rng.gen_range(-current_intensity..current_intensity);
}
