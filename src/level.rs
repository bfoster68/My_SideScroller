use bevy::prelude::*;
use rand::Rng;

use crate::collectibles::{spawn_coin, spawn_coin_at, Coin};
use crate::constants::*;
use crate::enemies::{spawn_enemy, Enemy};
use crate::hazards::{spawn_spike, Spike};
use crate::player::Score;
use crate::state::GameState;

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

/// Tracks how far right we've generated content.
#[derive(Resource)]
pub struct ChunkTracker {
    pub rightmost_ground_x: f32,
    pub rightmost_platform_x: f32,
    pub last_platform_y: f32,
}

impl Default for ChunkTracker {
    fn default() -> Self {
        Self {
            rightmost_ground_x: SPAWN_X - 200.0,
            rightmost_platform_x: SPAWN_X,
            last_platform_y: GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0,
        }
    }
}

/// Current difficulty level (0.0–1.0) driven by score.
#[derive(Resource, Default)]
pub struct Difficulty {
    pub value: f32,
}

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChunkTracker>()
            .init_resource::<Difficulty>()
            .add_systems(
                Update,
                (
                    update_difficulty,
                    generate_chunks,
                    despawn_behind_camera,
                    moving_platform_system,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::Playing), reset_level_if_needed);
    }
}

/// Reset level state when starting a new game (from Menu or GameOver).
fn reset_level_if_needed(
    mut commands: Commands,
    mut chunk_tracker: ResMut<ChunkTracker>,
    mut difficulty: ResMut<Difficulty>,
    platform_query: Query<Entity, With<Platform>>,
    decor_query: Query<Entity, With<PlatformDecor>>,
    enemy_query: Query<Entity, With<Enemy>>,
    coin_query: Query<Entity, With<Coin>>,
    spike_query: Query<Entity, With<Spike>>,
) {
    // Only reset if there are already platforms (coming from GameOver).
    // On first play from Menu, there will be none, and chunks will generate naturally.
    if platform_query.is_empty() {
        return;
    }

    // Despawn everything
    for entity in &platform_query {
        commands.entity(entity).despawn();
    }
    for entity in &decor_query {
        commands.entity(entity).despawn();
    }
    for entity in &enemy_query {
        commands.entity(entity).despawn();
    }
    for entity in &coin_query {
        commands.entity(entity).despawn();
    }
    for entity in &spike_query {
        commands.entity(entity).despawn();
    }

    // Reset trackers
    *chunk_tracker = ChunkTracker::default();
    difficulty.value = 0.0;
}

/// Update difficulty based on current score.
fn update_difficulty(score: Res<Score>, mut difficulty: ResMut<Difficulty>) {
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
) {
    let Ok(camera_tf) = camera_query.single() else {
        return;
    };
    let camera_x = camera_tf.translation.x;
    let generate_to = camera_x + GENERATE_AHEAD;

    let mut rng = rand::thread_rng();
    let d = difficulty.value;

    let ground_color = Color::srgb(0.35, 0.28, 0.18);
    let plat_colors = [
        Color::srgb(0.28, 0.42, 0.28),
        Color::srgb(0.30, 0.35, 0.45),
        Color::srgb(0.45, 0.35, 0.25),
        Color::srgb(0.35, 0.40, 0.30),
    ];

    // --- Generate ground segments ---
    let gap_chance = lerp_diff_f64(MIN_GROUND_GAP_CHANCE, MAX_GROUND_GAP_CHANCE, d);
    while tracker.rightmost_ground_x < generate_to {
        let seg_x = tracker.rightmost_ground_x + GROUND_SEGMENT_WIDTH / 2.0;

        // First two segments always solid, then chance of gaps
        let is_gap = tracker.rightmost_ground_x > SPAWN_X + GROUND_SEGMENT_WIDTH
            && rng.gen_bool(gap_chance);

        if !is_gap {
            spawn_platform(
                &mut commands,
                seg_x,
                GROUND_Y,
                GROUND_SEGMENT_WIDTH,
                GROUND_HEIGHT,
                ground_color,
                true,
            );
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

    let mut color_idx: usize = 0;

    while tracker.rightmost_platform_x < generate_to {
        let dx = rng.gen_range(min_gap..max_gap);
        let dy = rng.gen_range(-MAX_JUMP_HEIGHT..MAX_JUMP_HEIGHT);

        let new_x = tracker.rightmost_platform_x + dx;
        let new_y = (tracker.last_platform_y + dy)
            .max(GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0)
            .min(GROUND_Y + 350.0);

        // Platform width decreases slightly with difficulty
        let min_w = lerp_diff(PLATFORM_MIN_WIDTH, PLATFORM_MIN_WIDTH * 0.7, d);
        let max_w = lerp_diff(PLATFORM_MAX_WIDTH, PLATFORM_MAX_WIDTH * 0.7, d);
        let width = rng.gen_range(min_w..max_w);
        let color = plat_colors[color_idx % plat_colors.len()];
        color_idx += 1;

        // Maybe make it a moving platform
        if rng.gen_bool(moving_chance.min(0.5)) {
            spawn_moving_platform(
                &mut commands,
                new_x,
                new_y,
                width,
                PLATFORM_HEIGHT,
                color,
                MOVING_PLATFORM_SPEED,
                MOVING_PLATFORM_RANGE,
            );
        } else {
            spawn_platform(
                &mut commands,
                new_x,
                new_y,
                width,
                PLATFORM_HEIGHT,
                color,
                false,
            );
        }

        // Decide what to place on this platform
        let roll: f64 = rng.gen();
        if roll < enemy_chance && width >= ENEMY_WIDTH * 2.5 {
            spawn_enemy(&mut commands, new_x, new_y, width);
        } else if roll < enemy_chance + spike_chance {
            spawn_spike(&mut commands, new_x, new_y);
        } else if roll < enemy_chance + spike_chance + coin_chance {
            spawn_coin(&mut commands, new_x, new_y);
        }

        // Mid-air coin between platforms
        if rng.gen_bool(0.3) {
            let mid_x = (tracker.rightmost_platform_x + new_x) / 2.0;
            let mid_y = ((tracker.last_platform_y + new_y) / 2.0) + 40.0;
            spawn_coin_at(&mut commands, mid_x, mid_y);
        }

        tracker.rightmost_platform_x = new_x;
        tracker.last_platform_y = new_y;
    }
}

/// Despawn entities that have fallen far behind the camera.
fn despawn_behind_camera(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera2d>>,
    query: Query<
        (Entity, &Transform),
        Or<(
            With<Platform>,
            With<PlatformDecor>,
            With<Enemy>,
            With<Coin>,
            With<Spike>,
        )>,
    >,
) {
    let Ok(camera_tf) = camera_query.single() else {
        return;
    };
    let cutoff = camera_tf.translation.x - DESPAWN_BEHIND;

    for (entity, transform) in &query {
        if transform.translation.x < cutoff {
            commands.entity(entity).despawn();
        }
    }
}

/// Oscillate moving platforms up and down.
fn moving_platform_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovingPlatform)>,
) {
    let t = time.elapsed_secs();
    for (mut transform, moving) in &mut query {
        transform.translation.y = moving.base_y + (t * moving.speed).sin() * moving.range;
    }
}

