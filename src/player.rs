use bevy::prelude::*;

use crate::animation::{AnimationTimer, CurrentAnim, FacingDirection, PlayerAnimState, SpriteSheets};
use crate::constants::*;
use crate::health::Health;
use crate::level::{Platform, PlatformSize};
use crate::state::GameState;

/// System set for player movement — other modules can schedule `.after(PlayerMovementSet)`.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlayerMovementSet;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub struct Velocity(pub Vec2);

#[derive(Component)]
pub struct Grounded {
    pub on_ground: bool,
    pub coyote_timer: f32,
}

/// Tracks jump count for double-jump support.
#[derive(Component)]
pub struct JumpCounter {
    pub jumps_remaining: u32,
}

/// Tracks whether the player is holding the jump button (for variable height).
#[derive(Component)]
pub struct JumpHeld(pub bool);

/// Score resource — global across the game session.
#[derive(Resource, Default)]
pub struct Score {
    pub value: u32,
}

/// Coins resource — global across the game session.
#[derive(Resource, Default)]
pub struct Coins {
    pub count: u32,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Score>()
            .init_resource::<Coins>()
            .add_systems(
                Update,
                spawn_player_if_missing.run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    player_input,
                    apply_gravity,
                    apply_velocity,
                    respawn_on_fall,
                )
                    .chain()
                    .in_set(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnEnter(GameState::GameOver), reset_player_on_game_over);
    }
}

fn spawn_player_if_missing(
    mut commands: Commands,
    query: Query<Entity, With<Player>>,
    sheets: Option<Res<SpriteSheets>>,
) {
    if !query.is_empty() {
        return;
    }

    let Some(sheets) = sheets else { return };

    commands
        .spawn((
            Sprite {
                image: sheets.idle.image.clone(),
                custom_size: Some(Vec2::new(PLAYER_WIDTH, PLAYER_HEIGHT)),
                texture_atlas: Some(TextureAtlas {
                    layout: sheets.idle.layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            Transform::from_xyz(SPAWN_X, SPAWN_Y, 1.0),
            Player,
            Velocity(Vec2::ZERO),
            Grounded {
                on_ground: true,
                coyote_timer: 0.0,
            },
            JumpCounter {
                jumps_remaining: MAX_JUMPS,
            },
            JumpHeld(false),
            Health::default(),
            PlayerAnimState::default(),
            FacingDirection::default(),
        ))
        .insert((
            CurrentAnim::default(),
            AnimationTimer {
                timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            },
        ));
}

fn player_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<
        (
            &mut Velocity,
            &mut Grounded,
            &mut JumpCounter,
            &mut JumpHeld,
        ),
        With<Player>,
    >,
) {
    let Ok((mut velocity, mut grounded, mut jump_counter, mut jump_held)) =
        query.single_mut()
    else {
        return;
    };

    // Horizontal movement
    let mut dir_x = 0.0;
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        dir_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        dir_x += 1.0;
    }
    velocity.0.x = dir_x * PLAYER_SPEED;

    // Coyote time — grace period after leaving a platform
    if grounded.on_ground {
        grounded.coyote_timer = COYOTE_TIME;
        jump_counter.jumps_remaining = MAX_JUMPS;
    } else {
        grounded.coyote_timer -= time.delta_secs();
    }

    let can_jump =
        grounded.on_ground || grounded.coyote_timer > 0.0 || jump_counter.jumps_remaining > 0;

    // Jump initiation
    if can_jump && keyboard.just_pressed(KeyCode::Space) {
        velocity.0.y = JUMP_FORCE;
        grounded.coyote_timer = 0.0;
        grounded.on_ground = false;
        jump_held.0 = true;

        // Consume a jump
        jump_counter.jumps_remaining = jump_counter.jumps_remaining.saturating_sub(1);
    }

    // Variable jump height — release early for a short hop
    if keyboard.just_released(KeyCode::Space) && jump_held.0 {
        jump_held.0 = false;
        if velocity.0.y > JUMP_FORCE_MIN {
            velocity.0.y = JUMP_FORCE_MIN;
        }
    }
}

fn apply_gravity(
    time: Res<Time>,
    mut query: Query<(&mut Velocity, &Grounded), With<Player>>,
) {
    let Ok((mut velocity, grounded)) = query.single_mut() else {
        return;
    };

    if !grounded.on_ground {
        velocity.0.y += GRAVITY * time.delta_secs();
    }
}

fn apply_velocity(
    time: Res<Time>,
    mut player_query: Query<
        (&mut Transform, &mut Velocity, &mut Grounded),
        With<Player>,
    >,
    platform_query: Query<
        (&Transform, &PlatformSize),
        (With<Platform>, Without<Player>),
    >,
) {
    let Ok((mut transform, mut velocity, mut grounded)) = player_query.single_mut() else {
        return;
    };

    // --- Horizontal pass ---
    transform.translation.x += velocity.0.x * time.delta_secs();

    let player_half_w = PLAYER_WIDTH / 2.0;
    let player_half_h = PLAYER_HEIGHT / 2.0;

    for (plat_tf, plat_size) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w)
            - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h)
            - (transform.translation.y - plat_tf.translation.y).abs();

        if overlap_x > 0.0 && overlap_y > 0.0 {
            if transform.translation.x < plat_tf.translation.x {
                transform.translation.x =
                    plat_tf.translation.x - plat_half_w - player_half_w;
            } else {
                transform.translation.x =
                    plat_tf.translation.x + plat_half_w + player_half_w;
            }
            velocity.0.x = 0.0;
        }
    }

    // --- Vertical pass ---
    transform.translation.y += velocity.0.y * time.delta_secs();

    grounded.on_ground = false;

    for (plat_tf, plat_size) in &platform_query {
        let plat_half_w = plat_size.0.x / 2.0;
        let plat_half_h = plat_size.0.y / 2.0;

        let overlap_x = (player_half_w + plat_half_w)
            - (transform.translation.x - plat_tf.translation.x).abs();
        let overlap_y = (player_half_h + plat_half_h)
            - (transform.translation.y - plat_tf.translation.y).abs();

        // Use a small epsilon so the player stays grounded when sitting
        // exactly on top of a platform (overlap_y == 0.0 after snap).
        if overlap_x > 0.0 && overlap_y >= -0.5 {
            if overlap_y <= 0.0 && transform.translation.y > plat_tf.translation.y {
                // Resting exactly on top — just mark grounded, no position correction.
                grounded.on_ground = true;
            } else if overlap_y > 0.0 && transform.translation.y > plat_tf.translation.y {
                // Landing on top
                transform.translation.y =
                    plat_tf.translation.y + plat_half_h + player_half_h;
                velocity.0.y = 0.0;
                grounded.on_ground = true;
            } else if overlap_y > 0.0 {
                // Bonking head on bottom
                transform.translation.y =
                    plat_tf.translation.y - plat_half_h - player_half_h;
                velocity.0.y = 0.0;
            }
        }
    }
}

