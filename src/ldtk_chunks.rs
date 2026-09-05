use std::collections::HashMap;

use bevy::prelude::*;
use serde_json::Value;

use crate::breakable::{spawn_breakable_platform, BreakableGroupCounter};
use crate::checkpoint::section_colors;
use crate::collectibles::{spawn_coin, spawn_coin_at};
use crate::constants::*;
use crate::enemies::{
    spawn_charging_enemy, spawn_enemy, spawn_flying_enemy, spawn_flying_ranged_enemy,
    spawn_shooter_enemy,
};
use crate::hazards::{
    spawn_boulder_spawner, spawn_lava, spawn_saw, spawn_spike, spawn_timed_trap,
};
use crate::level::{spawn_moving_platform, spawn_platform};
use crate::powerups::spawn_powerup;
use crate::sprites::GameSprites;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Collision tile types parsed from IntGrid layer.
/// Values correspond to LDtk IntGrid values:
///   1=Solid, 2=Breakable, 3=Ground, 4=OneWay, 5=Conveyor, 6=Ice, 7=Crumbling, 8=Spring
#[derive(Debug, Clone, Copy, PartialEq)]
enum CollisionType {
    Solid,
    Breakable,
    Ground,
    OneWay,
    Conveyor,
    Ice,
    Crumbling,
    Spring,
}

/// A single collision tile from the IntGrid layer.
#[derive(Debug, Clone)]
struct CollisionTile {
    grid_x: i32,
    grid_y: i32,
    tile_type: CollisionType,
}

/// A pre-parsed entity placement from the Entities layer.
#[derive(Debug, Clone)]
struct ChunkEntity {
    identifier: String,
    local_x: f32,
    local_y: f32,
    width: f32,
    height: f32,
    fields: HashMap<String, FieldVal>,
}

/// Simplified field value from LDtk custom fields.
#[derive(Debug, Clone)]
enum FieldVal {
    Float(f32),
    Int(i32),
    Bool(bool),
    Str(String),
}

impl FieldVal {
    fn as_f32(&self) -> f32 {
        match self {
            FieldVal::Float(v) => *v,
            FieldVal::Int(v) => *v as f32,
            _ => 0.0,
        }
    }

    #[allow(dead_code)]
    fn as_i32(&self) -> i32 {
        match self {
            FieldVal::Int(v) => *v,
            FieldVal::Float(v) => *v as i32,
            _ => 0,
        }
    }
}

/// Metadata and content for a single hand-designed chunk.
#[derive(Debug, Clone)]
pub struct ChunkTemplate {
    pub name: String,
    pub width_px: f32,
    pub height_px: f32,
    pub difficulty_min: f32,
    pub difficulty_max: f32,
    pub entry_y: f32,
    pub exit_y: f32,
    pub has_ground: bool,
    /// Optional section affinity (0-3). None = any section.
    pub section: Option<u32>,
    collision_tiles: Vec<CollisionTile>,
    entities: Vec<ChunkEntity>,
}

/// Pool of all available LDtk chunks.
#[derive(Resource, Default)]
pub struct ChunkPool {
    pub templates: Vec<ChunkTemplate>,
    pub loaded: bool,
    /// Name of the last chunk placed (to avoid back-to-back repeats).
    pub last_placed: Option<String>,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct LdtkChunksPlugin;

impl Plugin for LdtkChunksPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkPool>()
            .add_systems(Startup, load_ldtk_project);
    }
}

