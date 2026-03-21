// Player
pub const PLAYER_SPEED: f32 = 300.0;
pub const GRAVITY: f32 = -800.0;
pub const JUMP_FORCE: f32 = 500.0;
pub const JUMP_FORCE_MIN: f32 = 250.0; // Short hop when tapping
pub const PLAYER_WIDTH: f32 = 48.0;
pub const PLAYER_HEIGHT: f32 = 64.0;
pub const MAX_JUMPS: u32 = 2; // Double jump

// Spawn / respawn
pub const SPAWN_X: f32 = -400.0;
pub const SPAWN_Y: f32 = -200.0;
pub const FALL_LIMIT: f32 = -420.0;

// Level generation
pub const GROUND_Y: f32 = -300.0;
pub const GROUND_HEIGHT: f32 = 40.0;
pub const PLATFORM_MIN_WIDTH: f32 = 100.0;
pub const PLATFORM_MAX_WIDTH: f32 = 250.0;
pub const PLATFORM_HEIGHT: f32 = 20.0;

// Infinite runner / chunk generation
pub const GENERATE_AHEAD: f32 = 2400.0;
pub const DESPAWN_BEHIND: f32 = 1200.0;
pub const GROUND_SEGMENT_WIDTH: f32 = 600.0;

// Difficulty scaling (score-based)
pub const DIFFICULTY_SCORE_MAX: f32 = 5000.0;
pub const MIN_PLATFORM_GAP: f32 = 200.0;
pub const MAX_PLATFORM_GAP: f32 = 420.0;
pub const MIN_ENEMY_CHANCE: f64 = 0.15;
pub const MAX_ENEMY_CHANCE: f64 = 0.45;
pub const MIN_SPIKE_CHANCE: f64 = 0.10;
pub const MAX_SPIKE_CHANCE: f64 = 0.25;
pub const MIN_GROUND_GAP_CHANCE: f64 = 0.10;
pub const MAX_GROUND_GAP_CHANCE: f64 = 0.40;

// Moving platforms
pub const MOVING_PLATFORM_CHANCE: f64 = 0.15;
pub const MOVING_PLATFORM_SPEED: f32 = 1.5;
pub const MOVING_PLATFORM_RANGE: f32 = 50.0;

// Jump physics constraints for reachability
pub const MAX_JUMP_HEIGHT: f32 = 130.0;

// Camera
pub const CAMERA_LERP_SPEED: f32 = 0.1;
pub const CAMERA_Y_OFFSET: f32 = 50.0;

// Screen shake
pub const SHAKE_MAX_OFFSET: f32 = 8.0;
pub const SHAKE_DECAY: f32 = 3.0;
pub const SHAKE_TRAUMA_ON_HIT: f32 = 0.5;

// Coyote time
pub const COYOTE_TIME: f32 = 0.1;

// Health
pub const MAX_HEALTH: i32 = 3;
pub const INVINCIBILITY_DURATION: f32 = 1.5;
pub const INVINCIBILITY_FLASH_RATE: f32 = 10.0;

// Parallax background
pub const PARALLAX_FAR_SPEED: f32 = 0.05;
pub const PARALLAX_MID_SPEED: f32 = 0.15;
pub const PARALLAX_NEAR_SPEED: f32 = 0.3;

// Particles
pub const PARTICLE_LIFETIME: f32 = 0.5;
pub const PARTICLE_COUNT_JUMP: usize = 6;
pub const PARTICLE_COUNT_LAND: usize = 10;
pub const PARTICLE_SPEED: f32 = 150.0;
pub const PARTICLE_SIZE: f32 = 4.0;
pub const PARTICLE_GRAVITY: f32 = -400.0;

// Enemies
pub const ENEMY_WIDTH: f32 = 40.0;
pub const ENEMY_HEIGHT: f32 = 48.0;
pub const ENEMY_SPEED: f32 = 80.0;
pub const ENEMY_STOMP_BOUNCE: f32 = 400.0;
pub const ENEMY_STOMP_THRESHOLD: f32 = 0.4;
pub const ENEMY_KILL_SCORE: u32 = 100;
pub const ENEMY_Z: f32 = 0.8;

// Flying enemy
pub const FLYING_ENEMY_SIZE: f32 = 36.0;
pub const FLYING_ENEMY_AMPLITUDE: f32 = 40.0;
pub const FLYING_ENEMY_FREQUENCY: f32 = 2.0;
pub const FLYING_ENEMY_Z: f32 = 0.8;

