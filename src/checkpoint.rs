use bevy::prelude::*;

use crate::audio::GameSettings;
use crate::constants::*;
use crate::highscore::HighScore;
use crate::player::{Player, Score};
use crate::save::ResumeFromCheckpoint;
use crate::state::GameState;

/// Tracks checkpoint and section progression data.
#[derive(Resource, Default, Clone)]
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

/// System set for checkpoint reset — other OnEnter(Playing) systems can order after this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CheckpointResetSet;

impl Plugin for CheckpointPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CheckpointData>()
            .add_systems(
                Update,
                (check_checkpoint, check_section, section_banner_tick, lerp_sky_color)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Playing), reset_checkpoint_data.in_set(CheckpointResetSet));
    }
}

/// Reset checkpoint data at start of a new game, unless resuming from checkpoint.
fn reset_checkpoint_data(
    mut data: ResMut<CheckpointData>,
    mut commands: Commands,
    flags: Query<Entity, With<CheckpointFlag>>,
    resume: Option<Res<ResumeFromCheckpoint>>,
    prev_state: Res<crate::state::PreviousGameState>,
) {
    // Coming back from Pause or Settings — nothing to reset.
    if matches!(
        prev_state.0,
        Some(crate::state::GameState::Paused) | Some(crate::state::GameState::Settings)
    ) {
        return;
    }

    // If resuming from checkpoint, don't reset — just mark as initialized
    if resume.is_some() {
        data.initialized = true;
        return;
    }

    // New game — always clear checkpoint data (whether from GameOver or fresh Menu start)
    let had_data = data.last_checkpoint_score > 0 || data.initialized;
    *data = CheckpointData::default();
    data.initialized = true;

    if had_data {
        for entity in &flags {
            commands.entity(entity).despawn();
        }
    }
}

/// Check if we've crossed a checkpoint threshold.
fn check_checkpoint(
    mut commands: Commands,
    score: Res<Score>,
    mut data: ResMut<CheckpointData>,
    player_query: Query<&Transform, With<Player>>,
    audio_handles: Option<Res<crate::audio::AudioHandles>>,
    settings: Res<GameSettings>,
    high_score: Res<HighScore>,
    mut active_slot: ResMut<crate::save::ActiveSlot>,
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

    // Auto-save to active slot
    let slot_data = crate::save::slot_data_from_checkpoint(&data);
    if let Some(id) = active_slot.0 {
        crate::save::update_slot(id, &slot_data);
    } else {
        // First checkpoint — create a new slot
        let id = crate::save::create_slot(&slot_data);
        active_slot.0 = Some(id);
    }
    // Also save settings
    crate::save::save_settings(&settings, &high_score);
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
    if (new_section > data.section && data.section > 0) || (new_section > 0 && data.section == 0 && score.value >= SECTION_INTERVAL) {
        // Jump to the detected section to prevent cascading if bonus pushes past next threshold
        data.section = new_section;

        // Award bonus score (capped so it can't push past the next section boundary)
        let next_boundary = (new_section + 1) * SECTION_INTERVAL;
        let max_bonus = next_boundary.saturating_sub(score.value).saturating_sub(1);
        score.value += SECTION_BONUS_SCORE.min(max_bonus);

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
                    TextFont { font_size: FontSize::Px(64.0), ..default() },
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
