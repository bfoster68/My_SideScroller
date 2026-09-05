//! Headless gameplay tests.
//!
//! These build a minimal Bevy app (no window, renderer, audio, or asset
//! loading), force the state to `Playing`, spawn real game entities with the
//! real spawn functions, feed input through the same `GameInput` resource the
//! game uses, and step the world with a fixed 1/60s timestep. They exercise the
//! actual movement, combat, pickup, and enemy systems end-to-end.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use bevy::transform::TransformPlugin;

use crate::checkpoint::CheckpointData;
use crate::collectibles::{spawn_coin, Coin, CollectiblesPlugin};
use crate::constants::*;
use crate::debug::GodMode;
use crate::enemies::{spawn_enemy, EnemiesPlugin, Enemy, Patrol};
use crate::health::{Health, HealthPlugin, Invincible};
use crate::input::GameInput;
use crate::level::Difficulty;
use crate::hazards::{spawn_spike, HazardsPlugin};
use crate::platforms::{spawn_platform, OneWayPlatform, SpringPlatform};
use crate::player::{
    Coins, Grounded, JumpCounter, JumpHeld, Player, PlayerPlugin, Score, Velocity,
};
use crate::spatial::SpatialPlugin;
use crate::sprites::GameSprites;
use crate::state::{GameState, PreviousGameState};

const DT: f32 = 1.0 / 60.0;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Build a headless app already in `GameState::Playing` with no player.
/// Spawn entities AFTER this so the OnEnter(Playing) reset is a no-op.
fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(StatesPlugin)
        .add_plugins(TransformPlugin)
        // Fixed, deterministic timestep.
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
        // Resources normally provided by plugins we intentionally skip.
        .init_resource::<GameInput>()
        .init_resource::<GameSprites>()
        .init_resource::<GodMode>()
        .init_resource::<Difficulty>()
        .init_resource::<CheckpointData>()
        .init_resource::<PreviousGameState>()
        .add_plugins(PlayerPlugin)
        .add_plugins(EnemiesPlugin)
        .add_plugins(CollectiblesPlugin)
        .add_plugins(HealthPlugin)
        .add_plugins(SpatialPlugin)
        .add_plugins(HazardsPlugin)
        .insert_state(GameState::Playing);
    // Enter Playing (fires OnEnter with no player present -> harmless).
    app.update();
    app
}

fn step(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update();
    }
}

fn set_input(app: &mut App, f: impl FnOnce(&mut GameInput)) {
    let mut gi = app.world_mut().resource_mut::<GameInput>();
    *gi = GameInput::default();
    f(&mut gi);
}

fn clear_input(app: &mut App) {
    *app.world_mut().resource_mut::<GameInput>() = GameInput::default();
}

/// Spawn a test player with the real gameplay component set.
fn spawn_player(app: &mut App, x: f32, y: f32, vy: f32, on_ground: bool) {
    app.world_mut().spawn((
        Sprite::default(),
        Transform::from_xyz(x, y, 1.0),
        Player,
        Velocity(Vec2::new(0.0, vy)),
        Grounded { on_ground, coyote_timer: 0.0 },
        JumpCounter { jumps_remaining: MAX_JUMPS },
        JumpHeld(false),
        Health::default(),
    ));
}

fn spawn_ground(app: &mut App, x: f32, y: f32, width: f32) -> Entity {
    let world = app.world_mut();
    let e = {
        let mut cmds = world.commands();
        spawn_platform(&mut cmds, x, y, width, PLATFORM_HEIGHT, Color::WHITE, true)
    };
    world.flush();
    e
}

#[derive(Debug, Clone, Copy)]
struct PlayerSnap {
    pos: Vec3,
    vel: Vec2,
    on_ground: bool,
    jumps: u32,
    hp: i32,
}

fn player(app: &mut App) -> PlayerSnap {
    let world = app.world_mut();
    let mut q = world.query_filtered::<
        (&Transform, &Velocity, &Grounded, &JumpCounter, &Health),
        With<Player>,
    >();
    let (tf, vel, g, jc, hp) = q.single(world).expect("player exists");
    PlayerSnap {
        pos: tf.translation,
        vel: vel.0,
        on_ground: g.on_ground,
        jumps: jc.jumps_remaining,
        hp: hp.current,
    }
}

