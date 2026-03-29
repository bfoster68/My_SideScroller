use bevy::prelude::*;
use rand::Rng;

use crate::breakable::{spawn_breakable_platform, BreakableGroupCounter};
use crate::checkpoint::{CheckpointData, CheckpointFlag, section_colors};
use crate::collectibles::{spawn_coin, spawn_coin_at, spawn_coin_on_moving, Coin};
use crate::constants::*;
use crate::enemies::{
    spawn_charging_enemy, spawn_enemy, spawn_enemy_on_moving, spawn_flying_enemy,
    spawn_flying_ranged_enemy, spawn_shooter_enemy, Enemy, Projectile,
};
use crate::hazards::{
    spawn_boulder_spawner, spawn_lava, spawn_saw, spawn_saw_on_moving, spawn_spike,
    spawn_spike_on_moving, spawn_timed_trap, BoulderSpawner, FallingBoulder, Lava, Saw, Spike,
    TimedTrap,
};
use crate::player::{Player, PlayerMovementSet, Score};
use crate::powerups::{spawn_powerup, PowerupKind};
use crate::save::ResumeFromCheckpoint;
use crate::sprites::GameSprites;
use crate::state::GameState;

/// System set for level reset on play start — runs after player position is set.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct LevelResetSet;

#[derive(Component)]
pub struct Platform;

#[derive(Component)]
pub struct PlatformSize(pub Vec2);

/// Marker for decorative elements attached to platforms.
#[derive(Component)]
pub struct PlatformDecor;

/// Moving platform oscillation data.
#[derive(Component)]
pub struct MovingPlatform {
    pub base_y: f32,
    pub speed: f32,
    pub range: f32,
}

/// Per-frame velocity of a platform (used to carry the player along).
#[derive(Component, Default)]
pub struct PlatformVelocity(pub Vec2);

/// One-way platform: player can jump through from below, drop through with down+jump.
#[derive(Component)]
pub struct OneWayPlatform;

/// Conveyor platform: pushes the player in a direction.
#[derive(Component)]
pub struct ConveyorPlatform {
    pub speed: f32, // positive = right, negative = left
}

/// Ice platform: reduced friction when player stands on it.
#[derive(Component)]
pub struct IcePlatform;

/// Spring/bounce platform: launches player upward on contact.
#[derive(Component)]
pub struct SpringPlatform {
    pub force_multiplier: f32,
}

/// Crumbling platform: starts shaking after player lands, then falls.
#[derive(Component)]
pub struct CrumblingPlatform {
    pub state: CrumbleState,
    pub timer: Timer,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CrumbleState {
    Idle,
    Shaking,
    Falling,
}

/// Height pattern for intentional platform placement variety.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeightPattern {
    Ascending,   // consistent upward steps
    Descending,  // consistent downward steps
    Plateau,     // similar height ±20px (rest area)
    Valley,      // descend then ascend
    Peak,        // ascend then descend
    Freeform,    // random walk (legacy behavior)
}

impl HeightPattern {
    /// Pick the next pattern, biased by current height and difficulty.
    fn pick_next(current_y: f32, difficulty: f32, previous: HeightPattern, rng: &mut impl rand::Rng) -> (HeightPattern, u32) {
        let ground_surface = GROUND_Y + GROUND_HEIGHT / 2.0;
        let ceiling = PLATFORM_CEILING_Y;
        let range = ceiling - ground_surface;
        // How high are we relative to the full range? 0.0 = bottom, 1.0 = top
        let height_ratio = ((current_y - ground_surface) / range).clamp(0.0, 1.0);

        // Build weighted options, avoiding repeat of previous
        let mut options: Vec<(HeightPattern, f64)> = Vec::new();

        // If low, favor ascending; if high, favor descending
        let asc_weight: f64 = (1.0 - height_ratio as f64) * 1.5 + 0.3;
        let desc_weight: f64 = height_ratio as f64 * 1.5 + 0.3;
        let plateau_weight: f64 = (0.8 - 0.3 * difficulty as f64).max(0.1);
        let valley_weight: f64 = 0.4 + 0.6 * difficulty as f64;
        let peak_weight: f64 = 0.4 + 0.6 * difficulty as f64;
        let freeform_weight: f64 = 0.5;

        let candidates: [(HeightPattern, f64); 6] = [
            (HeightPattern::Ascending, asc_weight),
            (HeightPattern::Descending, desc_weight),
            (HeightPattern::Plateau, plateau_weight),
            (HeightPattern::Valley, valley_weight),
            (HeightPattern::Peak, peak_weight),
            (HeightPattern::Freeform, freeform_weight),
        ];

        for (pat, weight) in &candidates {
            if *pat != previous {
                options.push((*pat, *weight));
            }
        }

        // Weighted random selection
        let total: f64 = options.iter().map(|(_, w)| w).sum();
        let mut roll = rng.gen::<f64>() * total;
        let mut chosen = HeightPattern::Freeform;
        for (pat, weight) in &options {
            roll -= weight;
            if roll <= 0.0 {
                chosen = *pat;
                break;
            }
        }

        // Pattern length: Valley/Peak need at least 4, others 2-5
        let len = match chosen {
            HeightPattern::Valley | HeightPattern::Peak => rng.gen_range(4..=6),
            HeightPattern::Plateau => rng.gen_range(2..=4),
            _ => rng.gen_range(2..=5),
        };

        (chosen, len)
    }