// Shooter enemy
pub const SHOOTER_WIDTH: f32 = 40.0;
pub const SHOOTER_HEIGHT: f32 = 48.0;
pub const SHOOTER_FIRE_INTERVAL: f32 = 2.0;
pub const PROJECTILE_SPEED: f32 = 200.0;
pub const PROJECTILE_SIZE: f32 = 8.0;
pub const PROJECTILE_DAMAGE: i32 = 1;
pub const PROJECTILE_Z: f32 = 0.75;
pub const PROJECTILE_LIFETIME: f32 = 4.0;
pub const SHOOTER_Z: f32 = 0.8;

// Collectibles
pub const COIN_SIZE: f32 = 20.0;
pub const COIN_BOB_AMPLITUDE: f32 = 6.0;
pub const COIN_BOB_SPEED: f32 = 3.0;
pub const COIN_SPIN_SPEED: f32 = 4.0;
pub const COIN_SCORE: u32 = 10;
pub const COIN_FLOAT_HEIGHT: f32 = 40.0;
pub const COIN_Z: f32 = 0.6;

// Hazards
pub const SPIKE_WIDTH: f32 = 24.0;
pub const SPIKE_HEIGHT: f32 = 20.0;
pub const SPIKE_DAMAGE: i32 = 1;
pub const SPIKE_Z: f32 = 0.7;

// Moving saw
pub const SAW_SIZE: f32 = 32.0;
pub const SAW_SPEED: f32 = 120.0;
pub const SAW_DAMAGE: i32 = 1;
pub const SAW_Z: f32 = 0.75;

// Lava
pub const LAVA_HEIGHT: f32 = 30.0;
pub const LAVA_DAMAGE: i32 = 999; // instant kill
pub const LAVA_Z: f32 = 0.3;

// High score
pub const HIGHSCORE_FILE: &str = "highscore.dat";

// Death animation
pub const DEATH_ANIM_DURATION: f32 = 0.8;

// Power-ups
pub const POWERUP_SIZE: f32 = 24.0;
pub const POWERUP_BOB_AMPLITUDE: f32 = 5.0;
pub const POWERUP_BOB_SPEED: f32 = 2.5;
pub const POWERUP_Z: f32 = 0.65;
pub const POWERUP_SPAWN_CHANCE: f64 = 0.05;
pub const SPEED_BOOST_DURATION: f32 = 5.0;
pub const SPEED_BOOST_MULTIPLIER: f32 = 1.5;
pub const TRIPLE_JUMP_DURATION: f32 = 8.0;
pub const TRIPLE_JUMP_MAX: u32 = 3;
pub const SHIELD_HITS: i32 = 1;

// Screen transitions
pub const FADE_DURATION: f32 = 0.3;

// Volume settings
pub const VOLUME_STEP: f32 = 0.1;
pub const DEFAULT_MASTER_VOLUME: f32 = 1.0;
pub const DEFAULT_SFX_VOLUME: f32 = 1.0;
pub const DEFAULT_MUSIC_VOLUME: f32 = 1.0;

// Checkpoints & Sections
pub const CHECKPOINT_INTERVAL: u32 = 500;
pub const SECTION_INTERVAL: u32 = 1000;
pub const SECTION_BANNER_DURATION: f32 = 2.0;
pub const SECTION_BONUS_SCORE: u32 = 250;

// Charging enemy
pub const CHARGING_ENEMY_SPEED: f32 = 240.0;
pub const CHARGING_DETECT_RANGE: f32 = 300.0;
pub const CHARGING_WIND_TIME: f32 = 0.3;
pub const CHARGING_DURATION: f32 = 1.5;
pub const CHARGING_RECOVERY: f32 = 0.5;
pub const CHARGING_ENEMY_WIDTH: f32 = 44.0;
pub const CHARGING_ENEMY_HEIGHT: f32 = 48.0;

// Flying ranged enemy
pub const FLYING_RANGED_FIRE_INTERVAL: f32 = 2.5;
pub const FLYING_RANGED_PROJ_SPEED: f32 = 180.0;

// Falling boulders
pub const BOULDER_SIZE: f32 = 40.0;
pub const BOULDER_GRAVITY: f32 = -600.0;
pub const BOULDER_DAMAGE: i32 = 1;
pub const BOULDER_SPAWN_INTERVAL: f32 = 3.0;
pub const BOULDER_WARNING_TIME: f32 = 0.5;
pub const BOULDER_Z: f32 = 0.8;

