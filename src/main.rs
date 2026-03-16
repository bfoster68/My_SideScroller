mod animation;
mod audio;
mod camera;
mod collectibles;
mod constants;
mod enemies;
mod hazards;
mod health;
mod highscore;
mod hud;
mod level;
mod parallax;
mod particles;
mod player;
mod powerups;
mod sprites;
mod state;
mod transition;

use bevy::{prelude::*, window::WindowResolution};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Side-Scroller".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.2)))
        // Game plugins
        .add_plugins(state::StatePlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(level::LevelPlugin)
        .add_plugins(health::HealthPlugin)
        .add_plugins(hud::HudPlugin)
        .add_plugins(animation::AnimationPlugin)
        .add_plugins(parallax::ParallaxPlugin)
        .add_plugins(particles::ParticlesPlugin)
        .add_plugins(enemies::EnemiesPlugin)
        .add_plugins(collectibles::CollectiblesPlugin)
        .add_plugins(hazards::HazardsPlugin)
        .add_plugins(powerups::PowerupsPlugin)
        .add_plugins(sprites::SpritesPlugin)
        .add_plugins(audio::GameAudioPlugin)
        .add_plugins(highscore::HighScorePlugin)
        .add_plugins(transition::TransitionPlugin)
        .run();
}