    /// Compute the target dy for this platform given where we are in the pattern.
    fn compute_dy(&self, step: u32, total: u32, difficulty: f32, rng: &mut impl rand::Rng) -> f32 {
        let progress = step as f32 / total as f32; // 0.0 to ~1.0
        match self {
            HeightPattern::Ascending => {
                let step_size = 40.0 + 50.0 * difficulty;
                rng.gen_range(step_size * 0.7..step_size * 1.3)
            }
            HeightPattern::Descending => {
                let step_size = 40.0 + 60.0 * difficulty;
                -rng.gen_range(step_size * 0.7..step_size * 1.3)
            }
            HeightPattern::Plateau => {
                rng.gen_range(-20.0..20.0)
            }
            HeightPattern::Valley => {
                // First half descends, second half ascends
                if progress < 0.5 {
                    -rng.gen_range(40.0..80.0)
                } else {
                    rng.gen_range(40.0..80.0)
                }
            }
            HeightPattern::Peak => {
                // First half ascends, second half descends
                if progress < 0.5 {
                    rng.gen_range(40.0..80.0)
                } else {
                    -rng.gen_range(40.0..80.0)
                }
            }
            HeightPattern::Freeform => {
                rng.gen_range(-MAX_JUMP_HEIGHT..MAX_JUMP_HEIGHT)
            }
        }
    }

    /// Get gap multiplier for this pattern (affects horizontal spacing).
    fn gap_multiplier(&self, step: u32, total: u32) -> f32 {
        match self {
            HeightPattern::Ascending => 0.8,   // tighter gaps for climbing
            HeightPattern::Descending => 1.2,  // wider gaps for falling
            HeightPattern::Plateau => 1.0,     // standard
            HeightPattern::Valley | HeightPattern::Peak => {
                let progress = step as f32 / total as f32;
                if progress < 0.5 { 0.9 } else { 1.1 }
            }
            HeightPattern::Freeform => 1.0,
        }
    }

    /// Get width multiplier (narrower on climbs, wider on descents).
    fn width_multiplier(&self) -> f32 {
        match self {
            HeightPattern::Ascending => 0.85,
            HeightPattern::Descending => 1.15,
            HeightPattern::Plateau => 1.2,
            HeightPattern::Valley | HeightPattern::Peak => 1.0,
            HeightPattern::Freeform => 1.0,
        }
    }
}