// Timed traps
pub const TIMED_TRAP_ON_DURATION: f32 = 1.5;
pub const TIMED_TRAP_OFF_DURATION: f32 = 2.0;
pub const TIMED_TRAP_WARNING: f32 = 0.3;
pub const TIMED_TRAP_DAMAGE: i32 = 1;

// Save system
pub const SAVE_FILE: &str = "save.json";

// Screen resolution options
pub const RESOLUTIONS: [(u32, u32); 3] = [(1280, 720), (1920, 1080), (2560, 1440)];
pub const RESOLUTION_LABELS: [&str; 3] = ["1280x720", "1920x1080", "2560x1440"];

// Performance
pub const MAX_PARTICLES: usize = 200;

// Breakable blocks
pub const BLOCK_WIDTH: f32 = 32.0;
pub const BLOCK_HEIGHT: f32 = 20.0; // matches PLATFORM_HEIGHT
pub const BLOCK_MAX_HEALTH: i32 = 3;
pub const BLOCK_DAMAGE_PER_STOMP: i32 = 1;
pub const BLOCK_PARTICLE_COUNT: usize = 6;
pub const BLOCK_HIT_PARTICLE_COUNT: usize = 3;
pub const BREAKABLE_MIN_CHANCE: f64 = 0.10;  // 10% at difficulty 0
pub const BREAKABLE_MAX_CHANCE: f64 = 0.45;  // 45% at max difficulty
pub const BREAKABLE_MIN_BLOCKS: usize = 3;
pub const BREAKABLE_MAX_BLOCKS: usize = 7;
pub const BREAKABLE_SCORE_PER_BLOCK: u32 = 25;
pub const BLOCK_WEAK_CHANCE: f64 = 0.35; // 35% of blocks spawn pre-damaged

// Debris (block destruction fragments)
pub const DEBRIS_COUNT: usize = 8;
pub const DEBRIS_MIN_SIZE: f32 = 4.0;
pub const DEBRIS_MAX_SIZE: f32 = 12.0;
pub const DEBRIS_MIN_SPEED: f32 = 80.0;
pub const DEBRIS_MAX_SPEED: f32 = 220.0;
pub const DEBRIS_GRAVITY: f32 = -500.0;
pub const DEBRIS_MAX_SPIN: f32 = 12.0; // radians/sec
pub const DEBRIS_LIFETIME_MIN: f32 = 0.6;
pub const DEBRIS_LIFETIME_MAX: f32 = 1.2;

// Enemy death effects
pub const ENEMY_DEATH_PARTICLE_COUNT: usize = 12;
pub const SCORE_POPUP_RISE_SPEED: f32 = 80.0;
pub const SCORE_POPUP_DURATION: f32 = 0.8;

// Knockback
pub const KNOCKBACK_FORCE: f32 = 200.0;
pub const KNOCKBACK_LIFT: f32 = 150.0;
pub const KNOCKBACK_DURATION: f32 = 0.25;

// Hit freeze
pub const STOMP_FREEZE_DURATION: f32 = 0.05;
pub const STOMP_FREEZE_SCALE: f32 = 0.05;
pub const DAMAGE_FREEZE_DURATION: f32 = 0.08;
pub const DAMAGE_FREEZE_SCALE: f32 = 0.02;

// Combo system
pub const MAX_COMBO_POWER: u32 = 4;
pub const COMBO_DISPLAY_DURATION: f32 = 2.0;

// Powerup duration bars
pub const POWERUP_BAR_WIDTH: f32 = 120.0;
pub const POWERUP_BAR_HEIGHT: f32 = 12.0;
#[allow(dead_code)]
pub const POWERUP_BAR_GAP: f32 = 4.0;

// LDtk chunk integration
pub const LDTK_CHUNK_CHANCE: f64 = 0.3;
pub const LDTK_CHUNK_MIN_SPACING: f32 = 800.0;
pub const LDTK_GRID_SIZE: f32 = 16.0;

// Gamepad
pub const GAMEPAD_DEADZONE: f32 = 0.3;
pub const STICK_NAV_INITIAL_DELAY: f32 = 0.3;
pub const STICK_NAV_REPEAT_DELAY: f32 = 0.15;
