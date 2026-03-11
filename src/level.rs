use bevy::prelude::*;
use rand::Rng;

use crate::collectibles::{spawn_coin, spawn_coin_at, Coin};
use crate::constants::*;
use crate::enemies::{spawn_enemy, Enemy};
use crate::hazards::{spawn_spike, Spike};
use crate::state::GameState;

#[derive(Component)]
pub struct Platform;

#[derive(Component)]
pub struct PlatformSize(pub Vec2);

/// Marker for decorative elements attached to platforms.
#[derive(Component)]
struct PlatformDecor;

pub struct LevelPlugin;

impl Plugin for LevelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), setup_level_if_empty)
            .add_systems(
                Update,
                regenerate_level.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Only generate platforms if none exist yet (avoids re-generating on unpause).
fn setup_level_if_empty(commands: Commands, platform_query: Query<Entity, With<Platform>>) {
    if platform_query.is_empty() {
        generate_platforms(commands);
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

    // Top surface highlight (lighter stripe)
    let highlight = if is_ground {
        Color::srgb(0.45, 0.55, 0.3) // Green grass-like top
    } else {
        lighten(color, 0.15)
    };
    let highlight_h = if is_ground { 6.0 } else { 3.0 };

    commands.spawn((
        Sprite::from_color(highlight, Vec2::new(width, highlight_h)),
        Transform::from_xyz(x, y + height / 2.0 - highlight_h / 2.0, 0.1),
        PlatformDecor,
    ));

    // Bottom edge shadow (darker stripe)
    let shadow_h = 2.0;
    commands.spawn((
        Sprite::from_color(darken(color, 0.15), Vec2::new(width, shadow_h)),
        Transform::from_xyz(x, y - height / 2.0 + shadow_h / 2.0, 0.1),
        PlatformDecor,
    ));

    // Left/right edge shading for floating platforms
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

pub fn generate_platforms(mut commands: Commands) {
    let mut rng = rand::thread_rng();

    let ground_color = Color::srgb(0.35, 0.28, 0.18);
    let plat_colors = [
        Color::srgb(0.28, 0.42, 0.28), // mossy green
        Color::srgb(0.30, 0.35, 0.45), // stone blue
        Color::srgb(0.45, 0.35, 0.25), // warm brown
        Color::srgb(0.35, 0.40, 0.30), // forest green
    ];

    // Ground spans the entire level
    let ground_segments = (LEVEL_WIDTH / 600.0).ceil() as i32;
    let level_start = SPAWN_X - 200.0;
    for i in 0..ground_segments {
        let x = level_start + (i as f32 * 600.0) + 300.0;
        if i > 1 && rng.gen_bool(0.2) {
            continue;
        }
        spawn_platform(
            &mut commands,
            x,
            GROUND_Y,
            600.0,
            GROUND_HEIGHT,
            ground_color,
            true,
        );
    }

    // Generate reachable floating platforms
    let mut prev_x = SPAWN_X;
    let mut prev_y = GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0;

    for i in 0..PLATFORM_COUNT {
        let dx = rng.gen_range(MIN_JUMP_DISTANCE..MAX_JUMP_DISTANCE);
        let dy = rng.gen_range(-MAX_JUMP_HEIGHT..MAX_JUMP_HEIGHT);

        let new_x = prev_x + dx;
        let new_y = (prev_y + dy)
            .max(GROUND_Y + GROUND_HEIGHT / 2.0 + 40.0)
            .min(GROUND_Y + 300.0);

        if new_x > level_start + LEVEL_WIDTH {
            break;
        }

        let width = rng.gen_range(PLATFORM_MIN_WIDTH..PLATFORM_MAX_WIDTH);
        let color = plat_colors[i % plat_colors.len()];

        spawn_platform(&mut commands, new_x, new_y, width, PLATFORM_HEIGHT, color, false);

        // Decide what to place on this platform (mutually exclusive)
        let roll: f64 = rng.gen();
        if roll < 0.25 && width >= ENEMY_WIDTH * 2.5 {
            // 25% chance: enemy (only on platforms wide enough to patrol)
            spawn_enemy(&mut commands, new_x, new_y, width);
        } else if roll < 0.40 {
            // 15% chance: spike
            spawn_spike(&mut commands, new_x, new_y);
        } else if roll < 0.75 {
            // 35% chance: coin
            spawn_coin(&mut commands, new_x, new_y);
        }
        // else 25%: bare platform

        // Occasionally spawn a mid-air coin between platforms
        if i > 0 && rng.gen_bool(0.3) {
            let mid_x = (prev_x + new_x) / 2.0;
            let mid_y = ((prev_y + new_y) / 2.0) + 40.0;
            spawn_coin_at(&mut commands, mid_x, mid_y);
        }

        prev_x = new_x;
        prev_y = new_y;
    }
}

fn regenerate_level(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    platform_query: Query<Entity, With<Platform>>,
    decor_query: Query<Entity, With<PlatformDecor>>,
    enemy_query: Query<Entity, With<Enemy>>,
    coin_query: Query<Entity, With<Coin>>,
    spike_query: Query<Entity, With<Spike>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }

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

    generate_platforms(commands);
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