/// Load and parse the LDtk project file at startup.
fn load_ldtk_project(mut pool: ResMut<ChunkPool>) {
    let path = "assets/levels/chunks.ldtk";
    let Ok(data) = std::fs::read_to_string(path) else {
        warn!("LDtk: No chunk file at {path}, procedural-only mode.");
        pool.loaded = false;
        return;
    };
    let Ok(root) = serde_json::from_str::<Value>(&data) else {
        error!("LDtk: Failed to parse {path}");
        pool.loaded = false;
        return;
    };

    let grid = root
        .get("defaultGridSize")
        .and_then(|v| v.as_f64())
        .unwrap_or(LDTK_GRID_SIZE as f64) as f32;

    let levels = match root.get("levels").and_then(|v| v.as_array()) {
        Some(l) => l,
        None => {
            warn!("LDtk: No levels found in project file.");
            pool.loaded = false;
            return;
        }
    };

    for level_val in levels {
        if let Some(template) = parse_level(level_val, grid) {
            info!(
                "LDtk: Loaded chunk '{}' ({}x{} px, diff {:.1}-{:.1}, entry_y={:.0}, exit_y={:.0}, ground={}, tiles={}, entities={})",
                template.name,
                template.width_px,
                template.height_px,
                template.difficulty_min,
                template.difficulty_max,
                template.entry_y,
                template.exit_y,
                template.has_ground,
                template.collision_tiles.len(),
                template.entities.len(),
            );
            pool.templates.push(template);
        }
    }

    info!("LDtk: Loaded {} chunk templates", pool.templates.len());
    pool.loaded = !pool.templates.is_empty();
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

fn parse_level(level: &Value, grid: f32) -> Option<ChunkTemplate> {
    let name = level
        .get("identifier")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed")
        .to_string();
    let width_px = level.get("pxWid").and_then(|v| v.as_f64())? as f32;
    let height_px = level.get("pxHei").and_then(|v| v.as_f64())? as f32;

    // Read level custom fields
    let fields = parse_field_instances(level.get("fieldInstances"));
    let difficulty_min = fields
        .get("difficulty_min")
        .map(|v| v.as_f32())
        .unwrap_or(0.0);
    let difficulty_max = fields
        .get("difficulty_max")
        .map(|v| v.as_f32())
        .unwrap_or(1.0);
    let has_ground = match fields.get("has_ground") {
        Some(FieldVal::Bool(b)) => *b,
        _ => false,
    };
    let section = fields.get("section").map(|v| v.as_i32() as u32);

    let layers = level.get("layerInstances").and_then(|v| v.as_array())?;

    let mut collision_tiles = Vec::new();
    let mut entities = Vec::new();
    let mut entry_y: Option<f32> = None;
    let mut exit_y: Option<f32> = None;

    for layer in layers {
        let layer_type = layer.get("__type").and_then(|v| v.as_str()).unwrap_or("");
        match layer_type {
            "IntGrid" => {
                parse_intgrid_layer(layer, grid, height_px, &mut collision_tiles);
            }
            "Entities" => {
                parse_entity_layer(
                    layer,
                    height_px,
                    &mut entities,
                    &mut entry_y,
                    &mut exit_y,
                );
            }
            _ => {}
        }
    }

    // Default entry/exit to mid-height if not explicitly placed
    let entry_y = entry_y.unwrap_or(height_px / 2.0);
    let exit_y = exit_y.unwrap_or(entry_y);

    Some(ChunkTemplate {
        name,
        width_px,
        height_px,
        difficulty_min,
        difficulty_max,
        entry_y,
        exit_y,
        has_ground,
        section,
        collision_tiles,
        entities,
    })
}

fn parse_intgrid_layer(
    layer: &Value,
    grid: f32,
    height_px: f32,
    out: &mut Vec<CollisionTile>,
) {
    let c_wid = layer.get("__cWid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let csv = match layer.get("intGridCsv").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return,
    };
    // Guard against divide-by-zero below when __cWid is missing or malformed.
    if c_wid <= 0 {
        warn!("LDtk: IntGrid layer has invalid __cWid {c_wid}, skipping");
        return;
    }

    for (i, val) in csv.iter().enumerate() {
        let v = val.as_i64().unwrap_or(0) as i32;
        if v == 0 {
            continue;
        }
        let gx = (i as i32) % c_wid;
        let gy = (i as i32) / c_wid;
        let tile_type = match v {
            2 => CollisionType::Breakable,
            3 => CollisionType::Ground,
            4 => CollisionType::OneWay,
            5 => CollisionType::Conveyor,
            6 => CollisionType::Ice,
            7 => CollisionType::Crumbling,
            8 => CollisionType::Spring,
            _ => CollisionType::Solid,
        };
        // Flip Y: LDtk is top-down, we want bottom-up
        let _ = height_px;
        let _ = grid;
        out.push(CollisionTile {
            grid_x: gx,
            grid_y: gy,
            tile_type,
        });
    }
}

fn parse_entity_layer(
    layer: &Value,
    height_px: f32,
    out: &mut Vec<ChunkEntity>,
    entry_y: &mut Option<f32>,
    exit_y: &mut Option<f32>,
) {
    let instances = match layer.get("entityInstances").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return,
    };

    for inst in instances {
        let identifier = inst
            .get("__identifier")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let px = inst.get("px").and_then(|v| v.as_array());
        let (ldtk_x, ldtk_y) = match px {
            Some(arr) if arr.len() >= 2 => (
                arr[0].as_f64().unwrap_or(0.0) as f32,
                arr[1].as_f64().unwrap_or(0.0) as f32,
            ),
            _ => continue,
        };

        // Flip Y
        let local_x = ldtk_x;
        let local_y = height_px - ldtk_y;

        let width = inst
            .get("width")
            .and_then(|v| v.as_f64())
            .unwrap_or(16.0) as f32;
        let height = inst
            .get("height")
            .and_then(|v| v.as_f64())
            .unwrap_or(16.0) as f32;

        let fields = parse_field_instances(inst.get("fieldInstances"));

        // Handle entry/exit markers
        if identifier == "EntryPoint" {
            *entry_y = Some(local_y);
            continue;
        }
        if identifier == "ExitPoint" {
            *exit_y = Some(local_y);
            continue;
        }

        out.push(ChunkEntity {
            identifier,
            local_x,
            local_y,
            width,
            height,
            fields,
        });
    }
}

