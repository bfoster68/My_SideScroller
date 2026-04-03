use bevy::prelude::*;

use crate::breakable::BreakableGroupCounter;
use crate::checkpoint::{CheckpointData, CheckpointFlag};
use crate::collectibles::Coin;
use crate::constants::*;
use crate::enemies::{Enemy, Projectile};
use crate::hazards::{BoulderSpawner, FallingBoulder, Lava, Saw, Spike, TimedTrap};
use crate::player::{Player, PlayerMovementSet, Score};
use crate::powerups::PowerupKind;
use crate::save::ResumeFromCheckpoint;
use crate::state::GameState;

// Re-export platform types and level_gen types so existing `use crate::level::*` imports work.
pub use crate::platforms::{
    Platform, PlatformSize, PlatformVelocity,
    OneWayPlatform, ConveyorPlatform, IcePlatform, SpringPlatform,
    CrumblingPlatform, CrumbleState,
    spawn_platform, spawn_moving_platform,
};
pub use crate::level_gen::HeightPattern;

/// System set for level reset on play start — runs after player position is set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelResetSet;

/// Tracks how far right we've generated content.
#[derive(Resource)]
pub struct ChunkTracker {
    pub rightmost_ground_x: f32,
    pub rightmost_platform_x: f32,
    pub last_platform_y: f32,
    pub consecutive_ground_gaps: u32,
    pub last_ldtk_chunk_x: f32,
    pub platforms_since_ground_level: u32,
    pub current_pattern: HeightPattern,
    pub pattern_remaining: u32,
    pub pattern_step: u32,
    pub pattern_total: u32,
    pub last_was_fragile: bool,
}

impl Default for ChunkTracker {
    fn default() -> Self {
        Self {
            rightmost_ground_x: SPAWN_X - 200.0,
            rightmost_platform_x: SPAWN_X,
            last_platform_y: GROUND_Y + GROUND_HEIGHT / 2.0 + 100.0,
            consecutive_ground_gaps: 0,
            last_ldtk_chunk_x: SPAWN_X - LDTK_CHUNK_MIN_SPACING * 2.0,
            platforms_since_ground_level: 0,
            current_pattern: HeightPattern::Ascending,
            pattern_remaining: 3,
            pattern_step: 0,
            pattern_total: 3,
            last_was_fragile: false,
        }
    }
}

/// Current difficulty level driven by score. Can exceed 1.0 at high scores.
#[derive(Resource, Default)]
pub struct Difficulty {
    pub value: f32,
}

/// Timer for periodic debug logging (debug builds only).
#[cfg(debug_assertions)]
#[derive(Resource)]
struct GenDebugTimer(Timer);

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkTracker>()
            .init_resource::<Difficulty>();

        #[cfg(debug_assertions)]
        app.insert_resource(GenDebugTimer(Timer::from_seconds(2.0, TimerMode::Repeating)));

        app.add_systems(
                Update,
                (
                    update_difficulty,
                    crate::level_gen::generate_chunks,
                    crate::platforms::despawn_behind_camera,
                    crate::platforms::moving_platform_system,
                    crate::platforms::update_crumbling_platforms,
                )
                    .chain()
                    .before(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            );

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            debug_generation.run_if(in_state(GameState::Playing)),
        );

        app.add_systems(OnEnter(GameState::Playing), reset_level_if_needed
                .in_set(LevelResetSet)
                .after(crate::player::PlayResetSet)
                .after(crate::checkpoint::CheckpointResetSet));
    }
}

