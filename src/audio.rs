use bevy::prelude::*;
use bevy::audio::Volume;

use crate::player::{Grounded, Player, Velocity};
use crate::state::GameState;

/// Global game settings — volume, etc. Future sprints can add more fields.
#[derive(Resource)]
pub struct GameSettings {
    pub master_volume: f32, // 0.0 to 1.0
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: 1.0,
        }
    }
}

/// Tracks previous grounded state for detecting landing.
#[derive(Resource, Default)]
struct AudioPrevGrounded(bool);

/// Tracks previous airborne state for detecting jump.
#[derive(Resource, Default)]
struct AudioPrevAirborne(bool);

/// Handles for loaded audio assets.
#[derive(Resource, Default)]
pub struct AudioHandles {
    pub jump: Option<Handle<AudioSource>>,
    pub land: Option<Handle<AudioSource>>,
    pub collect: Option<Handle<AudioSource>>,
    pub hit: Option<Handle<AudioSource>>,
    pub music: Option<Handle<AudioSource>>,
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .init_resource::<AudioPrevGrounded>()
            .init_resource::<AudioPrevAirborne>()
            .init_resource::<AudioHandles>()
            .add_systems(Startup, load_audio_assets)
            .add_systems(
                Update,
                (play_jump_sfx, play_land_sfx)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Attempt to load audio files from assets/audio/. Silently continues if files
/// are missing — the game works fine without audio assets.
fn load_audio_assets(asset_server: Res<AssetServer>, mut handles: ResMut<AudioHandles>) {
    // These will silently fail to load if the files don't exist.
    // The play_* systems check for Some before playing.
    handles.jump = Some(asset_server.load("audio/jump.ogg"));
    handles.land = Some(asset_server.load("audio/land.ogg"));
    handles.collect = Some(asset_server.load("audio/collect.ogg"));
    handles.hit = Some(asset_server.load("audio/hit.ogg"));
    handles.music = Some(asset_server.load("audio/music.ogg"));
}

fn play_jump_sfx(
    mut commands: Commands,
    mut prev_airborne: ResMut<AudioPrevAirborne>,
    query: Query<(&Grounded, &Velocity), With<Player>>,
    handles: Res<AudioHandles>,
    settings: Res<GameSettings>,
) {
    let Ok((grounded, velocity)) = query.single() else {
        return;
    };

    let is_airborne = !grounded.on_ground;

    if is_airborne && !prev_airborne.0 && velocity.0.y > 0.0 {
        if let Some(ref handle) = handles.jump {
            commands.spawn((
                AudioPlayer::new(handle.clone()),
                PlaybackSettings {
                    volume: Volume::Linear(settings.master_volume),
                    ..default()
                },
            ));
        }
    }

    prev_airborne.0 = is_airborne;
}

fn play_land_sfx(
    mut commands: Commands,
    mut prev_grounded: ResMut<AudioPrevGrounded>,
    query: Query<&Grounded, With<Player>>,
    handles: Res<AudioHandles>,
    settings: Res<GameSettings>,
) {
    let Ok(grounded) = query.single() else {
        return;
    };

    if grounded.on_ground && !prev_grounded.0 {
        if let Some(ref handle) = handles.land {
            commands.spawn((
                AudioPlayer::new(handle.clone()),
                PlaybackSettings {
                    volume: Volume::Linear(settings.master_volume),
                    ..default()
                },
            ));
        }
    }

    prev_grounded.0 = grounded.on_ground;
}