fn respawn_on_fall(
    mut query: Query<
        (&mut Transform, &mut Velocity, &mut Grounded, &mut JumpCounter),
        With<Player>,
    >,
) {
    let Ok((mut transform, mut velocity, mut grounded, mut jump_counter)) =
        query.single_mut()
    else {
        return;
    };

    if transform.translation.y < FALL_LIMIT {
        transform.translation.x = SPAWN_X;
        transform.translation.y = SPAWN_Y;
        velocity.0 = Vec2::ZERO;
        grounded.on_ground = true;
        grounded.coyote_timer = 0.0;
        jump_counter.jumps_remaining = MAX_JUMPS;
    }
}

fn reset_player_on_game_over(
    mut query: Query<
        (
            &mut Transform,
            &mut Velocity,
            &mut Grounded,
            &mut JumpCounter,
            &mut Health,
        ),
        With<Player>,
    >,
    mut score: ResMut<Score>,
    mut coins: ResMut<Coins>,
) {
    let Ok((mut transform, mut velocity, mut grounded, mut jump_counter, mut health)) =
        query.single_mut()
    else {
        return;
    };

    transform.translation.x = SPAWN_X;
    transform.translation.y = SPAWN_Y;
    velocity.0 = Vec2::ZERO;
    grounded.on_ground = true;
    grounded.coyote_timer = 0.0;
    jump_counter.jumps_remaining = MAX_JUMPS;
    health.current = health.max;
    score.value = 0;
    coins.count = 0;
}
