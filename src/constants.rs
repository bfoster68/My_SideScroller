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

// High score
pub const HIGHSCORE_FILE: &str = "highscore.dat";

// Screen transitions
pub const FADE_DURATION: f32 = 0.3;