fn enemy_count(app: &mut App) -> usize {
    app.world_mut().query_filtered::<(), With<Enemy>>().iter(app.world()).count()
}

fn coin_count(app: &mut App) -> usize {
    app.world_mut().query_filtered::<(), With<Coin>>().iter(app.world()).count()
}

fn score(app: &App) -> u32 {
    app.world().resource::<Score>().value
}

fn player_is_invincible(app: &mut App) -> bool {
    app.world_mut()
        .query_filtered::<(), (With<Player>, With<Invincible>)>()
        .iter(app.world())
        .count()
        > 0
}

/// Top surface Y of a platform centered at `y`.
fn plat_top(y: f32) -> f32 {
    y + PLATFORM_HEIGHT / 2.0
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

#[test]
fn player_falls_and_lands_on_platform() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    // Start well above the platform, airborne.
    spawn_player(&mut app, 0.0, 200.0, 0.0, false);

    step(&mut app, 180); // 3 seconds — plenty of time to fall

    let p = player(&mut app);
    assert!(p.on_ground, "player should be grounded after falling, got {p:?}");
    assert_eq!(p.vel.y, 0.0, "vertical velocity should be zeroed on landing");
    let expected_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    assert!(
        (p.pos.y - expected_y).abs() < 0.5,
        "player should rest on platform top: expected y={expected_y}, got {}",
        p.pos.y
    );
}

#[test]
fn player_moves_horizontally_with_input() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0);
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 0.0, rest_y, 0.0, true);
    step(&mut app, 2);

    let start_x = player(&mut app).pos.x;
    set_input(&mut app, |gi| gi.move_x = 1.0);
    step(&mut app, 30); // 0.5s
    let right_x = player(&mut app).pos.x;
    assert!(right_x > start_x + 100.0, "should move right: {start_x} -> {right_x}");

    set_input(&mut app, |gi| gi.move_x = -1.0);
    step(&mut app, 60); // 1.0s
    let left_x = player(&mut app).pos.x;
    assert!(left_x < right_x - 100.0, "should move left: {right_x} -> {left_x}");

    // Player must stay grounded while walking on flat ground.
    assert!(player(&mut app).on_ground, "walking should not leave the ground");
}

#[test]
fn jump_then_double_jump_but_no_triple_jump() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0);
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 0.0, rest_y, 0.0, true);
    step(&mut app, 2);
    assert!(player(&mut app).on_ground);

    // --- First (ground) jump ---
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(!p.on_ground, "should leave the ground on jump");
    assert!(p.vel.y > 300.0, "ground jump should launch upward, vy={}", p.vel.y);
    assert_eq!(p.jumps, MAX_JUMPS - 1, "ground jump consumes one jump");

    // Keep holding (so the short-hop cut does not trigger) until falling.
    set_input(&mut app, |gi| gi.jump_held = true);
    let mut guard = 0;
    while player(&mut app).vel.y >= 0.0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 300, "player never started falling after first jump");
    }
    assert!(!player(&mut app).on_ground, "should still be airborne at apex");

    // --- Second (air / double) jump while falling ---
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(p.vel.y >= 0.0, "double jump should cancel the fall, vy={}", p.vel.y);
    assert_eq!(p.jumps, 0, "double jump consumes the last jump");

    // Fall again.
    set_input(&mut app, |gi| gi.jump_held = true);
    let mut guard = 0;
    while player(&mut app).vel.y >= 0.0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 300, "player never started falling after double jump");
    }
    let falling_vy = player(&mut app).vel.y;
    assert!(falling_vy < 0.0);

    // --- Third press must NOT jump ---
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(
        p.vel.y < 0.0,
        "a third jump must be rejected (still falling), but vy={} (was {falling_vy})",
        p.vel.y
    );
    assert_eq!(p.jumps, 0);
}

