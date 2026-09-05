mod animation;
mod audio;
mod breakable;
mod camera;
mod checkpoint;
mod collectibles;
mod constants;
mod debug;
mod enemies;
mod enemy_types;
mod hazards;
mod health;
mod highscore;
mod hud;
mod input;
mod ldtk_chunks;
mod level;
mod level_gen;
mod mountain_bg;
mod particles;
mod platforms;
mod player;
mod powerups;
mod save;
mod spatial;
mod sprites;
mod state;
mod touch;
mod transition;

#[cfg(test)]
mod tests;

use bevy::{prelude::*, window::WindowResolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Side-Scroller".into(),
                resolution: WindowResolution::new(1280, 720),
                canvas: Some("#bevy-canvas".into()),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.2)))
        // Core infrastructure (order matters: save loads data, input runs in PreUpdate)
        .add_plugins(input::InputPlugin)
        .add_plugins(touch::TouchPlugin)
        .add_plugins(save::SavePlugin)
        // Game plugins
        .add_plugins(state::StatePlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(ldtk_chunks::LdtkChunksPlugin)
        .add_plugins(level::LevelPlugin)
        .add_plugins(breakable::BreakablePlugin)
        .add_plugins(health::HealthPlugin)
        .add_plugins(hud::HudPlugin)
        .add_plugins(animation::AnimationPlugin)
        .add_plugins(mountain_bg::MountainBgPlugin)
        .add_plugins(particles::ParticlesPlugin)
        .add_plugins(enemies::EnemiesPlugin)
        .add_plugins(collectibles::CollectiblesPlugin)
        .add_plugins(hazards::HazardsPlugin)
        .add_plugins(powerups::PowerupsPlugin)
        .add_plugins(spatial::SpatialPlugin)
        .add_plugins(sprites::SpritesPlugin)
        .add_plugins(audio::GameAudioPlugin)
        .add_plugins(highscore::HighScorePlugin)
        .add_plugins(transition::TransitionPlugin)
        .add_plugins(checkpoint::CheckpointPlugin)
        .add_plugins(debug::DebugPlugin)
        .run();
}
