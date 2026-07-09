//! Simple CPU-driven parallax mountain background.
//!
//! Replaces the old GPU compute-shader background (which relied on Bevy's
//! render-graph API, removed in Bevy 0.19). Each layer is a row of tall
//! colored sprites whose jagged tops form a rolling-hill silhouette. Layers
//! scroll horizontally at different fractional rates for a parallax effect and
//! wrap infinitely around the camera. Works on all platforms (native + WASM);
//! the dynamic sky color is still driven by `ClearColor` in `checkpoint.rs`.

use bevy::prelude::*;

/// Z depth for the background — behind everything else.
const BG_Z: f32 = -50.0;

/// Per-tile parallax data used to reposition the sprite each frame.
#[derive(Component)]
struct ParallaxTile {
    /// Scroll rate relative to the camera (0 = distant/slow, 1 = locked on).
    factor: f32,
    /// Horizontal distance between adjacent tiles in this layer.
    spacing: f32,
    /// Number of tiles in this layer (for infinite wrap).
    count: u32,
    /// This tile's index within its layer.
    index: u32,
    /// Vertical offset from the camera center (includes per-tile jitter).
    base_y: f32,
}

/// Static description of one parallax layer.
struct LayerCfg {
    factor: f32,
    spacing: f32,
    count: u32,
    center_y: f32,
    height: f32,
    z: f32,
    color: Color,
}

/// Far → near. Distant layers are hazier/bluer and scroll slower.
const LAYERS: &[LayerCfg] = &[
    LayerCfg {
        factor: 0.10,
        spacing: 400.0,
        count: 6,
        center_y: -230.0,
        height: 420.0,
        z: BG_Z,
        color: Color::srgb(0.30, 0.35, 0.48),
    },
    LayerCfg {
        factor: 0.22,
        spacing: 340.0,
        count: 7,
        center_y: -270.0,
        height: 420.0,
        z: BG_Z + 1.0,
        color: Color::srgb(0.22, 0.29, 0.38),
    },
    LayerCfg {
        factor: 0.40,
        spacing: 280.0,
        count: 8,
        center_y: -300.0,
        height: 420.0,
        z: BG_Z + 2.0,
        color: Color::srgb(0.15, 0.21, 0.27),
    },
];

pub struct MountainBgPlugin;

impl Plugin for MountainBgPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_mountain_bg)
            .add_systems(Update, update_parallax);
    }
}

/// Deterministic per-tile vertical jitter so the silhouette isn't uniform.
fn tile_jitter(index: u32) -> f32 {
    // Cheap hash → [-40, 40]
    let h = (index.wrapping_mul(2654435761) >> 16) % 100;
    (h as f32 / 100.0 - 0.5) * 80.0
}

fn setup_mountain_bg(mut commands: Commands) {
    for layer in LAYERS {
        for index in 0..layer.count {
            let base_y = layer.center_y + tile_jitter(index);
            commands.spawn((
                Sprite::from_color(
                    layer.color,
                    // Slight overlap avoids seams between adjacent tiles.
                    Vec2::new(layer.spacing + 2.0, layer.height),
                ),
                Transform::from_xyz(0.0, base_y, layer.z),
                ParallaxTile {
                    factor: layer.factor,
                    spacing: layer.spacing,
                    count: layer.count,
                    index,
                    base_y,
                },
            ));
        }
    }
}

/// Reposition each tile around the camera every frame: scroll at the layer's
/// parallax rate and wrap infinitely so the hills always fill the view.
fn update_parallax(
    camera_query: Query<&Transform, (With<Camera2d>, Without<ParallaxTile>)>,
    mut tiles: Query<(&mut Transform, &ParallaxTile)>,
) {
    let Ok(cam_tf) = camera_query.single() else {
        return;
    };
    let cam_x = cam_tf.translation.x;
    let cam_y = cam_tf.translation.y;

    for (mut tf, tile) in &mut tiles {
        let total = tile.spacing * tile.count as f32;
        let raw = tile.index as f32 * tile.spacing - cam_x * tile.factor;
        // Center the wrapped range on the camera.
        let wrapped = raw.rem_euclid(total) - total / 2.0;
        tf.translation.x = cam_x + wrapped;
        tf.translation.y = cam_y + tile.base_y;
    }
}
