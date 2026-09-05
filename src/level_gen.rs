use bevy::prelude::*;
use rand::Rng;

use crate::breakable::{spawn_breakable_platform, BreakableGroupCounter};
use crate::checkpoint::{CheckpointData, section_colors};
use crate::collectibles::{spawn_coin, spawn_coin_at, spawn_coin_on_moving};
use crate::constants::*;
use crate::enemies::{
    spawn_charging_enemy, spawn_enemy, spawn_enemy_on_moving, spawn_flying_enemy,
    spawn_flying_ranged_enemy, spawn_shooter_enemy,
};
use crate::hazards::{
    spawn_boulder_spawner, spawn_lava, spawn_saw, spawn_saw_on_moving, spawn_spike,
    spawn_spike_on_moving, spawn_timed_trap,
};
use crate::level::{ChunkTracker, Difficulty, lerp_ext, lerp_ext_f64};
use crate::platforms::{
    spawn_platform, spawn_moving_platform, CrumblingPlatform, CrumbleState,
    OneWayPlatform, ConveyorPlatform, IcePlatform, SpringPlatform,
};
use crate::player::Player;
use crate::powerups::spawn_powerup;
use crate::sprites::GameSprites;

/// Height pattern for intentional platform placement variety.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HeightPattern {
    Ascending,
    Descending,
    Plateau,
    Valley,
    Peak,
    Freeform,
}

impl HeightPattern {
    /// Pick the next pattern, biased by current height and difficulty.
    pub fn pick_next(current_y: f32, difficulty: f32, previous: HeightPattern, rng: &mut impl rand::Rng) -> (HeightPattern, u32) {
        let ground_surface = GROUND_Y + GROUND_HEIGHT / 2.0;
        let ceiling = PLATFORM_CEILING_Y;
        let range = ceiling - ground_surface;
        let height_ratio = ((current_y - ground_surface) / range).clamp(0.0, 1.0);

        let mut options: Vec<(HeightPattern, f64)> = Vec::new();

        let asc_weight: f64 = (1.0 - height_ratio as f64) * 1.5 + 0.3;
        let desc_weight: f64 = height_ratio as f64 * 1.5 + 0.3;
        let d_clamped = (difficulty as f64).min(2.5);
        let plateau_weight: f64 = (0.8 - 0.3 * d_clamped).max(0.05);
        let valley_weight: f64 = 0.4 + 0.6 * d_clamped;
        let peak_weight: f64 = 0.4 + 0.6 * d_clamped;
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

        let len = match chosen {
            HeightPattern::Valley | HeightPattern::Peak => rng.gen_range(4..=6),
            HeightPattern::Plateau => rng.gen_range(2..=4),
            _ => rng.gen_range(2..=5),
        };

        (chosen, len)
    }