#[test]
fn releasing_jump_early_gives_short_hop() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0);
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;

    // Full-height jump: hold until apex, record apex height.
    spawn_player(&mut app, 0.0, rest_y, 0.0, true);
    step(&mut app, 2);
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    set_input(&mut app, |gi| gi.jump_held = true);
    let mut full_apex = f32::MIN;
    for _ in 0..300 {
        step(&mut app, 1);
        let p = player(&mut app);
        full_apex = full_apex.max(p.pos.y);
        if p.on_ground {
            break;
        }
    }

    // Short hop: press then release immediately.
    let mut app2 = make_app();
    spawn_ground(&mut app2, 0.0, 0.0, 2000.0);
    spawn_player(&mut app2, 0.0, rest_y, 0.0, true);
    step(&mut app2, 2);
    set_input(&mut app2, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app2, 1);
    set_input(&mut app2, |gi| gi.jump_released = true); // let go right away
    step(&mut app2, 1);
    clear_input(&mut app2);
    let mut short_apex = f32::MIN;
    for _ in 0..300 {
        step(&mut app2, 1);
        let p = player(&mut app2);
        short_apex = short_apex.max(p.pos.y);
        if p.on_ground {
            break;
        }
    }

    assert!(
        short_apex < full_apex - 20.0,
        "short hop ({short_apex}) should be clearly lower than full jump ({full_apex})"
    );
    assert!(short_apex > rest_y + 5.0, "short hop should still leave the ground");
}

// ---------------------------------------------------------------------------
// Combat (the only attack is the stomp)
// ---------------------------------------------------------------------------

#[test]
fn stomping_enemy_kills_it_scores_and_bounces() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    {
        let world = app.world_mut();
        let img = world.resource::<GameSprites>().enemy_walk.clone();
        let mut cmds = world.commands();
        spawn_enemy(&mut cmds, 0.0, 0.0, 600.0, img);
        world.flush();
    }
    assert_eq!(enemy_count(&mut app), 1);

    // Enemy center y = plat_top + ENEMY_HEIGHT/2. Put the player's bottom just
    // above the enemy's top, falling straight down onto it.
    let enemy_top = plat_top(0.0) + ENEMY_HEIGHT;
    let player_y = enemy_top + PLAYER_HEIGHT / 2.0 + 6.0;
    spawn_player(&mut app, 0.0, player_y, -60.0, false);

    step(&mut app, 30);

    assert_eq!(enemy_count(&mut app), 0, "stomped enemy should be despawned");
    assert_eq!(score(&app), ENEMY_KILL_SCORE, "stomp should award the kill score");
    let p = player(&mut app);
    assert_eq!(p.hp, MAX_HEALTH, "a clean stomp must not damage the player");
    // Bounce: after the stomp the player is launched upward.
    assert!(p.vel.y > 0.0 || p.pos.y > player_y, "stomp should bounce the player upward, {p:?}");
}

#[test]
fn side_contact_with_enemy_damages_player_once() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    {
        let world = app.world_mut();
        let img = world.resource::<GameSprites>().enemy_walk.clone();
        let mut cmds = world.commands();
        spawn_enemy(&mut cmds, 0.0, 0.0, 600.0, img);
        world.flush();
    }

    // Stand beside the enemy at ground level, overlapping horizontally but
    // with the player's feet well below the stomp zone.
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 30.0, rest_y, 0.0, true);

    step(&mut app, 5);

    let p = player(&mut app);
    assert_eq!(p.hp, MAX_HEALTH - 1, "side contact should deal exactly 1 damage, got hp={}", p.hp);
    assert!(player_is_invincible(&mut app), "player should get i-frames after damage");
    assert_eq!(enemy_count(&mut app), 1, "side contact must not kill the enemy");
    assert_eq!(score(&app), 0, "no score for taking damage");

    // Invincibility must prevent a second hit while still overlapping.
    step(&mut app, 30);
    assert_eq!(
        player(&mut app).hp,
        MAX_HEALTH - 1,
        "i-frames should block repeated damage from continued contact"
    );
}