/// Tracks how far right we've generated content.
#[derive(Resource)]
pub struct ChunkTracker {
    pub rightmost_ground_x: f32,
    pub rightmost_platform_x: f32,
    pub last_platform_y: f32,
    /// How many consecutive ground gaps have been generated.
    pub consecutive_ground_gaps: u32,
    /// X position where the last LDtk chunk was placed (for spacing).
    pub last_ldtk_chunk_x: f32,
    /// Count of platforms since last ground-reachable one.
    pub platforms_since_ground_level: u32,
    /// Current height pattern being generated.
    pub current_pattern: HeightPattern,
    /// Steps remaining in the current pattern.
    pub pattern_remaining: u32,
    /// Current step within the pattern (for progress tracking).
    pub pattern_step: u32,
    /// Total steps of the current pattern (for progress ratio).
    pub pattern_total: u32,
    /// Was the last platform a crumbling or breakable type?
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

/// Current difficulty level (0.0–1.0) driven by score.
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
                    generate_chunks,
                    despawn_behind_camera,
                    moving_platform_system,
                    update_crumbling_platforms,
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
/// On resume, positions the chunk tracker near the checkpoint so terrain
/// generates around the player instead of at the beginning.
fn reset_level_if_needed(
    mut commands: Commands,
    mut chunk_tracker: ResMut<ChunkTracker>,
    mut difficulty: ResMut<Difficulty>,
    mut group_counter: ResMut<BreakableGroupCounter>,
    checkpoint: Res<CheckpointData>,
    resume: Option<Res<ResumeFromCheckpoint>>,
    prev_state: Res<crate::state::PreviousGameState>,
    // Single broad query for all level entities — Without<ChildOf> so recursive despawn
    // handles children automatically without double-despawn warnings.
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
    // Coming back from Pause or Settings — level is still intact, do nothing.
    if matches!(
        prev_state.0,
        Some(crate::state::GameState::Paused) | Some(crate::state::GameState::Settings)
    ) {
        return;
    }

    let is_resume = resume.is_some();

    // Despawn all existing level entities (recursive despawn handles children)
    for entity in &level_entities {
        commands.entity(entity).despawn();
    }

    group_counter.0 = 0;

    if is_resume {
        // Position chunk tracker near the checkpoint so terrain generates around the player
        let cx = checkpoint.checkpoint_x;
        chunk_tracker.rightmost_ground_x = cx - 200.0;
        chunk_tracker.rightmost_platform_x = cx - 100.0;
        chunk_tracker.last_platform_y = checkpoint.checkpoint_y.max(GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0);
        chunk_tracker.consecutive_ground_gaps = 0;
        chunk_tracker.last_ldtk_chunk_x = cx - LDTK_CHUNK_MIN_SPACING * 2.0;

        // Restore difficulty to match the checkpoint score
        difficulty.value = (checkpoint.last_checkpoint_score as f32 / DIFFICULTY_SCORE_MAX).min(1.0);
    } else {
        // Fresh start
        *chunk_tracker = ChunkTracker::default();
        difficulty.value = 0.0;
    }
}

/// Update difficulty based on current score.
fn update_difficulty(score: Res<Score>, mut difficulty: ResMut<Difficulty>) {
    if !score.is_changed() {
        return;
    }
    difficulty.value = (score.value as f32 / DIFFICULTY_SCORE_MAX).min(1.0);
}

/// Lerp a value between low and high based on difficulty.
fn lerp_diff(low: f32, high: f32, d: f32) -> f32 {
    low + (high - low) * d
}

fn lerp_diff_f64(low: f64, high: f64, d: f32) -> f64 {
    low + (high - low) * d as f64
}

/// Generate new ground and platform chunks ahead of the camera.
fn generate_chunks(
    mut commands: Commands,
    mut tracker: ResMut<ChunkTracker>,
    difficulty: Res<Difficulty>,
    camera_query: Query<&Transform, With<Camera2d>>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    game_sprites: Res<GameSprites>,
    checkpoint_data: Res<CheckpointData>,
    mut group_counter: ResMut<BreakableGroupCounter>,
    mut chunk_pool: ResMut<crate::ldtk_chunks::ChunkPool>,
) {
    let Ok(camera_tf) = camera_query.single() else {
        return;
    };
    // Use the further-right of camera or player position, so terrain always
    // generates around the player even before the camera has caught up.
    let mut ref_x = camera_tf.translation.x;
    if let Ok(player_tf) = player_query.single() {
        ref_x = ref_x.max(player_tf.translation.x);
    }
    let generate_to = ref_x + GENERATE_AHEAD;

    let mut rng = rand::thread_rng();
    let d = difficulty.value;

    // Section-based color theming
    let (ground_color, plat_colors) = section_colors(checkpoint_data.section);

    // --- Generate ground segments ---
    let gap_chance = lerp_diff_f64(MIN_GROUND_GAP_CHANCE, MAX_GROUND_GAP_CHANCE, d);
    while tracker.rightmost_ground_x < generate_to {
        let seg_x = tracker.rightmost_ground_x + GROUND_SEGMENT_WIDTH / 2.0;

        // First two segments always solid, then chance of gaps.
        // Never allow more than 2 consecutive gaps — force solid ground so
        // the player always has a path forward.
        let is_gap = tracker.rightmost_ground_x > SPAWN_X + GROUND_SEGMENT_WIDTH
            && tracker.consecutive_ground_gaps < 2
            && rng.gen_bool(gap_chance);

        if !is_gap {
            let _ = spawn_platform(
                &mut commands,
                seg_x,
                GROUND_Y,
                GROUND_SEGMENT_WIDTH,
                GROUND_HEIGHT,
                ground_color,
                true,
            );
            tracker.consecutive_ground_gaps = 0;
        } else {
            // Fill ground gap with lava
            spawn_lava(&mut commands, seg_x, GROUND_SEGMENT_WIDTH, game_sprites.lava.clone());
            tracker.consecutive_ground_gaps += 1;
        }

        tracker.rightmost_ground_x += GROUND_SEGMENT_WIDTH;
    }

    // --- Generate floating platforms ---
    let min_gap = lerp_diff(MIN_PLATFORM_GAP, MAX_PLATFORM_GAP * 0.6, d);
    let max_gap = lerp_diff(MIN_PLATFORM_GAP + 80.0, MAX_PLATFORM_GAP, d);
    let enemy_chance = lerp_diff_f64(MIN_ENEMY_CHANCE, MAX_ENEMY_CHANCE, d);
    let spike_chance = lerp_diff_f64(MIN_SPIKE_CHANCE, MAX_SPIKE_CHANCE, d);
    // Coin chance fills remaining probability (minus bare platform %)
    let bare_chance = (0.25_f64 - 0.10 * d as f64).max(0.10);
    let coin_chance = (1.0 - enemy_chance - spike_chance - bare_chance).max(0.0);
    let moving_chance = MOVING_PLATFORM_CHANCE + 0.15 * d as f64;
    let powerup_chance = POWERUP_SPAWN_CHANCE_MIN + (POWERUP_SPAWN_CHANCE_MAX - POWERUP_SPAWN_CHANCE_MIN) * d as f64;

    let mut color_idx: usize = 0;

    while tracker.rightmost_platform_x < generate_to {
        // --- Try placing an LDtk hand-designed chunk ---
        let ldtk_spacing_ok = (tracker.rightmost_platform_x - tracker.last_ldtk_chunk_x)
            > LDTK_CHUNK_MIN_SPACING;
        if chunk_pool.loaded && ldtk_spacing_ok && rng.gen_bool(LDTK_CHUNK_CHANCE) {
            let last_name = chunk_pool.last_placed.clone();
            if let Some(template) = crate::ldtk_chunks::select_chunk(
                &chunk_pool,
                d,
                tracker.last_platform_y,
                MAX_JUMP_HEIGHT,
                checkpoint_data.section,
                last_name.as_deref(),
                &mut rng,
            ) {
                let chunk_name = template.name.clone();
                let chunk_entry_y = template.entry_y;
                let chunk_exit_y = template.exit_y;
                let chunk_has_ground = template.has_ground;

                let offset_x = tracker.rightmost_platform_x + min_gap;
                let base_y = tracker.last_platform_y - chunk_entry_y;

                let chunk_width = crate::ldtk_chunks::spawn_chunk(
                    &mut commands,
                    template,
                    offset_x,
                    base_y,
                    &game_sprites,
                    checkpoint_data.section,
                    &mut group_counter,
                );

                tracker.rightmost_platform_x = offset_x + chunk_width;
                tracker.last_platform_y = base_y + chunk_exit_y;
                tracker.last_ldtk_chunk_x = offset_x;
                chunk_pool.last_placed = Some(chunk_name);

                if chunk_has_ground {
                    tracker.rightmost_ground_x =
                        tracker.rightmost_ground_x.max(offset_x + chunk_width);
                }
                continue;
            }
        }

        // --- Procedural platform generation (fallback) ---

        // Advance pattern state machine
        if tracker.pattern_remaining == 0 {
            let (new_pattern, new_len) = HeightPattern::pick_next(
                tracker.last_platform_y, d, tracker.current_pattern, &mut rng,
            );
            tracker.current_pattern = new_pattern;
            tracker.pattern_remaining = new_len;
            tracker.pattern_step = 0;
            tracker.pattern_total = new_len;
        }

        // Compute pattern-aware dy and gap
        let gap_mult = tracker.current_pattern.gap_multiplier(tracker.pattern_step, tracker.pattern_total);
        let base_gap = rng.gen_range(min_gap..max_gap);
        let dx = (base_gap * gap_mult).max(min_gap);

        // Constrain upward dy for wider gaps
        let gap_fraction = ((dx - min_gap) / (max_gap - min_gap + 1.0)).clamp(0.0, 1.0);
        let max_rise = MAX_JUMP_HEIGHT * (1.0 - 0.4 * gap_fraction);

        let pattern_dy = tracker.current_pattern.compute_dy(
            tracker.pattern_step, tracker.pattern_total, d, &mut rng,
        );
        // Clamp dy to physics limits
        let dy = pattern_dy.clamp(-MAX_JUMP_HEIGHT, max_rise);

        // Ground surface Y for reference
        let ground_surface = GROUND_Y + GROUND_HEIGHT / 2.0;

        let new_x = tracker.rightmost_platform_x + dx;
        let mut new_y = (tracker.last_platform_y + dy)
            .max(ground_surface + 60.0)
            .min(PLATFORM_CEILING_Y);

        // Safety: every 10 platforms, force one within jump range of the ground
        // so the player always has a way back up if they fall.
        tracker.platforms_since_ground_level += 1;
        if tracker.platforms_since_ground_level >= 10 {
            new_y = ground_surface + rng.gen_range(60.0..MAX_JUMP_HEIGHT);
            tracker.platforms_since_ground_level = 0;
        }

        // Advance pattern step
        tracker.pattern_step += 1;
        tracker.pattern_remaining -= 1;

        // Platform width: pattern-aware + difficulty scaling
        let width_mult = tracker.current_pattern.width_multiplier();
        let min_w = lerp_diff(PLATFORM_MIN_WIDTH, PLATFORM_MIN_WIDTH * 0.7, d) * width_mult;
        let max_w = lerp_diff(PLATFORM_MAX_WIDTH, PLATFORM_MAX_WIDTH * 0.7, d) * width_mult;
        let width = rng.gen_range(min_w.max(60.0)..max_w.max(min_w + 10.0));
        let color = plat_colors[color_idx % plat_colors.len()];
        color_idx += 1;

        // --- Platform type selection (weighted) ---
        let breakable_chance = lerp_diff(BREAKABLE_MIN_CHANCE as f32, BREAKABLE_MAX_CHANCE as f32, d) as f64;
        let crumble_chance = if d > 0.15 {
            lerp_diff_f64(CRUMBLE_MIN_CHANCE, CRUMBLE_MAX_CHANCE, d)
        } else { 0.0 };
        let one_way_chance = ONE_WAY_CHANCE;
        let conveyor_chance = if d > CONVEYOR_START_DIFFICULTY { CONVEYOR_CHANCE } else { 0.0 };
        let ice_chance = if d > ICE_START_DIFFICULTY { ICE_CHANCE } else { 0.0 };
        let spring_chance = if tracker.current_pattern == HeightPattern::Ascending { SPRING_CHANCE } else { 0.0 };

        // Normalize: regular + moving fill remaining weight
        let special_total = breakable_chance + crumble_chance + one_way_chance
            + conveyor_chance + ice_chance + spring_chance;
        let regular_moving_share = (1.0 - special_total).max(0.2);
        let moving_share = moving_chance.min(0.3) * regular_moving_share;

        let type_roll: f64 = rng.gen();
        let mut threshold = 0.0;

        // Prevent back-to-back fragile platforms
        let allow_fragile = !tracker.last_was_fragile;

        // Determine platform type
        threshold += breakable_chance;
        let is_breakable = allow_fragile && type_roll < threshold;

        let is_crumbling = if !is_breakable {
            threshold += crumble_chance;
            allow_fragile && type_roll < threshold
        } else { false };

        let is_one_way = if !is_breakable && !is_crumbling {
            threshold += one_way_chance;
            type_roll < threshold
        } else { false };

        let is_conveyor = if !is_breakable && !is_crumbling && !is_one_way {
            threshold += conveyor_chance;
            type_roll < threshold
        } else { false };

        let is_ice = if !is_breakable && !is_crumbling && !is_one_way && !is_conveyor {
            threshold += ice_chance;
            type_roll < threshold
        } else { false };

        let is_spring = if !is_breakable && !is_crumbling && !is_one_way && !is_conveyor && !is_ice {
            threshold += spring_chance;
            type_roll < threshold
        } else { false };

        let is_moving = if !is_breakable && !is_crumbling && !is_one_way && !is_conveyor
            && !is_ice && !is_spring
        {
            threshold += moving_share;
            type_roll < threshold
        } else { false };

        // Track fragile state
        tracker.last_was_fragile = is_breakable || is_crumbling;
        if is_moving {
            let plat_entity = spawn_moving_platform(
                &mut commands,
                new_x,
                new_y,
                width,
                PLATFORM_HEIGHT,
                color,
                MOVING_PLATFORM_SPEED,
                MOVING_PLATFORM_RANGE,
            );

            // Spawn occupants as children so they move with the platform
            let roll: f64 = rng.gen();
            let saw_chance = spike_chance * 0.5;
            let spike_remaining = spike_chance - saw_chance;
            if roll < enemy_chance && width >= ENEMY_WIDTH * 2.5 {
                let img = game_sprites.enemy_walk.clone();
                commands.entity(plat_entity).with_children(|parent| {
                    spawn_enemy_on_moving(parent, width, img);
                });
            } else if roll < enemy_chance + saw_chance && width >= SAW_SIZE * 3.0 {
                let img = game_sprites.saw.clone();
                commands.entity(plat_entity).with_children(|parent| {
                    spawn_saw_on_moving(parent, width, img);
                });
            } else if roll < enemy_chance + saw_chance + spike_remaining {
                let img = game_sprites.spike.clone();
                commands.entity(plat_entity).with_children(|parent| {
                    spawn_spike_on_moving(parent, img);
                });
            } else if roll < enemy_chance + spike_chance + coin_chance {
                let img = game_sprites.coin.clone();
                commands.entity(plat_entity).with_children(|parent| {
                    spawn_coin_on_moving(parent, img);
                });
            }
        } else if is_breakable {
            // Breakable platform: row of destructible blocks
            let num_blocks = rng.gen_range(BREAKABLE_MIN_BLOCKS..=BREAKABLE_MAX_BLOCKS);
            group_counter.0 += 1;
            let _actual_width = spawn_breakable_platform(
                &mut commands,
                new_x,
                new_y,
                num_blocks,
                group_counter.0,
            );

            // Place entities on breakable platforms (coins only — no enemies/hazards)
            let roll: f64 = rng.gen();
            if roll < coin_chance {
                if rng.gen_bool(powerup_chance) {
                    spawn_powerup(
                        &mut commands, new_x, new_y,
                        game_sprites.powerup_speed.clone(),
                        game_sprites.powerup_jump.clone(),
                        game_sprites.powerup_shield.clone(),
                    );
                } else {
                    spawn_coin(&mut commands, new_x, new_y, game_sprites.coin.clone());
                }
            }
        } else if is_crumbling {
            // Crumbling platform — shakes then falls after player lands
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width, PLATFORM_HEIGHT,
                Color::srgb(0.6, 0.5, 0.4), // sandy/cracked color
                false,
            );
            commands.entity(plat_entity).insert(CrumblingPlatform {
                state: CrumbleState::Idle,
                timer: Timer::from_seconds(CRUMBLE_WARN_TIME, TimerMode::Once),
            });
            // Only coins on crumbling platforms
            if rng.gen_bool(coin_chance) {
                spawn_coin(&mut commands, new_x, new_y, game_sprites.coin.clone());
            }
        } else if is_one_way {
            // One-way platform — can jump through from below
            let ow_color = Color::srgba(
                color.to_srgba().red * 0.8,
                color.to_srgba().green * 0.8,
                color.to_srgba().blue * 1.2,
                0.7,
            );
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width, PLATFORM_HEIGHT,
                ow_color, false,
            );
            commands.entity(plat_entity).insert(OneWayPlatform);
            // Standard entity placement on one-way platforms
            place_standard_entities(
                &mut commands, &game_sprites, &mut rng,
                new_x, new_y, width, d,
                enemy_chance, spike_chance, coin_chance, powerup_chance,
            );
        } else if is_conveyor {
            // Conveyor platform — pushes player left or right
            let direction = if rng.gen_bool(0.5) { 1.0 } else { -1.0 };
            let conv_color = if direction > 0.0 {
                Color::srgb(0.5, 0.7, 0.5) // greenish for right
            } else {
                Color::srgb(0.7, 0.5, 0.5) // reddish for left
            };
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width, PLATFORM_HEIGHT,
                conv_color, false,
            );
            commands.entity(plat_entity).insert(ConveyorPlatform {
                speed: CONVEYOR_SPEED * direction,
            });
            // Only coins on conveyor platforms (hazards would be unfair)
            if rng.gen_bool(coin_chance) {
                spawn_coin(&mut commands, new_x, new_y, game_sprites.coin.clone());
            }
        } else if is_ice {
            // Ice platform — reduced friction
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width, PLATFORM_HEIGHT,
                Color::srgb(0.7, 0.85, 0.95), // blue-white ice
                false,
            );
            commands.entity(plat_entity).insert(IcePlatform);
            place_standard_entities(
                &mut commands, &game_sprites, &mut rng,
                new_x, new_y, width, d,
                enemy_chance, spike_chance, coin_chance, powerup_chance,
            );
        } else if is_spring {
            // Spring platform — bounces player upward
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width.max(80.0), PLATFORM_HEIGHT,
                Color::srgb(0.3, 0.8, 0.3), // green spring
                false,
            );
            commands.entity(plat_entity).insert(SpringPlatform {
                force_multiplier: SPRING_BOUNCE_MULTIPLIER,
            });
            // Coins above spring platforms to reward the bounce
            let coin_y = new_y + COIN_FLOAT_HEIGHT + 60.0;
            spawn_coin_at(&mut commands, new_x, coin_y, game_sprites.coin.clone());
        } else {
            // Regular static platform
            let plat_entity = spawn_platform(
                &mut commands, new_x, new_y, width, PLATFORM_HEIGHT,
                color, false,
            );
            place_standard_entities(
                &mut commands, &game_sprites, &mut rng,
                new_x, new_y, width, d,
                enemy_chance, spike_chance, coin_chance, powerup_chance,
            );

            // Boulder spawner — small chance on platforms at higher difficulty
            if d > 0.4 && rng.gen_bool(0.05) {
                spawn_boulder_spawner(&mut commands, new_x, new_y, game_sprites.boulder.clone(), game_sprites.boulder_warning.clone());
            }
            let _ = plat_entity; // suppress unused warning
        }

        // Coin formations between platforms (Sonic-style variety).
        // Minimum Y ensures coins never spawn below or inside platforms.
        if rng.gen_bool(0.4) {
            let prev_x = tracker.rightmost_platform_x;
            let prev_y = tracker.last_platform_y;
            let min_coin_y = prev_y.max(new_y) + PLATFORM_HEIGHT / 2.0 + COIN_SIZE;
            let pattern: u32 = rng.gen_range(0..5);

            match pattern {
                0 => {
                    // Arc of coins — parabolic path above both platforms
                    let count = rng.gen_range(4..7);
                    let arc_base = prev_y.max(new_y) + COIN_FLOAT_HEIGHT;
                    for i in 0..count {
                        let t = (i as f32 + 1.0) / (count as f32 + 1.0);
                        let cx = prev_x + (new_x - prev_x) * t;
                        let arc_h = 80.0 * (4.0 * t * (1.0 - t));
                        let cy = (arc_base + arc_h).max(min_coin_y);
                        spawn_coin_at(&mut commands, cx, cy, game_sprites.coin.clone());
                    }
                }
                1 => {
                    // Horizontal line at jump height above destination platform
                    let count = rng.gen_range(3..6);
                    let line_y = new_y + COIN_FLOAT_HEIGHT;
                    let spread = (count as f32 - 1.0) * 30.0;
                    let start_x = new_x - spread / 2.0;
                    for i in 0..count {
                        spawn_coin_at(
                            &mut commands,
                            start_x + i as f32 * 30.0,
                            line_y.max(min_coin_y),
                            game_sprites.coin.clone(),
                        );
                    }
                }
                2 => {
                    // Vertical stack above platform — reward for precise landing
                    let count = rng.gen_range(3..5);
                    let stack_x = new_x;
                    let base = new_y + COIN_FLOAT_HEIGHT;
                    for i in 0..count {
                        spawn_coin_at(
                            &mut commands,
                            stack_x,
                            (base + i as f32 * 30.0).max(min_coin_y),
                            game_sprites.coin.clone(),
                        );
                    }
                }
                3 => {
                    // Diagonal trail — always ascending between platforms
                    let count = rng.gen_range(3..6);
                    for i in 0..count {
                        let t = (i as f32 + 1.0) / (count as f32 + 1.0);
                        let cx = prev_x + (new_x - prev_x) * t;
                        let cy = prev_y.min(new_y) + COIN_FLOAT_HEIGHT
                            + i as f32 * 25.0;
                        spawn_coin_at(&mut commands, cx, cy.max(min_coin_y), game_sprites.coin.clone());
                    }
                }
                _ => {
                    // Diamond/ring shape — floating between platforms
                    let mid_x = (prev_x + new_x) / 2.0;
                    let mid_y = (prev_y.max(new_y) + 60.0).max(min_coin_y + 30.0);
                    let r = 25.0;
                    spawn_coin_at(&mut commands, mid_x, mid_y + r, game_sprites.coin.clone());
                    spawn_coin_at(&mut commands, mid_x + r, mid_y, game_sprites.coin.clone());
                    spawn_coin_at(&mut commands, mid_x, (mid_y - r).max(min_coin_y), game_sprites.coin.clone());
                    spawn_coin_at(&mut commands, mid_x - r, mid_y, game_sprites.coin.clone());
                }
            }
        }

        tracker.rightmost_platform_x = new_x;
        tracker.last_platform_y = new_y;
    }
}