fn parse_field_instances(val: Option<&Value>) -> HashMap<String, FieldVal> {
    let mut map = HashMap::new();
    let arr = match val.and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return map,
    };
    for field in arr {
        let id = match field.get("__identifier").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let val = field.get("__value");
        if let Some(v) = val {
            if let Some(f) = v.as_f64() {
                map.insert(id, FieldVal::Float(f as f32));
            } else if let Some(i) = v.as_i64() {
                map.insert(id, FieldVal::Int(i as i32));
            } else if let Some(b) = v.as_bool() {
                map.insert(id, FieldVal::Bool(b));
            } else if let Some(s) = v.as_str() {
                map.insert(id, FieldVal::Str(s.to_string()));
            }
        }
    }
    map
}

// ---------------------------------------------------------------------------
// Chunk selection
// ---------------------------------------------------------------------------

/// Pick a suitable chunk for the current difficulty, entry height, and section.
/// Avoids repeating the last chunk. Returns None if no chunk fits.
pub fn select_chunk<'a>(
    pool: &'a ChunkPool,
    difficulty: f32,
    entry_y: f32,
    max_y_delta: f32,
    section: u32,
    last_chunk_name: Option<&str>,
    rng: &mut impl rand::Rng,
) -> Option<&'a ChunkTemplate> {
    use rand::seq::SliceRandom;

    let candidates: Vec<&ChunkTemplate> = pool
        .templates
        .iter()
        .filter(|t| difficulty >= t.difficulty_min && difficulty <= t.difficulty_max)
        .filter(|t| (t.entry_y - entry_y).abs() <= max_y_delta * 2.0)
        // Section filter: chunk matches if it has no section preference or matches current
        .filter(|t| t.section.is_none() || t.section == Some(section % 4))
        // Avoid repeating the same chunk back-to-back
        .filter(|t| last_chunk_name.map_or(true, |last| t.name != last))
        .collect();

    // If no candidates after avoiding repeat, try again without the repeat filter
    if candidates.is_empty() {
        let fallback: Vec<&ChunkTemplate> = pool
            .templates
            .iter()
            .filter(|t| difficulty >= t.difficulty_min && difficulty <= t.difficulty_max)
            .filter(|t| (t.entry_y - entry_y).abs() <= max_y_delta * 2.0)
            .filter(|t| t.section.is_none() || t.section == Some(section % 4))
            .collect();
        return fallback.choose(rng).copied();
    }

    candidates.choose(rng).copied()
}

// ---------------------------------------------------------------------------
// Chunk spawning
// ---------------------------------------------------------------------------

