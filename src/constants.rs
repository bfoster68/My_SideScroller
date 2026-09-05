// Player
pub const PLAYER_SPEED: f32 = 300.0;
pub const GRAVITY: f32 = -800.0;
// Cap fall speed so a long drop can't skip past a platform (or an enemy's
// stomp zone) in a single frame.
pub const TERMINAL_VELOCITY: f32 = 1200.0;
pub const JUMP_FORCE: f32 = 500.0;
pub const JUMP_FORCE_MIN: f32 = 250.0; // Short hop when tapping
pub const DOUBLE_JUMP_MAX_RATIO: f32 = 0.75; // Best case: 75% of full jump (used while rising)
pub const DOUBLE_JUMP_MIN_RATIO: f32 = 0.40; // Worst case: 40% of full jump (used while falling fast)
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

// Difficulty scaling (score-based, multi-phase)
// Phase 1: 0→5000 pts = linear 0.0→1.0 (same as before)
// Phase 2: 5000+ pts = logarithmic growth past 1.0 (never caps)
pub const DIFFICULTY_PHASE1_SCORE: f32 = 5000.0;
pub const DIFFICULTY_PHASE2_DIVISOR: f32 = 5000.0; // controls log growth rate

pub const MIN_PLATFORM_GAP: f32 = 200.0;
pub const MAX_PLATFORM_GAP: f32 = 420.0;
pub const EXTREME_PLATFORM_GAP: f32 = 520.0;
pub const MIN_ENEMY_CHANCE: f64 = 0.15;
pub const MAX_ENEMY_CHANCE: f64 = 0.45;
pub const EXTREME_ENEMY_CHANCE: f64 = 0.65;
pub const MIN_SPIKE_CHANCE: f64 = 0.10;
pub const MAX_SPIKE_CHANCE: f64 = 0.25;
pub const EXTREME_SPIKE_CHANCE: f64 = 0.40;
pub const MIN_GROUND_GAP_CHANCE: f64 = 0.10;
pub const MAX_GROUND_GAP_CHANCE: f64 = 0.40;
pub const EXTREME_GROUND_GAP_CHANCE: f64 = 0.60;

// Moving platforms
pub const MOVING_PLATFORM_CHANCE: f64 = 0.15;
pub const MOVING_PLATFORM_SPEED: f32 = 1.5;
pub const MOVING_PLATFORM_RANGE: f32 = 50.0;

// Extreme difficulty scaling (d > 1.0)
pub const EXTREME_ENEMY_SPEED: f32 = 140.0;
pub const EXTREME_CRUMBLE_CHANCE: f64 = 0.18;
pub const EXTREME_MOVING_CHANCE: f64 = 0.45;
pub const EXTREME_BARE_CHANCE_MIN: f64 = 0.05;
pub const EXTREME_BOULDER_CHANCE: f64 = 0.12;
pub const EXTREME_PLATFORM_MIN_WIDTH_MULT: f32 = 0.5; // platforms shrink to 50% of base
pub const EXTREME_PLATFORM_MAX_WIDTH_MULT: f32 = 0.5;
pub const EXTREME_COIN_BONUS: f32 = 80.0; // coin value bonus at extreme (vs 40 at d=1)
pub const EXTREME_POWERUP_DURATION_MULT: f32 = 0.6; // powerups last 60% as long at extreme

// Jump physics constraints for reachability
pub const MAX_JUMP_HEIGHT: f32 = 130.0;
pub const PLATFORM_CEILING_Y: f32 = 250.0; // max Y for floating platforms

// Camera
pub const CAMERA_LERP_SPEED: f32 = 0.1;
pub const CAMERA_Y_OFFSET: f32 = 70.0;

// New platform types
pub const ICE_FRICTION_MULTIPLIER: f32 = 0.3;
pub const CONVEYOR_SPEED: f32 = 80.0;
pub const SPRING_BOUNCE_MULTIPLIER: f32 = 1.8;
pub const CRUMBLE_WARN_TIME: f32 = 0.8;
pub const CRUMBLE_FALL_TIME: f32 = 0.4;

// Platform type spawn chances
pub const ONE_WAY_CHANCE: f64 = 0.12;
pub const CONVEYOR_CHANCE: f64 = 0.06;
pub const CONVEYOR_START_DIFFICULTY: f32 = 0.3;
pub const ICE_CHANCE: f64 = 0.05;
pub const ICE_START_DIFFICULTY: f32 = 0.2;
pub const CRUMBLE_MIN_CHANCE: f64 = 0.06;
pub const CRUMBLE_MAX_CHANCE: f64 = 0.10;
pub const SPRING_CHANCE: f64 = 0.04;

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