/// Helper: place standard entities (enemies/hazards/coins) on a platform.
fn place_standard_entities(
    commands: &mut Commands,
    game_sprites: &GameSprites,
    rng: &mut impl rand::Rng,
    x: f32,
    y: f32,
    width: f32,
    d: f32,
    enemy_chance: f64,
    spike_chance: f64,
    coin_chance: f64,
    powerup_chance: f64,
) {
    let roll: f64 = rng.gen();
    let saw_chance = spike_chance * 0.5;
    let spike_remaining = spike_chance - saw_chance;
    let charging_pct = if d > CHARGING_START_DIFFICULTY { CHARGING_SPAWN_WEIGHT } else { 0.0 };
    let flying_ranged_pct = if d > FLYING_RANGED_START_DIFFICULTY { FLYING_RANGED_SPAWN_WEIGHT } else { 0.0 };
    let remaining = 1.0 - charging_pct - flying_ranged_pct;
    let base_total = ENEMY_WALKING_WEIGHT + ENEMY_FLYING_WEIGHT + ENEMY_SHOOTER_WEIGHT;
    let walking_chance = enemy_chance * (ENEMY_WALKING_WEIGHT * remaining / base_total);
    let flying_chance = enemy_chance * (ENEMY_FLYING_WEIGHT * remaining / base_total);
    let shooter_chance = enemy_chance * (ENEMY_SHOOTER_WEIGHT * remaining / base_total);
    let charging_chance = enemy_chance * charging_pct;
    let flying_ranged_chance = enemy_chance * flying_ranged_pct;

    if roll < walking_chance && width >= ENEMY_WIDTH * 2.5 {
        spawn_enemy(commands, x, y, width, game_sprites.enemy_walk.clone());
    } else if roll < walking_chance + flying_chance {
        spawn_flying_enemy(commands, x, y, game_sprites.enemy_fly.clone());
    } else if roll < walking_chance + flying_chance + shooter_chance {
        spawn_shooter_enemy(commands, x, y, game_sprites.enemy_shooter.clone());
    } else if roll < walking_chance + flying_chance + shooter_chance + charging_chance && width >= CHARGING_ENEMY_WIDTH * 2.5 {
        spawn_charging_enemy(commands, x, y, width, game_sprites.enemy_charging.clone());
    } else if roll < walking_chance + flying_chance + shooter_chance + charging_chance + flying_ranged_chance {
        spawn_flying_ranged_enemy(commands, x, y, game_sprites.enemy_flying_ranged.clone());
    } else if roll < enemy_chance + saw_chance && width >= SAW_SIZE * 3.0 {
        spawn_saw(commands, x, y, width, game_sprites.saw.clone());
    } else if roll < enemy_chance + saw_chance + spike_remaining {
        if d > 0.3 && rng.gen_bool(0.3) {
            spawn_timed_trap(commands, x, y, game_sprites.timed_trap.clone());
        } else {
            spawn_spike(commands, x, y, game_sprites.spike.clone());
        }
    } else if roll < enemy_chance + spike_chance + coin_chance {
        if rng.gen_bool(powerup_chance) {
            spawn_powerup(
                commands, x, y,
                game_sprites.powerup_speed.clone(),
                game_sprites.powerup_jump.clone(),
                game_sprites.powerup_shield.clone(),
            );
        } else {
            spawn_coin(commands, x, y, game_sprites.coin.clone());
        }
    }
}

