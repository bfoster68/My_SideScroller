use bevy::prelude::*;

use crate::constants::*;
use crate::player::Player;
use crate::state::GameState;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera)
            .add_systems(
                Update,
                camera_follow.run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
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