// Shooter enemy (fire rate and speed scale with difficulty)
pub const SHOOTER_WIDTH: f32 = 40.0;
pub const SHOOTER_HEIGHT: f32 = 48.0;
pub const SHOOTER_FIRE_INTERVAL_MAX: f32 = 2.5; // slow at low difficulty
pub const SHOOTER_FIRE_INTERVAL_MIN: f32 = 1.2; // fast at high difficulty
pub const SHOOTER_FIRE_INTERVAL_EXTREME: f32 = 0.7; // very fast at extreme difficulty
pub const SHOOTER_RANGE: f32 = 600.0; // only fire when player is within range
pub const PROJECTILE_SPEED_MIN: f32 = 160.0;
pub const PROJECTILE_SPEED_MAX: f32 = 280.0;
pub const PROJECTILE_SPEED_EXTREME: f32 = 380.0;
pub const PROJECTILE_SIZE: f32 = 8.0;
pub const PROJECTILE_DAMAGE: i32 = 1;
pub const PROJECTILE_Z: f32 = 0.75;
pub const PROJECTILE_LIFETIME: f32 = 2.0;
pub const SHOOTER_Z: f32 = 0.8;

// Collectibles
pub const COIN_SIZE: f32 = 28.0;
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
pub const POWERUP_SPAWN_CHANCE_MIN: f64 = 0.05;
pub const POWERUP_SPAWN_CHANCE_MAX: f64 = 0.12;
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
pub const CHARGING_DETECT_RANGE: f32 = 220.0;
pub const CHARGING_WIND_TIME: f32 = 0.45;
pub const CHARGING_DURATION: f32 = 1.5;
pub const CHARGING_RECOVERY: f32 = 0.5;
pub const CHARGING_ENEMY_WIDTH: f32 = 44.0;
pub const CHARGING_ENEMY_HEIGHT: f32 = 48.0;

// Enemy type spawn weights and thresholds
pub const CHARGING_START_DIFFICULTY: f32 = 0.3;
pub const FLYING_RANGED_START_DIFFICULTY: f32 = 0.5;
pub const CHARGING_SPAWN_WEIGHT: f64 = 0.15;
pub const FLYING_RANGED_SPAWN_WEIGHT: f64 = 0.15;
pub const ENEMY_WALKING_WEIGHT: f64 = 0.72;
pub const ENEMY_FLYING_WEIGHT: f64 = 0.18;
pub const ENEMY_SHOOTER_WEIGHT: f64 = 0.10;

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
pub const BLOCK_WEAR_RATE: f32 = 0.4; // wear points per second while running on block
pub const BLOCK_PARTICLE_COUNT: usize = 6;
pub const BLOCK_HIT_PARTICLE_COUNT: usize = 3;
pub const BREAKABLE_MIN_CHANCE: f64 = 0.10;  // 10% at difficulty 0
pub const BREAKABLE_MAX_CHANCE: f64 = 0.45;  // 45% at difficulty 1.0
pub const BREAKABLE_EXTREME_CHANCE: f64 = 0.65; // 65% at extreme difficulty
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
pub const POWERUP_BAR_GAP: f32 = 4.0;

// LDtk chunk integration
pub const LDTK_CHUNK_CHANCE: f64 = 0.3;
pub const LDTK_CHUNK_MIN_SPACING: f32 = 800.0;
pub const LDTK_GRID_SIZE: f32 = 16.0;

// Gamepad
pub const GAMEPAD_DEADZONE: f32 = 0.3;
pub const STICK_NAV_INITIAL_DELAY: f32 = 0.3;
pub const STICK_NAV_REPEAT_DELAY: f32 = 0.15;

// Touch controls (zones as [x%, y%, width%, height%] of screen)
pub const TOUCH_BUTTON_ALPHA: f32 = 0.25;
pub const TOUCH_BUTTON_PRESSED_ALPHA: f32 = 0.5;
pub const TOUCH_HIDE_DELAY: f32 = 3.0;
// Left button: bottom-left
pub const TOUCH_LEFT_X: f32 = 0.02;
pub const TOUCH_LEFT_Y: f32 = 0.70;
pub const TOUCH_LEFT_W: f32 = 0.15;
pub const TOUCH_LEFT_H: f32 = 0.25;
// Right button: next to left
pub const TOUCH_RIGHT_X: f32 = 0.18;
pub const TOUCH_RIGHT_Y: f32 = 0.70;
pub const TOUCH_RIGHT_W: f32 = 0.15;
pub const TOUCH_RIGHT_H: f32 = 0.25;
// Jump button: bottom-right, larger
pub const TOUCH_JUMP_X: f32 = 0.75;
pub const TOUCH_JUMP_Y: f32 = 0.65;
pub const TOUCH_JUMP_W: f32 = 0.23;
pub const TOUCH_JUMP_H: f32 = 0.30;
// Pause button: top-right
pub const TOUCH_PAUSE_X: f32 = 0.90;
pub const TOUCH_PAUSE_Y: f32 = 0.02;
pub const TOUCH_PAUSE_W: f32 = 0.08;
pub const TOUCH_PAUSE_H: f32 = 0.08;