#[test]
fn walking_enemy_patrols_within_platform_bounds() {
    let mut app = make_app();
    let width = 200.0;
    spawn_ground(&mut app, 0.0, 0.0, width);
    {
        let world = app.world_mut();
        let img = world.resource::<GameSprites>().enemy_walk.clone();
        let mut cmds = world.commands();
        spawn_enemy(&mut cmds, 0.0, 0.0, width, img);
        world.flush();
    }
    // A player is required for some systems to run; park it far away.
    spawn_player(&mut app, 5000.0, 5000.0, 0.0, true);

    let (left, right) = {
        let world = app.world_mut();
        let mut q = world.query::<&Patrol>();
        let pat = q.single(world).unwrap();
        (pat.left_bound, pat.right_bound)
    };
    assert!(left < right);

    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    for _ in 0..600 {
        // 10 seconds — many direction reversals
        step(&mut app, 1);
        let world = app.world_mut();
        let mut q = world.query_filtered::<&Transform, With<Enemy>>();
        let x = q.single(world).unwrap().translation.x;
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        assert!(
            x >= left - 0.01 && x <= right + 0.01,
            "enemy escaped patrol bounds: x={x}, bounds=[{left}, {right}]"
        );
    }
    // It should actually travel across most of the range, i.e. really patrol.
    assert!(max_x - min_x > (right - left) * 0.8, "enemy should sweep its patrol range");
}

// ---------------------------------------------------------------------------
// Pickups
// ---------------------------------------------------------------------------

#[test]
fn player_collects_coin_and_scores() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    {
        let world = app.world_mut();
        let img = world.resource::<GameSprites>().coin.clone();
        let mut cmds = world.commands();
        spawn_coin(&mut cmds, 0.0, 0.0, img);
        world.flush();
    }
    assert_eq!(coin_count(&mut app), 1);

    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 0.0, rest_y, 0.0, true);

    step(&mut app, 10);

    assert_eq!(coin_count(&mut app), 0, "coin should be collected");
    assert_eq!(app.world().resource::<Coins>().count, 1);
    assert_eq!(score(&app), COIN_SCORE, "coin should award base score at difficulty 0");
}

// ---------------------------------------------------------------------------
// Regression tests for bugs found in the QA audit. Each one failed on the
// pre-fix code and encodes the intended behaviour.
// ---------------------------------------------------------------------------

fn set_jumps(app: &mut App, n: u32) {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&mut JumpCounter, With<Player>>();
    q.single_mut(world).unwrap().jumps_remaining = n;
}

fn set_jump_held(app: &mut App, held: bool) {
    let world = app.world_mut();
    let mut q = world.query_filtered::<&mut JumpHeld, With<Player>>();
    q.single_mut(world).unwrap().0 = held;
}

fn spawn_one_way(app: &mut App, x: f32, y: f32, width: f32) -> Entity {
    let e = spawn_ground(app, x, y, width);
    app.world_mut().entity_mut(e).insert(OneWayPlatform);
    e
}

fn spawn_spring(app: &mut App, x: f32, y: f32, width: f32) -> Entity {
    let e = spawn_ground(app, x, y, width);
    app.world_mut()
        .entity_mut(e)
        .insert(SpringPlatform { force_multiplier: SPRING_BOUNCE_MULTIPLIER });
    e
}

fn spawn_walker(app: &mut App, plat_x: f32, plat_y: f32, plat_w: f32) {
    let world = app.world_mut();
    let img = world.resource::<GameSprites>().enemy_walk.clone();
    let mut cmds = world.commands();
    spawn_enemy(&mut cmds, plat_x, plat_y, plat_w, img);
    world.flush();
}

/// BUG: one-way platforms had a 4px landing window, so any real fall passed
/// straight through them. Landing must work from a full-speed fall.
#[test]
fn one_way_platform_catches_a_full_speed_fall() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0); // safety floor
    spawn_one_way(&mut app, 0.0, 200.0, 300.0);
    spawn_player(&mut app, 0.0, 600.0, 0.0, false); // 400px drop

    step(&mut app, 180);

    let p = player(&mut app);
    let expected_y = plat_top(200.0) + PLAYER_HEIGHT / 2.0;
    assert!(p.on_ground, "should land on the one-way platform, got {p:?}");
    assert!(
        (p.pos.y - expected_y).abs() < 1.0,
        "should be resting on the one-way platform (y={expected_y}), not the floor: {p:?}"
    );
}

