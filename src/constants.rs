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
pub const FALL_LIMIT: f32 = -600.0;

// Level generation
pub const LEVEL_WIDTH: f32 = 5000.0;
pub const GROUND_Y: f32 = -300.0;
pub const GROUND_HEIGHT: f32 = 40.0;
pub const PLATFORM_MIN_WIDTH: f32 = 100.0;
pub const PLATFORM_MAX_WIDTH: f32 = 250.0;
pub const PLATFORM_HEIGHT: f32 = 20.0;
pub const PLATFORM_COUNT: usize = 30;

// Jump physics constraints for reachability
pub const MAX_JUMP_HEIGHT: f32 = 130.0;
pub const MAX_JUMP_DISTANCE: f32 = 300.0;
pub const MIN_JUMP_DISTANCE: f32 = 150.0;

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