#[cfg(debug_assertions)]
fn debug_generation(
    time: Res<Time>,
    mut timer: ResMut<GenDebugTimer>,
    tracker: Res<ChunkTracker>,
    camera_query: Query<&Transform, With<Camera2d>>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    platform_query: Query<Entity, (With<Platform>, Without<ChildOf>)>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() { return; }

    let cam_x = camera_query.single().map(|t| t.translation.x).unwrap_or(0.0);
    let plyr_x = player_query.single().map(|t| t.translation.x).unwrap_or(0.0);
    let plat_count = platform_query.iter().count();

    info!("DBG: cam={:.0} plyr={:.0} trk_gnd={:.0} trk_plat={:.0} plats={} gen_to={:.0}",
        cam_x, plyr_x, tracker.rightmost_ground_x, tracker.rightmost_platform_x,
        plat_count, cam_x.max(plyr_x) + GENERATE_AHEAD);
}

/// Reset level state when starting a new game or resuming from checkpoint.
fn reset_level_if_needed(
    mut commands: Commands,
    mut chunk_tracker: ResMut<ChunkTracker>,
    mut difficulty: ResMut<Difficulty>,
    mut group_counter: ResMut<BreakableGroupCounter>,
    checkpoint: Res<CheckpointData>,
    resume: Option<Res<ResumeFromCheckpoint>>,
    prev_state: Res<crate::state::PreviousGameState>,
    level_entities: Query<
        Entity,
        (
            Or<(
                With<Platform>,
                With<Enemy>,
                With<Coin>,
                With<Spike>,
                With<Saw>,
                With<Lava>,
                With<Projectile>,
                With<PowerupKind>,
                With<BoulderSpawner>,
                With<FallingBoulder>,
                With<TimedTrap>,
                With<CheckpointFlag>,
                With<CrumblingPlatform>,
            )>,
            Without<ChildOf>,
        ),
    >,
) {
    if matches!(
        prev_state.0,
        Some(crate::state::GameState::Paused) | Some(crate::state::GameState::Settings)
    ) {
        return;
    }

    let is_resume = resume.is_some();

    for entity in &level_entities {
        commands.entity(entity).despawn();
    }

    group_counter.0 = 0;

    if is_resume {
        let cx = checkpoint.checkpoint_x;
        chunk_tracker.rightmost_ground_x = cx - 200.0;
        chunk_tracker.rightmost_platform_x = cx - 100.0;
        chunk_tracker.last_platform_y = checkpoint.checkpoint_y.max(GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0);
        chunk_tracker.consecutive_ground_gaps = 0;
        chunk_tracker.last_ldtk_chunk_x = cx - LDTK_CHUNK_MIN_SPACING * 2.0;

        difficulty.value = compute_difficulty(checkpoint.last_checkpoint_score);
    } else {
        *chunk_tracker = ChunkTracker::default();
        difficulty.value = 0.0;
    }
}

/// Update difficulty based on current score.
fn update_difficulty(score: Res<Score>, mut difficulty: ResMut<Difficulty>) {
    if !score.is_changed() {
        return;
    }
    difficulty.value = compute_difficulty(score.value);
}

/// Compute difficulty from a score value.
pub(crate) fn compute_difficulty(score: u32) -> f32 {
    let s = score as f32;
    if s <= DIFFICULTY_PHASE1_SCORE {
        s / DIFFICULTY_PHASE1_SCORE
    } else {
        let excess = s - DIFFICULTY_PHASE1_SCORE;
        1.0 + (1.0 + excess / DIFFICULTY_PHASE2_DIVISOR).ln()
    }
}

/// Extended lerp: 0→1 maps low→high, 1→2.5 maps high→extreme, capped at extreme.
pub(crate) fn lerp_ext(low: f32, high: f32, extreme: f32, d: f32) -> f32 {
    if d <= 1.0 {
        low + (high - low) * d
    } else {
        let t = ((d - 1.0) / 1.5).min(1.0);
        high + (extreme - high) * t
    }
}

pub(crate) fn lerp_ext_f64(low: f64, high: f64, extreme: f64, d: f32) -> f64 {
    if d <= 1.0 {
        low + (high - low) * d as f64
    } else {
        let t = (((d - 1.0) / 1.5) as f64).min(1.0);
        high + (extreme - high) * t
    }
}