/// Update crumbling platforms: shake when triggered, then fall and despawn.
fn update_crumbling_platforms(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<(&Transform, &crate::player::Grounded), With<Player>>,
    mut query: Query<(Entity, &mut Transform, &mut CrumblingPlatform, &PlatformSize), Without<Player>>,
) {
    let player_on = player_query.single().ok().and_then(|(pt, grounded)| {
        if grounded.on_ground { Some(pt.translation) } else { None }
    });

    for (entity, mut transform, mut crumble, size) in &mut query {
        match crumble.state {
            CrumbleState::Idle => {
                // Check if player is standing on this platform
                if let Some(pp) = player_on {
                    let half_w = size.0.x / 2.0 + PLAYER_WIDTH / 2.0;
                    let on_top = (pp.x - transform.translation.x).abs() < half_w
                        && (pp.y - transform.translation.y) > 0.0
                        && (pp.y - transform.translation.y) < size.0.y + PLAYER_HEIGHT;
                    if on_top {
                        crumble.state = CrumbleState::Shaking;
                        crumble.timer = Timer::from_seconds(CRUMBLE_WARN_TIME, TimerMode::Once);
                    }
                }
            }
            CrumbleState::Shaking => {
                crumble.timer.tick(time.delta());
                // Visual shake
                let shake = (time.elapsed_secs() * 40.0).sin() * 2.0;
                transform.translation.x += shake * time.delta_secs() * 10.0;
                if crumble.timer.just_finished() {
                    crumble.state = CrumbleState::Falling;
                    crumble.timer = Timer::from_seconds(CRUMBLE_FALL_TIME, TimerMode::Once);
                }
            }
            CrumbleState::Falling => {
                crumble.timer.tick(time.delta());
                transform.translation.y += GRAVITY * 0.5 * time.delta_secs();
                // Fade out
                if crumble.timer.just_finished() || transform.translation.y < FALL_LIMIT {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

/// Despawn entities that have fallen far behind the camera.
/// Uses Transform (not GlobalTransform) because all queried entities are root
/// entities (Without<ChildOf>), so Transform IS their world position.  Using
/// GlobalTransform here would cause newly-spawned entities to be despawned
/// immediately because GlobalTransform isn't propagated until PostUpdate.
/// Uses the minimum of camera and player X to avoid despawning terrain around the
/// player when the camera hasn't caught up yet (e.g., after respawn).
fn despawn_behind_camera(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera2d>>,
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    query: Query<
        (Entity, &Transform),
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
            )>,
            Without<ChildOf>,
        ),
    >,
) {
    let Ok(camera_tf) = camera_query.single() else {
        return;
    };
    // Use the leftmost of camera or player position so we never despawn
    // terrain around the player before the camera catches up.
    let mut ref_x = camera_tf.translation.x;
    if let Ok(player_tf) = player_query.single() {
        ref_x = ref_x.min(player_tf.translation.x);
    }
    let cutoff = ref_x - DESPAWN_BEHIND;

    for (entity, tf) in &query {
        if tf.translation.x < cutoff {
            commands.entity(entity).despawn();
        }
    }
}

/// Oscillate moving platforms up and down, storing the per-frame delta.
fn moving_platform_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovingPlatform, &mut PlatformVelocity)>,
) {
    let dt = time.delta_secs();
    for (mut transform, moving, mut plat_vel) in &mut query {
        let old_y = transform.translation.y;
        // Use cumulative delta to avoid discontinuities on unpause
        let t = time.elapsed_secs();
        let new_y = moving.base_y + (t * moving.speed).sin() * moving.range;
        let delta_y = new_y - old_y;
        // Store as velocity (per second) so consumers can multiply by dt
        plat_vel.0.y = if dt > 0.0 { delta_y / dt } else { 0.0 };
        transform.translation.y = new_y;
    }
}

/// Spawn a platform with visual surface highlights. Returns the entity.
pub fn spawn_platform(
    commands: &mut Commands,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    is_ground: bool,
) -> Entity {
    let highlight = if is_ground {
        Color::srgb(0.45, 0.55, 0.3)
    } else {
        lighten(color, 0.15)
    };
    let highlight_h = if is_ground { 6.0 } else { 3.0 };
    let shadow_h = 2.0;

    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        PlatformSize(Vec2::new(width, height)),
    )).with_children(|parent| {
        // Top surface highlight
        parent.spawn((
            Sprite::from_color(highlight, Vec2::new(width, highlight_h)),
            Transform::from_xyz(0.0, height / 2.0 - highlight_h / 2.0, 0.1),
            PlatformDecor,
        ));
        // Bottom edge shadow
        parent.spawn((
            Sprite::from_color(darken(color, 0.15), Vec2::new(width, shadow_h)),
            Transform::from_xyz(0.0, -height / 2.0 + shadow_h / 2.0, 0.1),
            PlatformDecor,
        ));
        // Side shading for floating platforms
        if !is_ground {
            let edge_w = 3.0;
            parent.spawn((
                Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
                Transform::from_xyz(-width / 2.0 + edge_w / 2.0, 0.0, 0.1),
                PlatformDecor,
            ));
            parent.spawn((
                Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
                Transform::from_xyz(width / 2.0 - edge_w / 2.0, 0.0, 0.1),
                PlatformDecor,
            ));
        }
    }).id()
}