/// BUG: drop-through was impossible (it misread "moving left" as Down, and the
/// same Jump press launched a hop). Down+Jump on a one-way platform must fall through.
#[test]
fn down_plus_jump_drops_through_one_way_platform() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0); // floor to land on afterwards
    spawn_one_way(&mut app, 0.0, 200.0, 300.0);
    let rest_y = plat_top(200.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 0.0, rest_y, 0.0, true);
    step(&mut app, 2);
    assert!(player(&mut app).on_ground);

    set_input(&mut app, |gi| {
        gi.down_held = true;
        gi.jump_pressed = true;
    });
    step(&mut app, 1);
    clear_input(&mut app);
    step(&mut app, 120);

    let p = player(&mut app);
    let floor_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    assert!(
        p.pos.y < 200.0,
        "player should have dropped below the one-way platform: {p:?}"
    );
    assert!(p.on_ground && (p.pos.y - floor_y).abs() < 1.0, "should land on the floor: {p:?}");
}

/// BUG: walking off a ledge charged one jump, and a coyote jump charged another,
/// so you only got ONE jump after leaving a ledge. Coyote jump must keep the double jump.
#[test]
fn coyote_jump_keeps_the_double_jump() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 200.0); // edges at x = +/-100
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 80.0, rest_y, 0.0, true);
    step(&mut app, 2);

    // Walk right until we leave the ledge.
    set_input(&mut app, |gi| gi.move_x = 1.0);
    let mut guard = 0;
    while player(&mut app).on_ground {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 120, "never walked off the ledge");
    }

    // Jump immediately, inside the coyote window.
    set_input(&mut app, |gi| {
        gi.move_x = 1.0;
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(p.vel.y > 300.0, "coyote jump should launch at full force: {p:?}");
    assert_eq!(p.jumps, MAX_JUMPS - 1, "coyote jump must leave the air jump available");

    // Fall, then double jump.
    set_input(&mut app, |gi| gi.jump_held = true);
    let mut guard = 0;
    while player(&mut app).vel.y >= 0.0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 300);
    }
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(p.vel.y >= 0.0, "double jump after a coyote jump must work: {p:?}");
    assert_eq!(p.jumps, 0);
}

/// BUG: the spring cleared `on_ground` before the jump refill ran, so landing on
/// a spring with 0 jumps launched you 1.8x high with no air jump. It must refill.
#[test]
fn spring_bounce_refills_air_jump() {
    let mut app = make_app();
    spawn_spring(&mut app, 0.0, 0.0, 2000.0);
    spawn_player(&mut app, 0.0, 300.0, 0.0, false);
    set_jumps(&mut app, 0); // arrived having spent both jumps

    // Fall onto the spring and get launched.
    let mut guard = 0;
    while player(&mut app).vel.y <= 400.0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 400, "spring never launched the player: {:?}", player(&mut app));
    }
    // Let the ledge-rule settle for a frame.
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(!p.on_ground, "should be airborne after the bounce");
    assert_eq!(p.jumps, MAX_JUMPS - 1, "spring landing must refill jumps: {p:?}");

    // The air jump must actually work on the way down.
    let mut guard = 0;
    while player(&mut app).vel.y >= 0.0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 400);
    }
    set_input(&mut app, |gi| {
        gi.jump_pressed = true;
        gi.jump_held = true;
    });
    step(&mut app, 1);
    let p = player(&mut app);
    assert!(p.vel.y >= 0.0, "air jump after a spring bounce must work: {p:?}");
    assert_eq!(p.jumps, 0);
}

