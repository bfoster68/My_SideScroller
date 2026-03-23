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
}

impl Default for ChunkTracker {
    fn default() -> Self {
        Self {
            rightmost_ground_x: SPAWN_X - 200.0,
            rightmost_platform_x: SPAWN_X,
            last_platform_y: GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0,
            consecutive_ground_gaps: 0,
            last_ldtk_chunk_x: SPAWN_X - LDTK_CHUNK_MIN_SPACING * 2.0,
            platforms_since_ground_level: 0,
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
                )
                    .chain()
                    .before(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            );

        #[cfg(debug_assertions)]
        app.add_systems(
            Update,
            debug_generation.run_if(in_state(GameState::Playing)),
        )
            .add_systems(OnEnter(GameState::Playing), reset_level_if_needed
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
    chunk_pool: Res<crate::ldtk_chunks::ChunkPool>,
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
    let coin_chance = 1.0 - enemy_chance - spike_chance - bare_chance;
    let moving_chance = MOVING_PLATFORM_CHANCE + 0.15 * d as f64;
    let powerup_chance = POWERUP_SPAWN_CHANCE_MIN + (POWERUP_SPAWN_CHANCE_MAX - POWERUP_SPAWN_CHANCE_MIN) * d as f64;

    let mut color_idx: usize = 0;

    while tracker.rightmost_platform_x < generate_to {
        // --- Try placing an LDtk hand-designed chunk ---
        let ldtk_spacing_ok = (tracker.rightmost_platform_x - tracker.last_ldtk_chunk_x)
            > LDTK_CHUNK_MIN_SPACING;
        if chunk_pool.loaded && ldtk_spacing_ok && rng.gen_bool(LDTK_CHUNK_CHANCE) {
            if let Some(template) = crate::ldtk_chunks::select_chunk(
                &chunk_pool,
                d,
                tracker.last_platform_y,
                MAX_JUMP_HEIGHT,
                &mut rng,
            ) {
                let offset_x = tracker.rightmost_platform_x + min_gap;
                // Align chunk entry_y with current platform height
                let base_y = tracker.last_platform_y - template.entry_y;

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
                tracker.last_platform_y = base_y + template.exit_y;
                tracker.last_ldtk_chunk_x = offset_x;

                // Advance ground tracker past chunk if it provides ground
                if template.has_ground {
                    tracker.rightmost_ground_x =
                        tracker.rightmost_ground_x.max(offset_x + chunk_width);
                }
                continue;
            }
        }

        // --- Procedural platform generation (fallback) ---
        let dx = rng.gen_range(min_gap..max_gap);

        // Constrain upward dy: for larger gaps, limit how much the platform can rise.
        // The player has a double jump, so max reachable height is generous (~130px),
        // but for very wide gaps the player needs horizontal travel time which reduces
        // effective rise. Scale max rise from full MAX_JUMP_HEIGHT at min_gap down to
        // 60% at max_gap.
        let gap_fraction = ((dx - min_gap) / (max_gap - min_gap + 1.0)).clamp(0.0, 1.0);
        let max_rise = MAX_JUMP_HEIGHT * (1.0 - 0.4 * gap_fraction);

        let dy = rng.gen_range(-MAX_JUMP_HEIGHT..max_rise);

        // Ground surface Y for reference
        let ground_surface = GROUND_Y + GROUND_HEIGHT / 2.0;

        let new_x = tracker.rightmost_platform_x + dx;
        let mut new_y = (tracker.last_platform_y + dy)
            .max(ground_surface + 60.0)
            .min(GROUND_Y + 350.0);

        // Safety: every 8 platforms, force one within jump range of the ground
        // so the player always has a way back up if they fall.
        tracker.platforms_since_ground_level += 1;
        if tracker.platforms_since_ground_level >= 8 {
            new_y = ground_surface + rng.gen_range(60.0..MAX_JUMP_HEIGHT);
            tracker.platforms_since_ground_level = 0;
        }

        // Platform width decreases slightly with difficulty
        let min_w = lerp_diff(PLATFORM_MIN_WIDTH, PLATFORM_MIN_WIDTH * 0.7, d);
        let max_w = lerp_diff(PLATFORM_MAX_WIDTH, PLATFORM_MAX_WIDTH * 0.7, d);
        let width = rng.gen_range(min_w..max_w);
        let color = plat_colors[color_idx % plat_colors.len()];
        color_idx += 1;

        // Maybe make it a moving or breakable platform
        let is_moving = rng.gen_bool(moving_chance.min(0.5));
        let breakable_chance = lerp_diff(BREAKABLE_MIN_CHANCE as f32, BREAKABLE_MAX_CHANCE as f32, d) as f64;
        let is_breakable = !is_moving && rng.gen_bool(breakable_chance.min(0.5));
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
        } else {
            let plat_entity = spawn_platform(
                &mut commands,
                new_x,
                new_y,
                width,
                PLATFORM_HEIGHT,
                color,
                false,
            );

            // Decide what to place on this platform
            let roll: f64 = rng.gen();
            let saw_chance = spike_chance * 0.5;
            let spike_remaining = spike_chance - saw_chance;
            // Split enemy budget: walking, flying, shooter, charging, flying_ranged
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
                spawn_enemy(&mut commands, new_x, new_y, width, game_sprites.enemy_walk.clone());
            } else if roll < walking_chance + flying_chance {
                spawn_flying_enemy(&mut commands, new_x, new_y, game_sprites.enemy_fly.clone());
            } else if roll < walking_chance + flying_chance + shooter_chance {
                spawn_shooter_enemy(&mut commands, new_x, new_y, game_sprites.enemy_shooter.clone());
            } else if roll < walking_chance + flying_chance + shooter_chance + charging_chance && width >= CHARGING_ENEMY_WIDTH * 2.5 {
                spawn_charging_enemy(&mut commands, new_x, new_y, width, game_sprites.enemy_charging.clone());
            } else if roll < walking_chance + flying_chance + shooter_chance + charging_chance + flying_ranged_chance {
                spawn_flying_ranged_enemy(&mut commands, new_x, new_y, game_sprites.enemy_flying_ranged.clone());
            } else if roll < enemy_chance + saw_chance && width >= SAW_SIZE * 3.0 {
                spawn_saw(&mut commands, new_x, new_y, width, game_sprites.saw.clone());
            } else if roll < enemy_chance + saw_chance + spike_remaining {
                // Timed traps replace some spikes at higher difficulty
                if d > 0.3 && rng.gen_bool(0.3) {
                    spawn_timed_trap(&mut commands, new_x, new_y, game_sprites.timed_trap.clone());
                } else {
                    spawn_spike(&mut commands, new_x, new_y, game_sprites.spike.clone());
                }
            } else if roll < enemy_chance + spike_chance + coin_chance {
                // Small chance to spawn a power-up instead of a coin
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

            // Boulder spawner — small chance on platforms at higher difficulty
            if d > 0.4 && rng.gen_bool(0.05) {
                spawn_boulder_spawner(&mut commands, new_x, new_y, game_sprites.boulder.clone(), game_sprites.boulder_warning.clone());
            }
        }

        // Coin formations between platforms (Sonic-style variety).
        // Minimum Y ensures coins never spawn below or inside platforms.
        if rng.gen_bool(0.4) {
            let prev_x = tracker.rightmost_platform_x;
            let prev_y = tracker.last_platform_y;
            let min_coin_y = prev_y.min(new_y) + PLATFORM_HEIGHT / 2.0 + COIN_SIZE;
            let pattern: u32 = rng.gen_range(0..5);

            match pattern {
                0 => {
                    // Arc of coins — parabolic path between platforms
                    let count = rng.gen_range(4..7);
                    for i in 0..count {
                        let t = (i as f32 + 1.0) / (count as f32 + 1.0);
                        let cx = prev_x + (new_x - prev_x) * t;
                        let base_y = prev_y + (new_y - prev_y) * t;
                        let arc_h = 80.0 * (4.0 * t * (1.0 - t));
                        let cy = (base_y + arc_h).max(min_coin_y);
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
    let t = time.elapsed_secs();
    for (mut transform, moving, mut plat_vel) in &mut query {
        let new_y = moving.base_y + (t * moving.speed).sin() * moving.range;
        plat_vel.0.y = new_y - transform.translation.y;
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
