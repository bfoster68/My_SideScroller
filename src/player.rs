use bevy::prelude::*;

use crate::animation::{AnimationTimer, CurrentAnim, FacingDirection, PlayerAnimState, SpriteSheets};
use crate::constants::*;
use crate::health::{DeathTimer, Health, HurtTimer};
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

/// Present on the player while they are performing a ground slide.
#[derive(Component)]
pub struct Sliding {
    pub timer: Timer,
}

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
                    tick_slide,
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

    // Use from_atlas_image + transform scale (struct literal Sprite doesn't render atlas correctly)
    let scale_x = PLAYER_WIDTH / sheets.idle.frame_size.x;
    let scale_y = PLAYER_HEIGHT / sheets.idle.frame_size.y;
    commands
        .spawn((
            Sprite::from_atlas_image(
                sheets.idle.image.clone(),
                TextureAtlas {
                    layout: sheets.idle.layout.clone(),
                    index: 0,
                },
            ),
            Transform::from_xyz(SPAWN_X, SPAWN_Y, 1.0)
                .with_scale(Vec3::new(scale_x, scale_y, 1.0)),
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
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut query: Query<
        (
            Entity,
            &mut Velocity,
            &mut Grounded,
            &mut JumpCounter,
            &mut JumpHeld,
            &FacingDirection,
            Option<&DeathTimer>,
            Option<&HurtTimer>,
            Option<&Sliding>,
        ),
        With<Player>,
    >,
) {
    let Ok((
        entity,
        mut velocity,
        mut grounded,
        mut jump_counter,
        mut jump_held,
        facing,
        death_timer,
        hurt_timer,
        sliding,
    )) = query.single_mut()
    else {
        return;
    };

    // Block all input during death animation
    if death_timer.is_some() {
        velocity.0.x = 0.0;
        return;
    }

    // Block movement input during hurt (knockback velocity continues)
    if hurt_timer.is_some() {
        return;
    }

    // While sliding, maintain slide velocity and block other input
    if sliding.is_some() {
        return;
    }

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

    // Slide initiation: Down arrow or S while grounded and moving
    let slide_pressed = keyboard.just_pressed(KeyCode::ArrowDown)
        || keyboard.just_pressed(KeyCode::KeyS);
    if slide_pressed && grounded.on_ground && dir_x.abs() > 0.0 {
        let slide_dir = match facing {
            FacingDirection::Left => -1.0,
            FacingDirection::Right => 1.0,
        };
        velocity.0.x = SLIDE_SPEED * slide_dir;
        commands.entity(entity).insert(Sliding {
            timer: Timer::from_seconds(SLIDE_DURATION, TimerMode::Once),
        });
    }
}

/// Count down the slide timer and remove the component when done.
fn tick_slide(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sliding, &mut Velocity), With<Player>>,
) {
    let Ok((entity, mut sliding, mut velocity)) = query.single_mut() else {
        return;
    };

    sliding.timer.tick(time.delta());

    if sliding.timer.is_finished() {
        commands.entity(entity).remove::<Sliding>();
        // Let velocity return to normal on next input frame
        velocity.0.x = 0.0;
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
    mut commands: Commands,
    mut query: Query<
        (Entity, &mut Transform, &mut Velocity, &mut Grounded, &mut JumpCounter),
        With<Player>,
    >,
) {
    let Ok((entity, mut transform, mut velocity, mut grounded, mut jump_counter)) =
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
        // Clean up any active slide
        commands.entity(entity).remove::<Sliding>();
    }
}

fn reset_player_on_game_over(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
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
    let Ok((entity, mut transform, mut velocity, mut grounded, mut jump_counter, mut health)) =
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
    // Clean up any lingering state components
    commands.entity(entity).remove::<Sliding>();
}