/// Spawn a platform with visual surface highlights.
fn spawn_platform(
    commands: &mut Commands,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    is_ground: bool,
) {
    // Main body
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        PlatformSize(Vec2::new(width, height)),
    ));

    // Top surface highlight
    let highlight = if is_ground {
        Color::srgb(0.45, 0.55, 0.3)
    } else {
        lighten(color, 0.15)
    };
    let highlight_h = if is_ground { 6.0 } else { 3.0 };

    commands.spawn((
        Sprite::from_color(highlight, Vec2::new(width, highlight_h)),
        Transform::from_xyz(x, y + height / 2.0 - highlight_h / 2.0, 0.1),
        PlatformDecor,
    ));

    // Bottom edge shadow
    let shadow_h = 2.0;
    commands.spawn((
        Sprite::from_color(darken(color, 0.15), Vec2::new(width, shadow_h)),
        Transform::from_xyz(x, y - height / 2.0 + shadow_h / 2.0, 0.1),
        PlatformDecor,
    ));

    // Side shading for floating platforms
    if !is_ground {
        let edge_w = 3.0;
        commands.spawn((
            Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
            Transform::from_xyz(x - width / 2.0 + edge_w / 2.0, y, 0.1),
            PlatformDecor,
        ));
        commands.spawn((
            Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
            Transform::from_xyz(x + width / 2.0 - edge_w / 2.0, y, 0.1),
            PlatformDecor,
        ));
    }
}

/// Spawn a platform that also has the MovingPlatform component.
fn spawn_moving_platform(
    commands: &mut Commands,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
    speed: f32,
    range: f32,
) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        PlatformSize(Vec2::new(width, height)),
        MovingPlatform {
            base_y: y,
            speed,
            range,
        },
    ));

    // Highlight
    let highlight = lighten(color, 0.15);
    commands.spawn((
        Sprite::from_color(highlight, Vec2::new(width, 3.0)),
        Transform::from_xyz(x, y + height / 2.0 - 1.5, 0.1),
        PlatformDecor,
    ));

    // Shadow
    commands.spawn((
        Sprite::from_color(darken(color, 0.15), Vec2::new(width, 2.0)),
        Transform::from_xyz(x, y - height / 2.0 + 1.0, 0.1),
        PlatformDecor,
    ));

    // Side edges
    let edge_w = 3.0;
    commands.spawn((
        Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
        Transform::from_xyz(x - width / 2.0 + edge_w / 2.0, y, 0.1),
        PlatformDecor,
    ));
    commands.spawn((
        Sprite::from_color(darken(color, 0.08), Vec2::new(edge_w, height)),
        Transform::from_xyz(x + width / 2.0 - edge_w / 2.0, y, 0.1),
        PlatformDecor,
    ));
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
