use bevy::prelude::*;
use bevy::audio::{AudioSink, PlaybackMode, Volume};

use crate::player::{Grounded, Player, Velocity};
use crate::state::GameState;

/// Marker for the background music entity.
#[derive(Component)]
struct BgMusic;

/// Global game settings — volume, etc.
#[derive(Resource)]
pub struct GameSettings {
    pub master_volume: f32, // 0.0 to 1.0
    pub sfx_volume: f32,    // 0.0 to 1.0
    pub music_volume: f32,  // 0.0 to 1.0
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            master_volume: crate::constants::DEFAULT_MASTER_VOLUME,
            sfx_volume: crate::constants::DEFAULT_SFX_VOLUME,
            music_volume: crate::constants::DEFAULT_MUSIC_VOLUME,
        }
    }
}

impl GameSettings {
    /// Effective volume for sound effects.
    pub fn effective_sfx_volume(&self) -> f32 {
        self.master_volume * self.sfx_volume
    }
    /// Effective volume for background music.
    pub fn effective_music_volume(&self) -> f32 {
        self.master_volume * self.music_volume
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
    pub powerup: Option<Handle<AudioSource>>,
    pub death: Option<Handle<AudioSource>>,
    pub shoot: Option<Handle<AudioSource>>,
    pub music: Option<Handle<AudioSource>>,
}

/// Spawn a one-shot SFX that auto-despawns after playing.
pub fn spawn_sfx(commands: &mut Commands, handle: &Handle<AudioSource>) {
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            ..default()
        },
    ));
}

/// Spawn a one-shot SFX at a specific volume (0.0–1.0).
pub fn spawn_sfx_at_volume(commands: &mut Commands, handle: &Handle<AudioSource>, volume: f32) {
    commands.spawn((
        AudioPlayer::new(handle.clone()),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(volume),
            ..default()
        },
    ));
}

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSettings>()
            .init_resource::<AudioPrevGrounded>()
            .init_resource::<AudioPrevAirborne>()
            .init_resource::<AudioHandles>()
            .add_systems(Startup, load_audio_assets)
            .add_systems(OnEnter(GameState::Playing), start_music)
            .add_systems(OnExit(GameState::Playing), stop_music)
            .add_systems(
                Update,
                (play_jump_sfx, play_land_sfx, update_music_volume)
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
    handles.powerup = Some(asset_server.load("audio/powerup.ogg"));
    handles.death = Some(asset_server.load("audio/death.ogg"));
    handles.shoot = Some(asset_server.load("audio/shoot.ogg"));
    handles.music = Some(asset_server.load("audio/music.ogg"));
}

/// Start looping background music when gameplay begins.
fn start_music(
    mut commands: Commands,
    handles: Res<AudioHandles>,
    settings: Res<GameSettings>,
    existing: Query<Entity, With<BgMusic>>,
) {
    // Don't spawn duplicate music entities
    if !existing.is_empty() {
        return;
    }

    if let Some(ref handle) = handles.music {
        commands.spawn((
            AudioPlayer::new(handle.clone()),
            PlaybackSettings {
                mode: PlaybackMode::Loop,
                volume: Volume::Linear(settings.effective_music_volume() * 0.4), // music quieter than SFX
                ..default()
            },
            BgMusic,
        ));
    }
}

/// Stop music when leaving gameplay.
fn stop_music(mut commands: Commands, query: Query<Entity, With<BgMusic>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn play_jump_sfx(
    mut commands: Commands,
    mut prev_airborne: ResMut<AudioPrevAirborne>,
    query: Query<(&Grounded, &Velocity), With<Player>>,
    handles: Res<AudioHandles>,
) {
    let Ok((grounded, velocity)) = query.single() else {
        return;
    };

    let is_airborne = !grounded.on_ground;

    if is_airborne && !prev_airborne.0 && velocity.0.y > 0.0 {
        if let Some(ref handle) = handles.jump {
            spawn_sfx(&mut commands, handle);
        }
    }

    prev_airborne.0 = is_airborne;
}

fn play_land_sfx(
    mut commands: Commands,
    mut prev_grounded: ResMut<AudioPrevGrounded>,
    query: Query<&Grounded, With<Player>>,
    handles: Res<AudioHandles>,
) {
    let Ok(grounded) = query.single() else {
        return;
    };

    if grounded.on_ground && !prev_grounded.0 {
        if let Some(ref handle) = handles.land {
            spawn_sfx(&mut commands, handle);
        }
    }

    prev_grounded.0 = grounded.on_ground;
}

/// Update the background music volume live when settings change.
fn update_music_volume(
    settings: Res<GameSettings>,
    mut music_query: Query<&mut AudioSink, With<BgMusic>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut sink in &mut music_query {
        sink.set_volume(Volume::Linear(settings.effective_music_volume() * 0.4));
    }
}
