use bevy::prelude::*;

use crate::checkpoint::CheckpointFlag;
use crate::collectibles::Coin;
use crate::constants::*;
use crate::enemies::{Enemy, Projectile};
use crate::hazards::{BoulderSpawner, FallingBoulder, Lava, Saw, Spike, TimedTrap};
use crate::player::{Player, Grounded};
use crate::powerups::PowerupKind;

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

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
    /// Resting X captured when shaking starts; the shake offsets from here.
    pub base_x: f32,
}

/// Horizontal shake amplitude (px) for a crumbling platform in its warning phase.
const CRUMBLE_SHAKE_AMPLITUDE: f32 = 3.0;

#[derive(Clone, Copy, PartialEq)]
pub enum CrumbleState {
    Idle,
    Shaking,
    Falling,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Oscillate moving platforms up and down, storing the per-frame delta.
pub fn moving_platform_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &MovingPlatform, &mut PlatformVelocity)>,
) {
    let dt = time.delta_secs();
    for (mut transform, moving, mut plat_vel) in &mut query {
        let old_y = transform.translation.y;
        let t = time.elapsed_secs();
        let new_y = moving.base_y + (t * moving.speed).sin() * moving.range;
        let delta_y = new_y - old_y;
        plat_vel.0.y = if dt > 0.0 { delta_y / dt } else { 0.0 };
        transform.translation.y = new_y;
    }
}

/// Update crumbling platforms: shake when triggered, then fall and despawn.
pub fn update_crumbling_platforms(
    mut commands: Commands,
    time: Res<Time>,
    player_query: Query<(&Transform, &Grounded), With<Player>>,
    mut query: Query<(Entity, &mut Transform, &mut CrumblingPlatform, &PlatformSize), Without<Player>>,
) {
    let player_on = player_query.single().ok().and_then(|(pt, grounded)| {
        if grounded.on_ground { Some(pt.translation) } else { None }
    });

    for (entity, mut transform, mut crumble, size) in &mut query {
        match crumble.state {
            CrumbleState::Idle => {
                if let Some(pp) = player_on {
                    let half_w = size.0.x / 2.0 + PLAYER_WIDTH / 2.0;
                    let on_top = (pp.x - transform.translation.x).abs() < half_w
                        && (pp.y - transform.translation.y) > 0.0
                        && (pp.y - transform.translation.y) < size.0.y + PLAYER_HEIGHT;
                    if on_top {
                        crumble.state = CrumbleState::Shaking;
                        crumble.timer = Timer::from_seconds(CRUMBLE_WARN_TIME, TimerMode::Once);
                        crumble.base_x = transform.translation.x;
                    }
                }
            }
            CrumbleState::Shaking => {
                crumble.timer.tick(time.delta());
                // Fix: set the shake offset absolutely from base_x. The old code
                // accumulated `sin * dt`, which integrates the wave into a ~0.5px
                // wobble that was effectively invisible.
                transform.translation.x = crumble.base_x
                    + (time.elapsed_secs() * 40.0).sin() * CRUMBLE_SHAKE_AMPLITUDE;
                if crumble.timer.just_finished() {
                    crumble.state = CrumbleState::Falling;
                    crumble.timer = Timer::from_seconds(CRUMBLE_FALL_TIME, TimerMode::Once);
                    transform.translation.x = crumble.base_x;
                }
            }
            CrumbleState::Falling => {
                crumble.timer.tick(time.delta());
                transform.translation.y += GRAVITY * 0.5 * time.delta_secs();
                if crumble.timer.just_finished() || transform.translation.y < FALL_LIMIT {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

/// Despawn entities that have fallen far behind the camera.
pub fn despawn_behind_camera(
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

// ---------------------------------------------------------------------------
// Spawn helpers
// ---------------------------------------------------------------------------

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
        parent.spawn((
            Sprite::from_color(highlight, Vec2::new(width, highlight_h)),
            Transform::from_xyz(0.0, height / 2.0 - highlight_h / 2.0, 0.1),
            PlatformDecor,
        ));
        parent.spawn((
            Sprite::from_color(darken(color, 0.15), Vec2::new(width, shadow_h)),
            Transform::from_xyz(0.0, -height / 2.0 + shadow_h / 2.0, 0.1),
            PlatformDecor,
        ));
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
        parent.spawn((
            Sprite::from_color(highlight, Vec2::new(width, 3.0)),
            Transform::from_xyz(0.0, height / 2.0 - 1.5, 0.1),
            PlatformDecor,
        ));
        parent.spawn((
            Sprite::from_color(darken(color, 0.15), Vec2::new(width, 2.0)),
            Transform::from_xyz(0.0, -height / 2.0 + 1.0, 0.1),
            PlatformDecor,
        ));
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

pub fn lighten(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgb(
        (srgba.red + amount).min(1.0),
        (srgba.green + amount).min(1.0),
        (srgba.blue + amount).min(1.0),
    )
}

pub fn darken(color: Color, amount: f32) -> Color {
    let srgba = color.to_srgba();
    Color::srgb(
        (srgba.red - amount).max(0.0),
        (srgba.green - amount).max(0.0),
        (srgba.blue - amount).max(0.0),
    )
}
