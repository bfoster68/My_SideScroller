use bevy::{prelude::*, window::WindowResolution};
use rand::Rng;

const PLAYER_SPEED: f32 = 300.0;
const GRAVITY: f32 = -800.0;
const JUMP_FORCE: f32 = 500.0;
const PLAYER_WIDTH: f32 = 48.0;
const PLAYER_HEIGHT: f32 = 64.0;

const SPAWN_X: f32 = -400.0;
const SPAWN_Y: f32 = -200.0;
const FALL_LIMIT: f32 = -600.0;

// Level generation
const LEVEL_WIDTH: f32 = 5000.0;
const GROUND_Y: f32 = -300.0;
const GROUND_HEIGHT: f32 = 40.0;
const PLATFORM_MIN_WIDTH: f32 = 100.0;
const PLATFORM_MAX_WIDTH: f32 = 250.0;
const PLATFORM_HEIGHT: f32 = 20.0;
const PLATFORM_COUNT: usize = 30;

// Jump physics constraints for reachability
const MAX_JUMP_HEIGHT: f32 = 130.0;
const MAX_JUMP_DISTANCE: f32 = 300.0;
const MIN_JUMP_DISTANCE: f32 = 150.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Grounded {
    on_ground: bool,
    coyote_timer: f32,
}

#[derive(Component)]
struct Platform;

#[derive(Component)]
struct PlatformSize(Vec2);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Side-Scroller".into(),
                resolution: WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.2)))
        .add_systems(Startup, setup)
        .add_systems(Update, (
            player_input,
            apply_gravity,
            apply_velocity,
            camera_follow,
            respawn_on_fall,
            regenerate_level,
        ).chain())
        .run();
}

fn spawn_platform(commands: &mut Commands, x: f32, y: f32, width: f32, height: f32, color: Color) {
    commands.spawn((
        Sprite::from_color(color, Vec2::new(width, height)),
        Transform::from_xyz(x, y, 0.0),
        Platform,
        PlatformSize(Vec2::new(width, height)),
    ));
}

fn generate_platforms(commands: &mut Commands) {
    let mut rng = rand::thread_rng();

    let ground_color = Color::srgb(0.4, 0.3, 0.2);
    let plat_colors = [
        Color::srgb(0.3, 0.5, 0.3),
        Color::srgb(0.3, 0.4, 0.5),
        Color::srgb(0.5, 0.4, 0.3),
        Color::srgb(0.4, 0.5, 0.4),
    ];

    // Ground spans the entire level
    let ground_segments = (LEVEL_WIDTH / 600.0).ceil() as i32;
    let level_start = SPAWN_X - 200.0;
    for i in 0..ground_segments {
        let x = level_start + (i as f32 * 600.0) + 300.0;
        // Randomly add gaps in the ground (skip some segments, but not the first two)
        if i > 1 && rng.gen_bool(0.2) {
            continue;
        }
        spawn_platform(commands, x, GROUND_Y, 600.0, GROUND_HEIGHT, ground_color);
    }

    // Generate reachable platforms
    // Start from a known position near spawn
    let mut prev_x = SPAWN_X;
    let mut prev_y = GROUND_Y + GROUND_HEIGHT / 2.0 + 80.0;

    for i in 0..PLATFORM_COUNT {
        // Random horizontal offset - always move right, within jump distance
        let dx = rng.gen_range(MIN_JUMP_DISTANCE..MAX_JUMP_DISTANCE);

        // Random vertical offset - can go up or down, but within jump height
        let dy = rng.gen_range(-MAX_JUMP_HEIGHT..MAX_JUMP_HEIGHT);

        let new_x = prev_x + dx;
        let new_y = (prev_y + dy)
            .max(GROUND_Y + GROUND_HEIGHT / 2.0 + 40.0)  // don't sink into ground
            .min(GROUND_Y + 300.0);                         // don't go too high

        // Stop generating if we've gone past the level
        if new_x > level_start + LEVEL_WIDTH {
            break;
        }

        let width = rng.gen_range(PLATFORM_MIN_WIDTH..PLATFORM_MAX_WIDTH);
        let color = plat_colors[i % plat_colors.len()];

        spawn_platform(commands, new_x, new_y, width, PLATFORM_HEIGHT, color);

        prev_x = new_x;
        prev_y = new_y;
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Player
    commands.spawn((
        Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT)),
        Transform::from_xyz(SPAWN_X, SPAWN_Y, 1.0),
        Player,
        Velocity(Vec2::ZERO),
        Grounded { on_ground: true, coyote_timer: 0.0 },
    ));

    generate_platforms(&mut commands);
}

fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    query: Single<(&mut Velocity, &mut Grounded), With<Player>>,
) {
    let (mut velocity, mut grounded) = query.into_inner();

    let mut dir_x = 0.0;
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        dir_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        dir_x += 1.0;
    }
    velocity.0.x = dir_x * PLAYER_SPEED;

    // Coyote time - you can still jump briefly after leaving a platform
    if grounded.on_ground {
        grounded.coyote_timer = 0.1; // 100ms grace period
    } else {
        grounded.coyote_timer -= time.delta_secs();
    }

    let can_jump = grounded.on_ground || grounded.coyote_timer > 0.0;

    if can_jump && keyboard.just_pressed(KeyCode::Space) {
        velocity.0.y = JUMP_FORCE;
        grounded.coyote_timer = 0.0; // consume the coyote time
        grounded.on_ground = false;
    }
}

fn apply_gravity(
    time: Res<Time>,
    query: Single<(&mut Velocity, &Grounded), With<Player>>,
) {
    let (mut velocity, grounded) = query.into_inner();

    if !grounded.on_ground {
        velocity.0.y += GRAVITY * time.delta_secs();
    }
}

fn apply_velocity(
    time: Res<Time>,
    player_query: Single<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
    platform_query: Query<(&Transform, &PlatformSize), (With<Platform>, Without<Player>)>,
) {
    let (mut transform, mut velocity, mut grounded) = player_query.into_inner();

    // Move horizontally first
    transform.translation.x += velocity.0.x * time.delta_secs();

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (plat_tf, plat_size) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w) - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h) - (transform.translation.y - plat_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            if transform.translation.x < plat_tf.translation.x {
                transform.translation.x = plat_tf.translation.x - plat_half_w - player_half_w;
            } else {
                transform.translation.x = plat_tf.translation.x + plat_half_w + player_half_w;
            }
            velocity.0.x = 0.0;
        }
    }

    // Move vertically
    transform.translation.y += velocity.0.y * time.delta_secs();

    grounded.on_ground = false;

    for (plat_tf, plat_size) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w) - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h) - (transform.translation.y - plat_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            if transform.translation.y > plat_tf.translation.y {
                transform.translation.y = plat_tf.translation.y + plat_half_h + player_half_h;
                velocity.0.y = 0.0;
                grounded.on_ground = true;
            } else {
                transform.translation.y = plat_tf.translation.y - plat_half_h - player_half_h;
                velocity.0.y = 0.0;
            }
        }
    }
}

fn camera_follow(
    player_query: Query<&Transform, (With<Player>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
) {
    let player_tf = player_query.single().unwrap();
    let mut camera_tf = camera_query.single_mut().unwrap();

    let target_x = player_tf.translation.x;
    let target_y = player_tf.translation.y + 50.0;

    let speed = 0.1;
    camera_tf.translation.x += (target_x - camera_tf.translation.x) * speed;
    camera_tf.translation.y += (target_y - camera_tf.translation.y) * speed;
}

fn respawn_on_fall(
    query: Single<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
) {
    let (mut transform, mut velocity, mut grounded) = query.into_inner();

    if transform.translation.y < FALL_LIMIT {
        transform.translation.x = SPAWN_X;
        transform.translation.y = SPAWN_Y;
        velocity.0 = Vec2::ZERO;
        grounded.on_ground = true;
        grounded.coyote_timer = 0.0;
    }
}

fn regenerate_level(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    platform_query: Query<Entity, With<Platform>>,
    player_query: Single<(&mut Transform, &mut Velocity, &mut Grounded), With<Player>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }

    for entity in &platform_query {
        commands.entity(entity).despawn();
    }

    let (mut transform, mut velocity, mut grounded) = player_query.into_inner();
    transform.translation.x = SPAWN_X;
    transform.translation.y = SPAWN_Y;
    velocity.0 = Vec2::ZERO;
    grounded.on_ground = true;
    grounded.coyote_timer = 0.0;

    generate_platforms(&mut commands);
}