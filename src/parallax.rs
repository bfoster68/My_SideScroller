use bevy::prelude::*;

use crate::constants::*;
use crate::state::GameState;

/// Tag for parallax background layers. `speed` is the fraction of camera movement
/// this layer follows: 0.0 = fully static, 1.0 = moves with camera.
#[derive(Component)]
pub struct ParallaxLayer {
    pub speed: f32,
    /// The layer's original X position before any parallax offset.
    pub home_x: f32,
}

pub struct ParallaxPlugin;

impl Plugin for ParallaxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_backgrounds)
            .add_systems(
                Update,
                scroll_parallax.run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_backgrounds(mut commands: Commands) {
    let layer_width = 4000.0;
    let layer_height = 1200.0;

    // === Far layer — dark sky panels (z = -30) ===
    for i in -1..=3 {
        let home_x = i as f32 * layer_width;
        commands.spawn((
            Sprite::from_color(
                Color::srgb(0.06, 0.06, 0.15),
                Vec2::new(layer_width, layer_height),
            ),
            Transform::from_xyz(home_x, 0.0, -30.0),
            ParallaxLayer {
                speed: PARALLAX_FAR_SPEED,
                home_x,
            },
        ));

        // Stars scattered on the far layer
        spawn_stars(&mut commands, home_x, -29.0, PARALLAX_FAR_SPEED);
    }

    // === Mid layer — hills (z = -20) ===
    for i in -1_i32..=4 {
        let base_x = i as f32 * 1200.0;
        for j in 0..5 {
            let home_x = base_x + j as f32 * 260.0 - 500.0;
            let hill_w = 400.0 + ((j * 137 + i.unsigned_abs() as usize * 89) % 250) as f32;
            let hill_h = 120.0 + ((j * 73 + i.unsigned_abs() as usize * 53) % 100) as f32;
            let shade = 0.10 + ((j * 41) % 5) as f32 * 0.01;

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(shade, shade + 0.04, shade + 0.01),
                    Vec2::new(hill_w, hill_h),
                ),
                Transform::from_xyz(home_x, GROUND_Y + hill_h * 0.25, -20.0),
                ParallaxLayer {
                    speed: PARALLAX_MID_SPEED,
                    home_x,
                },
            ));
        }
    }

    // === Near layer — foreground foliage (z = -10) ===
    for i in -1_i32..=6 {
        let base_x = i as f32 * 800.0;
        for j in 0..4 {
            let home_x = base_x + j as f32 * 220.0 - 300.0;
            let bush_w = 60.0 + ((j * 53 + i.unsigned_abs() as usize * 37) % 70) as f32;
            let bush_h = 25.0 + ((j * 29 + i.unsigned_abs() as usize * 19) % 25) as f32;
            let g = 0.12 + ((j * 31) % 8) as f32 * 0.01;

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(0.04, g, 0.04),
                    Vec2::new(bush_w, bush_h),
                ),
                Transform::from_xyz(home_x, GROUND_Y + bush_h * 0.2 - 10.0, -10.0),
                ParallaxLayer {
                    speed: PARALLAX_NEAR_SPEED,
                    home_x,
                },
            ));
        }
    }
}

fn spawn_stars(commands: &mut Commands, base_x: f32, z: f32, speed: f32) {
    for i in 0..15 {
        let seed = (base_x as i32).wrapping_mul(31).wrapping_add(i * 97);
        let home_x = base_x + ((seed.unsigned_abs() % 3800) as f32) - 1900.0;
        let sy = ((seed.wrapping_mul(73).unsigned_abs() % 600) as f32) - 100.0;
        let brightness = 0.3 + ((seed.wrapping_mul(41).unsigned_abs() % 50) as f32) / 100.0;
        let size = 1.5 + ((seed.wrapping_mul(53).unsigned_abs() % 25) as f32) / 10.0;

        commands.spawn((
            Sprite::from_color(
                Color::srgb(brightness, brightness, brightness + 0.15),
                Vec2::new(size, size),
            ),
            Transform::from_xyz(home_x, sy, z),
            ParallaxLayer { speed, home_x },
        ));
    }
}

/// Each frame, offset each layer based on the camera's current X position.
fn scroll_parallax(
    camera_query: Query<&Transform, (With<Camera2d>, Without<ParallaxLayer>)>,
    mut layer_query: Query<(&mut Transform, &ParallaxLayer), Without<Camera2d>>,
) {
    let Ok(camera_tf) = camera_query.single() else {
        return;
    };
    let cam_x = camera_tf.translation.x;

    for (mut tf, layer) in &mut layer_query {
        // Layer world position = home + camera_x * speed
        // A speed of 0 means static; the camera scrolls past it.
        // A speed of 1 means it tracks the camera perfectly.
        tf.translation.x = layer.home_x + cam_x * layer.speed;
    }
}