/// BUG: i-frames were read once before the damage loop, so two hazards touching
/// the player on the same frame (enemy + spike) cost 2 HP. Only one hit may land.
#[test]
fn two_simultaneous_hits_cost_only_one_hp() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    spawn_walker(&mut app, 0.0, 0.0, 600.0);
    {
        let world = app.world_mut();
        let img = world.resource::<GameSprites>().spike.clone();
        let mut cmds = world.commands();
        spawn_spike(&mut cmds, 0.0, 0.0, img);
        world.flush();
    }
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 20.0, rest_y, 0.0, true); // overlaps both

    step(&mut app, 3);

    let p = player(&mut app);
    assert_eq!(
        p.hp,
        MAX_HEALTH - 1,
        "enemy + spike on the same frame must only deal one hit, got {p:?}"
    );
    step(&mut app, 30);
    assert_eq!(player(&mut app).hp, MAX_HEALTH - 1, "i-frames must hold");
}

/// BUG: a long fall moved ~20px/frame, skipping the 19px stomp band, so landing
/// squarely on an enemy from a height counted as side contact and hurt you.
#[test]
fn fast_fall_onto_enemy_is_still_a_stomp() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 100.0); // narrow: enemy patrols within +/-30
    spawn_walker(&mut app, 0.0, 0.0, 100.0);
    spawn_player(&mut app, 0.0, 1500.0, 0.0, false); // reaches terminal velocity

    step(&mut app, 240);

    let p = player(&mut app);
    assert_eq!(enemy_count(&mut app), 0, "high-speed landing on an enemy must stomp it: {p:?}");
    assert_eq!(score(&app), ENEMY_KILL_SCORE);
    assert_eq!(p.hp, MAX_HEALTH, "a stomp must never damage the player: {p:?}");
}

/// BUG: no terminal velocity meant unbounded fall speed (tunneling). Fall speed
/// must be capped.
#[test]
fn fall_speed_is_capped_at_terminal_velocity() {
    let mut app = make_app();
    spawn_player(&mut app, 0.0, 5000.0, 0.0, false);

    step(&mut app, 240); // 4s of free fall — uncapped would be -3200

    let p = player(&mut app);
    assert!(
        (p.vel.y + TERMINAL_VELOCITY).abs() < 1.0,
        "fall speed should be capped at -{TERMINAL_VELOCITY}, got {}",
        p.vel.y
    );
}

/// BUG: the knockback impulse was overwritten by input every frame, so a hit
/// had zero horizontal pushback. Taking a hit must push the player away.
#[test]
fn knockback_pushes_player_away_from_enemy() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 2000.0);
    spawn_walker(&mut app, 0.0, 0.0, 2000.0);
    let rest_y = plat_top(0.0) + PLAYER_HEIGHT / 2.0;
    spawn_player(&mut app, 30.0, rest_y, 0.0, true); // just right of the enemy
    let start_x = 30.0;

    step(&mut app, 15); // take the hit, ride the knockback

    let p = player(&mut app);
    assert_eq!(p.hp, MAX_HEALTH - 1, "should have taken the hit: {p:?}");
    assert!(
        p.pos.x > start_x + 10.0,
        "knockback should push the player away from the enemy: x went {start_x} -> {}",
        p.pos.x
    );
}

/// BUG: releasing Jump right after a stomp clipped the bounce to a short hop
/// (and a grounded stomp had its bounce eaten). The stomp bounce must survive.
#[test]
fn stomp_bounce_is_not_cut_by_releasing_jump() {
    let mut app = make_app();
    spawn_ground(&mut app, 0.0, 0.0, 600.0);
    spawn_walker(&mut app, 0.0, 0.0, 600.0);
    let enemy_top = plat_top(0.0) + ENEMY_HEIGHT;
    spawn_player(&mut app, 0.0, enemy_top + PLAYER_HEIGHT / 2.0 + 6.0, -60.0, false);
    set_jump_held(&mut app, true); // still holding Jump from the approach
    set_input(&mut app, |gi| gi.jump_held = true);

    let mut guard = 0;
    while enemy_count(&mut app) > 0 {
        step(&mut app, 1);
        guard += 1;
        assert!(guard < 120, "never stomped the enemy");
    }
    // Let go of Jump the frame after the stomp.
    set_input(&mut app, |gi| gi.jump_released = true);
    step(&mut app, 1);

    let p = player(&mut app);
    assert!(
        p.vel.y > JUMP_FORCE_MIN + 50.0,
        "stomp bounce must not be clipped to a short hop: vy={}",
        p.vel.y
    );
}