/// Spawn all entities from a chunk template at the given world offset.
/// `offset_x` is the left edge of the chunk in world space.
/// `base_y` aligns the chunk's coordinate system to the game world.
/// Returns the chunk width so the generation tracker can advance.
pub fn spawn_chunk(
    commands: &mut Commands,
    template: &ChunkTemplate,
    offset_x: f32,
    base_y: f32,
    game_sprites: &GameSprites,
    section: u32,
    group_counter: &mut BreakableGroupCounter,
) -> f32 {
    let grid = LDTK_GRID_SIZE;
    let height_px = template.height_px;
    let (ground_color, plat_colors) = section_colors(section);

    // --- Spawn collision tiles (merged into horizontal runs) ---
    spawn_collision_runs(
        commands,
        &template.collision_tiles,
        grid,
        height_px,
        offset_x,
        base_y,
        ground_color,
        &plat_colors,
        group_counter,
    );

    // --- Spawn entities ---
    for ent in &template.entities {
        let wx = offset_x + ent.local_x;
        // Convert chunk-local Y (bottom-up) to world Y
        // Chunk Y=0 maps to base_y, chunk Y=height maps to base_y+height
        let wy = base_y + ent.local_y;

        match ent.identifier.as_str() {
            "WalkingEnemy" => {
                let patrol = ent
                    .fields
                    .get("patrol_width")
                    .map(|v| v.as_f32())
                    .unwrap_or(ent.width.max(ENEMY_WIDTH * 3.0));
                spawn_enemy(commands, wx, wy, patrol, game_sprites.enemy_walk.clone());
            }
            "FlyingEnemy" => {
                spawn_flying_enemy(commands, wx, wy, game_sprites.enemy_fly.clone());
            }
            "ShooterEnemy" => {
                spawn_shooter_enemy(commands, wx, wy, game_sprites.enemy_shooter.clone());
            }
            "ChargingEnemy" => {
                let patrol = ent
                    .fields
                    .get("patrol_width")
                    .map(|v| v.as_f32())
                    .unwrap_or(ent.width.max(CHARGING_ENEMY_WIDTH * 3.0));
                spawn_charging_enemy(
                    commands,
                    wx,
                    wy,
                    patrol,
                    game_sprites.enemy_charging.clone(),
                );
            }
            "FlyingRangedEnemy" => {
                spawn_flying_ranged_enemy(
                    commands,
                    wx,
                    wy,
                    game_sprites.enemy_flying_ranged.clone(),
                );
            }
            "Coin" => {
                spawn_coin_at(commands, wx, wy, game_sprites.coin.clone());
            }
            "CoinOnPlatform" => {
                // Coin placed relative to a platform surface
                spawn_coin(commands, wx, wy, game_sprites.coin.clone());
            }
            "Spike" => {
                spawn_spike(commands, wx, wy, game_sprites.spike.clone());
            }
            "Saw" => {
                let patrol = ent
                    .fields
                    .get("patrol_width")
                    .map(|v| v.as_f32())
                    .unwrap_or(ent.width.max(SAW_SIZE * 3.0));
                spawn_saw(commands, wx, wy, patrol, game_sprites.saw.clone());
            }
            "Lava" => {
                spawn_lava(commands, wx, ent.width, game_sprites.lava.clone());
            }
            "TimedTrap" => {
                spawn_timed_trap(commands, wx, wy, game_sprites.timed_trap.clone());
            }
            "BoulderSpawner" => {
                spawn_boulder_spawner(
                    commands,
                    wx,
                    wy,
                    game_sprites.boulder.clone(),
                    game_sprites.boulder_warning.clone(),
                );
            }
            "Powerup" | "PowerupSpeed" => {
                spawn_powerup(
                    commands,
                    wx,
                    wy,
                    game_sprites.powerup_speed.clone(),
                    game_sprites.powerup_jump.clone(),
                    game_sprites.powerup_shield.clone(),
                );
            }
            "MovingPlatform" => {
                let speed = ent
                    .fields
                    .get("speed")
                    .map(|v| v.as_f32())
                    .unwrap_or(MOVING_PLATFORM_SPEED);
                let range = ent
                    .fields
                    .get("range")
                    .map(|v| v.as_f32())
                    .unwrap_or(MOVING_PLATFORM_RANGE);
                let width = ent.width.max(PLATFORM_MIN_WIDTH);
                let color = plat_colors[0];
                spawn_moving_platform(
                    commands, wx, wy, width, PLATFORM_HEIGHT, color, speed, range,
                );
            }
            "BreakableRow" => {
                let count = ent
                    .fields
                    .get("block_count")
                    .map(|v| v.as_i32() as usize)
                    .unwrap_or(BREAKABLE_MIN_BLOCKS);
                group_counter.0 += 1;
                spawn_breakable_platform(commands, wx, wy, count, group_counter.0);
            }
            other => {
                warn!("LDtk: Unknown entity type '{other}' in chunk '{}'", template.name);
            }
        }
    }

    template.width_px
}