    /// Compute the target dy for this platform given where we are in the pattern.
    pub fn compute_dy(&self, step: u32, total: u32, difficulty: f32, rng: &mut impl rand::Rng) -> f32 {
        let progress = step as f32 / total as f32;
        match self {
            HeightPattern::Ascending => {
                let step_size = 40.0 + 50.0 * difficulty.min(2.0);
                rng.gen_range(step_size * 0.7..step_size * 1.3)
            }
            HeightPattern::Descending => {
                let step_size = 40.0 + 60.0 * difficulty.min(2.0);
                -rng.gen_range(step_size * 0.7..step_size * 1.3)
            }
            HeightPattern::Plateau => {
                rng.gen_range(-20.0..20.0)
            }
            HeightPattern::Valley => {
                if progress < 0.5 {
                    -rng.gen_range(40.0..80.0)
                } else {
                    rng.gen_range(40.0..80.0)
                }
            }
            HeightPattern::Peak => {
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

    /// Get gap multiplier for this pattern.
    pub fn gap_multiplier(&self, step: u32, total: u32) -> f32 {
        match self {
            HeightPattern::Ascending => 0.8,
            HeightPattern::Descending => 1.2,
            HeightPattern::Plateau => 1.0,
            HeightPattern::Valley | HeightPattern::Peak => {
                let progress = step as f32 / total as f32;
                if progress < 0.5 { 0.9 } else { 1.1 }
            }
            HeightPattern::Freeform => 1.0,
        }
    }

    /// Get width multiplier.
    pub fn width_multiplier(&self) -> f32 {
        match self {
            HeightPattern::Ascending => 0.85,
            HeightPattern::Descending => 1.15,
            HeightPattern::Plateau => 1.2,
            HeightPattern::Valley | HeightPattern::Peak => 1.0,
            HeightPattern::Freeform => 1.0,
        }
    }
}

/// Generate new ground and platform chunks ahead of the camera.
pub(crate) fn generate_chunks(
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
    let gap_chance = lerp_ext_f64(MIN_GROUND_GAP_CHANCE, MAX_GROUND_GAP_CHANCE, EXTREME_GROUND_GAP_CHANCE, d);
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
    let min_gap = lerp_ext(MIN_PLATFORM_GAP, MAX_PLATFORM_GAP * 0.6, EXTREME_PLATFORM_GAP * 0.6, d);
    let max_gap = lerp_ext(MIN_PLATFORM_GAP + 80.0, MAX_PLATFORM_GAP, EXTREME_PLATFORM_GAP, d);
    let enemy_chance = lerp_ext_f64(MIN_ENEMY_CHANCE, MAX_ENEMY_CHANCE, EXTREME_ENEMY_CHANCE, d);
    let spike_chance = lerp_ext_f64(MIN_SPIKE_CHANCE, MAX_SPIKE_CHANCE, EXTREME_SPIKE_CHANCE, d);
    // Coin chance fills remaining probability (minus bare platform %)
    let bare_min = if d > 1.0 {
        lerp_ext_f64(0.10, 0.10, EXTREME_BARE_CHANCE_MIN, d)
    } else { 0.10 };
    let bare_chance = (0.25_f64 - 0.10 * (d.min(1.0) as f64)).max(bare_min);
    let coin_chance = (1.0 - enemy_chance - spike_chance - bare_chance).max(0.0);
    let moving_chance = lerp_ext_f64(MOVING_PLATFORM_CHANCE, MOVING_PLATFORM_CHANCE + 0.15, EXTREME_MOVING_CHANCE, d);
    let powerup_chance = POWERUP_SPAWN_CHANCE_MIN + (POWERUP_SPAWN_CHANCE_MAX - POWERUP_SPAWN_CHANCE_MIN) * (d.min(1.0) as f64);

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
                // Fix: keep the generation frontier inside the same playable band the
                // procedural generator clamps to, so a chunk exit can't push it out.
                let ground_surface = GROUND_Y + GROUND_HEIGHT / 2.0;
                tracker.last_platform_y = tracker
                    .last_platform_y
                    .clamp(ground_surface + 60.0, PLATFORM_CEILING_Y);
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
        let min_w = lerp_ext(PLATFORM_MIN_WIDTH, PLATFORM_MIN_WIDTH * 0.7, PLATFORM_MIN_WIDTH * EXTREME_PLATFORM_MIN_WIDTH_MULT, d) * width_mult;
        let max_w = lerp_ext(PLATFORM_MAX_WIDTH, PLATFORM_MAX_WIDTH * 0.7, PLATFORM_MAX_WIDTH * EXTREME_PLATFORM_MAX_WIDTH_MULT, d) * width_mult;
        let width = rng.gen_range(min_w.max(60.0)..max_w.max(min_w + 10.0));
        let color = plat_colors[color_idx % plat_colors.len()];
        color_idx += 1;

        // --- Platform type selection (weighted) ---
        let breakable_chance = lerp_ext_f64(BREAKABLE_MIN_CHANCE, BREAKABLE_MAX_CHANCE, BREAKABLE_EXTREME_CHANCE, d);
        let crumble_chance = if d > 0.15 {
            lerp_ext_f64(CRUMBLE_MIN_CHANCE, CRUMBLE_MAX_CHANCE, EXTREME_CRUMBLE_CHANCE, d)
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
                base_x: 0.0, // captured when shaking starts
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

            // Boulder spawner — chance scales with difficulty
            let boulder_chance = if d > 1.0 {
                lerp_ext_f64(0.05, 0.05, EXTREME_BOULDER_CHANCE, d)
            } else { 0.05 };
            if d > 0.4 && rng.gen_bool(boulder_chance) {
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
pub(crate) fn place_standard_entities(
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
