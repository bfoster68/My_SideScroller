use bevy::prelude::*;

use crate::constants::*;
use crate::player::{Player, Score};
use crate::state::GameState;

/// Tracks checkpoint and section progression data.
#[derive(Resource, Default)]
pub struct CheckpointData {
    pub last_checkpoint_score: u32,
    pub checkpoint_x: f32,
    pub checkpoint_y: f32,
    pub section: u32,
    pub initialized: bool,
}

/// Marker for checkpoint flag entities in the world.
#[derive(Component)]
pub struct CheckpointFlag;

/// Active section banner timer — inserted as a resource when a section transition occurs.
#[derive(Resource)]
pub struct SectionBanner {
    pub section: u32,
    pub timer: Timer,
}

/// Section color themes for platform generation.
pub fn section_colors(section: u32) -> (Color, [Color; 4]) {
    match section % 4 {
        0 => (
            // Earthy greens/browns (default)
            Color::srgb(0.35, 0.28, 0.18),
            [
                Color::srgb(0.28, 0.42, 0.28),
                Color::srgb(0.30, 0.35, 0.45),
                Color::srgb(0.45, 0.35, 0.25),
                Color::srgb(0.35, 0.40, 0.30),
            ],
        ),
        1 => (
            // Icy blues/whites
            Color::srgb(0.25, 0.30, 0.40),
            [
                Color::srgb(0.35, 0.50, 0.65),
                Color::srgb(0.45, 0.55, 0.70),
                Color::srgb(0.30, 0.45, 0.60),
                Color::srgb(0.40, 0.50, 0.55),
            ],
        ),
        2 => (
            // Volcanic reds/oranges
            Color::srgb(0.35, 0.20, 0.15),
            [
                Color::srgb(0.55, 0.25, 0.20),
                Color::srgb(0.50, 0.30, 0.15),
                Color::srgb(0.60, 0.25, 0.18),
                Color::srgb(0.45, 0.28, 0.20),
            ],
        ),
        _ => (
            // Dark purples
            Color::srgb(0.20, 0.15, 0.30),
            [
                Color::srgb(0.35, 0.25, 0.50),
                Color::srgb(0.30, 0.20, 0.45),
                Color::srgb(0.40, 0.28, 0.55),
                Color::srgb(0.32, 0.22, 0.48),
            ],
        ),
    }
}

/// Sky colors for each section (used with ClearColor).
pub fn section_sky_color(section: u32) -> Color {
    match section % 4 {
        0 => Color::srgb(0.1, 0.1, 0.2),   // dark blue (default)
        1 => Color::srgb(0.12, 0.15, 0.25), // icy night
        2 => Color::srgb(0.18, 0.08, 0.05), // volcanic dark
        _ => Color::srgb(0.10, 0.05, 0.18), // purple night
    }
}

pub struct CheckpointPlugin;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointData>()
            .add_systems(
                Update,
                (check_checkpoint, check_section, section_banner_tick, lerp_sky_color)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Playing), reset_checkpoint_data);
    }
}

/// Reset checkpoint data at start of a new game.
fn reset_checkpoint_data(
    mut data: ResMut<CheckpointData>,
    mut commands: Commands,
    flags: Query<Entity, With<CheckpointFlag>>,
) {
    // Only reset if we're coming from GameOver (data was initialized)
    if data.initialized {
        *data = CheckpointData::default();
        for entity in &flags {
            commands.entity(entity).despawn();
        }
    }
    data.initialized = true;
}

/// Check if we've crossed a checkpoint threshold.
fn check_checkpoint(
    mut commands: Commands,
    score: Res<Score>,
    mut data: ResMut<CheckpointData>,
    player_query: Query<&Transform, With<Player>>,
    audio_handles: Option<Res<crate::audio::AudioHandles>>,
) {
    if score.value < data.last_checkpoint_score + CHECKPOINT_INTERVAL {
        return;
    }

    let Ok(player_tf) = player_query.single() else { return };

    // Update checkpoint position
    data.checkpoint_x = player_tf.translation.x;
    data.checkpoint_y = player_tf.translation.y;
    data.last_checkpoint_score = (score.value / CHECKPOINT_INTERVAL) * CHECKPOINT_INTERVAL;

    // Spawn a visual flag marker
    commands.spawn((
        Sprite::from_color(Color::srgb(1.0, 0.8, 0.0), Vec2::new(8.0, 32.0)),
        Transform::from_xyz(data.checkpoint_x, data.checkpoint_y + 40.0, 0.9),
        CheckpointFlag,
    ));

    // Play collect SFX as checkpoint sound
    if let Some(ref handles) = audio_handles {
        if let Some(ref handle) = handles.collect {
            crate::audio::spawn_sfx(&mut commands, handle);
        }
    }
}

/// Check if we've entered a new section.
fn check_section(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut data: ResMut<CheckpointData>,
    banner: Option<Res<SectionBanner>>,
) {
    if banner.is_some() {
        return; // Already showing a banner
    }

    let new_section = score.value / SECTION_INTERVAL;
    if new_section > data.section && data.section > 0 || (new_section > 0 && data.section == 0 && score.value >= SECTION_INTERVAL) {
        data.section = new_section;

        // Award bonus score
        score.value += SECTION_BONUS_SCORE;

        // Insert banner resource
        commands.insert_resource(SectionBanner {
            section: new_section,
            timer: Timer::from_seconds(SECTION_BANNER_DURATION, TimerMode::Once),
        });
    }
}

/// Tick the section banner timer and spawn/despawn UI.
fn section_banner_tick(
    mut commands: Commands,
    time: Res<Time>,
    mut banner: Option<ResMut<SectionBanner>>,
    banner_ui: Query<Entity, With<crate::hud::SectionBannerUi>>,
) {
    let Some(ref mut banner) = banner else {
        // No banner active — make sure UI is gone
        for entity in &banner_ui {
            commands.entity(entity).despawn();
        }
        return;
    };

    banner.timer.tick(time.delta());

    // Spawn UI if not yet spawned
    if banner_ui.is_empty() {
        commands
            .spawn((
                crate::hud::SectionBannerUi,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new(format!("SECTION {}!", banner.section + 1)),
                    TextFont { font_size: 64.0, ..default() },
                    TextColor(Color::srgb(1.0, 0.9, 0.3)),
                ));
            });
    }

    if banner.timer.is_finished() {
        for entity in &banner_ui {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<SectionBanner>();
    }
}

/// Smoothly lerp the sky color to match the current section.
fn lerp_sky_color(
    data: Res<CheckpointData>,
    mut clear_color: ResMut<ClearColor>,
    time: Res<Time>,
) {
    let target = section_sky_color(data.section);
    let current = clear_color.0.to_srgba();
    let target_srgba = target.to_srgba();
    let t = (time.delta_secs() * 0.5).min(1.0); // slow lerp

    clear_color.0 = Color::srgb(
        current.red + (target_srgba.red - current.red) * t,
        current.green + (target_srgba.green - current.green) * t,
        current.blue + (target_srgba.blue - current.blue) * t,
    );
}