/// Spawn a platform that also has the MovingPlatform component.
pub fn spawn_moving_platform(
    commands: &mut Commands,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    speed: f32,
    range: f32,
) -> Entity {
    let highlight = lighten(color, 0.15);
    let edge_w = 3.0;

    let entity = commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        PlatformSize(Vec2::new(width, height)),
        MovingPlatform {
            base_y: y,
            speed,
            range,
        },
        PlatformVelocity::default(),
    )).with_children(|parent| {
        // Highlight (relative to parent)
        parent.spawn((
            Sprite::from_color(highlight, Vec2::new(width, 3.0)),
            Transform::from_xyz(0.0, height / 2.0 - 1.5, 0.1),
            PlatformDecor,
        ));
        // Shadow
        parent.spawn((
            Sprite::from_color(darken(color, 0.15), Vec2::new(width, 2.0)),
            Transform::from_xyz(0.0, -height / 2.0 + 1.0, 0.1),
            PlatformDecor,
        ));
        // Side edges
        parent.spawn((
            Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
            Transform::from_xyz(-width / 2.0 + edge_w / 2.0, 0.0, 0.1),
            PlatformDecor,
        ));
        parent.spawn((
            Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
            Transform::from_xyz(width / 2.0 - edge_w / 2.0, 0.0, 0.1),
            PlatformDecor,
        ));
    }).id();

    entity
}

// ---------------------------------------------------------------------------
// Color helpers
// ---------------------------------------------------------------------------

fn lighten(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgb(
        (srgba.red + amount).min(1.0),
        (srgba.green + amount).min(1.0),
        (srgba.blue + amount).min(1.0),
    )
}

fn darken(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgb(
        (srgba.red - amount).max(0.0),
        (srgba.green - amount).max(0.0),
        (srgba.blue - amount).max(0.0),
    )
}