/// Merge adjacent IntGrid collision tiles into horizontal runs and spawn platforms.
fn spawn_collision_runs(
    commands: &mut Commands,
    tiles: &[CollisionTile],
    grid: f32,
    height_px: f32,
    offset_x: f32,
    base_y: f32,
    ground_color: Color,
    plat_colors: &[Color; 4],
    group_counter: &mut BreakableGroupCounter,
) {
    if tiles.is_empty() {
        return;
    }

    // Group tiles by row (grid_y)
    let mut rows: HashMap<i32, Vec<&CollisionTile>> = HashMap::new();
    for tile in tiles {
        rows.entry(tile.grid_y).or_default().push(tile);
    }

    let mut color_idx: usize = 0;

    for (gy, mut row_tiles) in rows {
        row_tiles.sort_by_key(|t| t.grid_x);

        // Merge consecutive tiles of the same type into runs
        let mut i = 0;
        while i < row_tiles.len() {
            let start = row_tiles[i];
            let tile_type = start.tile_type;
            let start_gx = start.grid_x;
            let mut end_gx = start_gx;

            // Extend run while consecutive and same type
            while i + 1 < row_tiles.len()
                && row_tiles[i + 1].grid_x == end_gx + 1
                && row_tiles[i + 1].tile_type == tile_type
            {
                end_gx = row_tiles[i + 1].grid_x;
                i += 1;
            }
            i += 1;

            let run_width = (end_gx - start_gx + 1) as f32 * grid;
            let center_local_x = (start_gx as f32 + (end_gx - start_gx) as f32 / 2.0 + 0.5) * grid;
            // Flip Y: grid row 0 is top in LDtk
            let center_local_y = height_px - (gy as f32 + 0.5) * grid;

            let wx = offset_x + center_local_x;
            let wy = base_y + center_local_y;

            match tile_type {
                CollisionType::Ground => {
                    spawn_platform(
                        commands,
                        wx,
                        wy,
                        run_width,
                        GROUND_HEIGHT,
                        ground_color,
                        true,
                    );
                }
                CollisionType::Solid => {
                    let color = plat_colors[color_idx % plat_colors.len()];
                    color_idx += 1;
                    spawn_platform(
                        commands,
                        wx,
                        wy,
                        run_width,
                        PLATFORM_HEIGHT,
                        color,
                        false,
                    );
                }
                CollisionType::Breakable => {
                    let num_blocks = (run_width / BLOCK_WIDTH).round() as usize;
                    if num_blocks > 0 {
                        group_counter.0 += 1;
                        spawn_breakable_platform(commands, wx, wy, num_blocks, group_counter.0);
                    }
                }
                CollisionType::OneWay => {
                    let color = plat_colors[color_idx % plat_colors.len()];
                    color_idx += 1;
                    let ow_color = Color::srgba(
                        color.to_srgba().red * 0.8,
                        color.to_srgba().green * 0.8,
                        color.to_srgba().blue * 1.2,
                        0.7,
                    );
                    let e = spawn_platform(commands, wx, wy, run_width, PLATFORM_HEIGHT, ow_color, false);
                    commands.entity(e).insert(crate::level::OneWayPlatform);
                }
                CollisionType::Conveyor => {
                    // Alternate direction based on color index
                    let direction = if color_idx % 2 == 0 { 1.0 } else { -1.0 };
                    color_idx += 1;
                    let conv_color = if direction > 0.0 {
                        Color::srgb(0.5, 0.7, 0.5)
                    } else {
                        Color::srgb(0.7, 0.5, 0.5)
                    };
                    let e = spawn_platform(commands, wx, wy, run_width, PLATFORM_HEIGHT, conv_color, false);
                    commands.entity(e).insert(crate::level::ConveyorPlatform {
                        speed: CONVEYOR_SPEED * direction,
                    });
                }
                CollisionType::Ice => {
                    let e = spawn_platform(
                        commands, wx, wy, run_width, PLATFORM_HEIGHT,
                        Color::srgb(0.7, 0.85, 0.95), false,
                    );
                    commands.entity(e).insert(crate::level::IcePlatform);
                }
                CollisionType::Crumbling => {
                    let e = spawn_platform(
                        commands, wx, wy, run_width, PLATFORM_HEIGHT,
                        Color::srgb(0.6, 0.5, 0.4), false,
                    );
                    commands.entity(e).insert(crate::level::CrumblingPlatform {
                        state: crate::level::CrumbleState::Idle,
                        timer: Timer::from_seconds(CRUMBLE_WARN_TIME, TimerMode::Once),
                    });
                }
                CollisionType::Spring => {
                    let e = spawn_platform(
                        commands, wx, wy, run_width.max(80.0), PLATFORM_HEIGHT,
                        Color::srgb(0.3, 0.8, 0.3), false,
                    );
                    commands.entity(e).insert(crate::level::SpringPlatform {
                        force_multiplier: SPRING_BOUNCE_MULTIPLIER,
                    });
                }
            }
        }
    }
}
