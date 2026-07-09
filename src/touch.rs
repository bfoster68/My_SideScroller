use bevy::prelude::*;

use crate::constants::*;
use crate::state::GameState;

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Per-zone touch state, updated each frame in PreUpdate.
#[derive(Resource, Default)]
pub struct TouchZoneState {
    pub left_held: bool,
    pub right_held: bool,
    pub jump_held: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub pause_pressed: bool,
}

/// Whether touch controls should be visible.
#[derive(Resource)]
pub struct TouchInputActive {
    pub active: bool,
    pub hide_timer: Timer,
}

impl Default for TouchInputActive {
    fn default() -> Self {
        Self {
            // Always start hidden — show only after first touch event detected
            active: false,
            hide_timer: Timer::from_seconds(TOUCH_HIDE_DELAY, TimerMode::Once),
        }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// Root node for the touch overlay — despawned on exit Playing.
#[derive(Component)]
struct TouchOverlayRoot;

/// Marker on each touch button for visual updates.
#[derive(Component, Clone, Copy, PartialEq)]
enum TouchButton {
    Left,
    Right,
    Jump,
    Pause,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TouchZoneState>()
            .init_resource::<TouchInputActive>()
            .add_systems(
                PreUpdate,
                process_touch_zones.before(crate::input::InputSet),
            )
            .add_systems(OnEnter(GameState::Playing), spawn_touch_overlay)
            .add_systems(OnExit(GameState::Playing), despawn_touch_overlay)
            .add_systems(
                Update,
                (update_touch_visibility, update_touch_visuals)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

// ---------------------------------------------------------------------------
// Touch zone detection
// ---------------------------------------------------------------------------

/// Check if a normalized screen position (0-1, 0-1) is inside a zone.
fn in_zone(nx: f32, ny: f32, zx: f32, zy: f32, zw: f32, zh: f32) -> bool {
    nx >= zx && nx <= zx + zw && ny >= zy && ny <= zy + zh
}

/// Read all active touches and map them to zone states.
fn process_touch_zones(
    touches: Res<Touches>,
    window_query: Query<&Window>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    mut zone: ResMut<TouchZoneState>,
    mut active: ResMut<TouchInputActive>,
) {
    // Reset zone state each frame
    zone.left_held = false;
    zone.right_held = false;
    zone.jump_held = false;
    zone.jump_pressed = false;
    zone.jump_released = false;
    zone.pause_pressed = false;

    // Auto-detect input method: show touch controls on first touch,
    // hide after keyboard/gamepad use for a few seconds
    if touches.iter().next().is_some()
        || touches.iter_just_pressed().next().is_some()
    {
        active.active = true;
        active.hide_timer.reset();
    }
    if keyboard.get_just_pressed().next().is_some()
        || gamepads.iter().next().is_some_and(|gp| {
            gp.just_pressed(GamepadButton::South)
                || gp.just_pressed(GamepadButton::DPadUp)
                || gp.just_pressed(GamepadButton::Start)
        })
    {
        active.hide_timer.reset();
        // Don't immediately hide — let the timer expire
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let w = window.width();
    let h = window.height();
    if w <= 0.0 || h <= 0.0 {
        return;
    }

    // Check all currently held touches
    for touch in touches.iter() {
        let nx = touch.position().x / w;
        let ny = touch.position().y / h;

        if in_zone(nx, ny, TOUCH_LEFT_X, TOUCH_LEFT_Y, TOUCH_LEFT_W, TOUCH_LEFT_H) {
            zone.left_held = true;
        }
        if in_zone(nx, ny, TOUCH_RIGHT_X, TOUCH_RIGHT_Y, TOUCH_RIGHT_W, TOUCH_RIGHT_H) {
            zone.right_held = true;
        }
        if in_zone(nx, ny, TOUCH_JUMP_X, TOUCH_JUMP_Y, TOUCH_JUMP_W, TOUCH_JUMP_H) {
            zone.jump_held = true;
        }
    }

    // Check just-pressed touches for press events
    for touch in touches.iter_just_pressed() {
        let nx = touch.position().x / w;
        let ny = touch.position().y / h;

        if in_zone(nx, ny, TOUCH_JUMP_X, TOUCH_JUMP_Y, TOUCH_JUMP_W, TOUCH_JUMP_H) {
            zone.jump_pressed = true;
        }
        if in_zone(nx, ny, TOUCH_PAUSE_X, TOUCH_PAUSE_Y, TOUCH_PAUSE_W, TOUCH_PAUSE_H) {
            zone.pause_pressed = true;
        }
    }

    // Check just-released touches for release events
    for touch in touches.iter_just_released() {
        let nx = touch.position().x / w;
        let ny = touch.position().y / h;

        if in_zone(nx, ny, TOUCH_JUMP_X, TOUCH_JUMP_Y, TOUCH_JUMP_W, TOUCH_JUMP_H) {
            zone.jump_released = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Visual overlay
// ---------------------------------------------------------------------------

fn spawn_touch_overlay(mut commands: Commands) {
    let btn_color = Color::srgba(1.0, 1.0, 1.0, TOUCH_BUTTON_ALPHA);
    let text_color = Color::srgba(1.0, 1.0, 1.0, 0.6);

    commands
        .spawn((
            TouchOverlayRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            // Don't block UI interaction underneath
            Pickable::IGNORE,
            GlobalZIndex(50),
        ))
        .with_children(|parent| {
            // Left button
            spawn_touch_button(
                parent,
                TouchButton::Left,
                "<",
                TOUCH_LEFT_X, TOUCH_LEFT_Y, TOUCH_LEFT_W, TOUCH_LEFT_H,
                btn_color, text_color, 32.0,
            );
            // Right button
            spawn_touch_button(
                parent,
                TouchButton::Right,
                ">",
                TOUCH_RIGHT_X, TOUCH_RIGHT_Y, TOUCH_RIGHT_W, TOUCH_RIGHT_H,
                btn_color, text_color, 32.0,
            );
            // Jump button
            spawn_touch_button(
                parent,
                TouchButton::Jump,
                "JUMP",
                TOUCH_JUMP_X, TOUCH_JUMP_Y, TOUCH_JUMP_W, TOUCH_JUMP_H,
                btn_color, text_color, 28.0,
            );
            // Pause button
            spawn_touch_button(
                parent,
                TouchButton::Pause,
                "| |",
                TOUCH_PAUSE_X, TOUCH_PAUSE_Y, TOUCH_PAUSE_W, TOUCH_PAUSE_H,
                btn_color, text_color, 20.0,
            );
        });
}

fn spawn_touch_button(
    parent: &mut ChildSpawnerCommands,
    button: TouchButton,
    label: &str,
    x_pct: f32,
    y_pct: f32,
    w_pct: f32,
    h_pct: f32,
    bg_color: Color,
    text_color: Color,
    font_size: f32,
) {
    parent
        .spawn((
            button,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(x_pct * 100.0),
                top: Val::Percent(y_pct * 100.0),
                width: Val::Percent(w_pct * 100.0),
                height: Val::Percent(h_pct * 100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(bg_color),
            Pickable::IGNORE,
        ))
        .with_children(|btn| {
            btn.spawn((
                Text::new(label),
                TextFont { font_size, ..default() },
                TextColor(text_color),
                Pickable::IGNORE,
            ));
        });
}

fn despawn_touch_overlay(
    mut commands: Commands,
    query: Query<Entity, With<TouchOverlayRoot>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// Visual updates
// ---------------------------------------------------------------------------

/// Show/hide overlay based on whether touch is the active input method.
fn update_touch_visibility(
    time: Res<Time>,
    mut active: ResMut<TouchInputActive>,
    mut query: Query<&mut Visibility, With<TouchOverlayRoot>>,
) {
    // Tick the hide timer when not actively touching
    active.hide_timer.tick(time.delta());

    // Show only if touch was detected and keyboard/gamepad hasn't taken over
    let visible = active.active && !active.hide_timer.is_finished();

    for mut vis in &mut query {
        *vis = if visible { Visibility::Inherited } else { Visibility::Hidden };
    }
}

/// Update button colors based on pressed state.
fn update_touch_visuals(
    zone: Res<TouchZoneState>,
    mut query: Query<(&TouchButton, &mut BackgroundColor)>,
) {
    for (button, mut bg) in &mut query {
        let pressed = match button {
            TouchButton::Left => zone.left_held,
            TouchButton::Right => zone.right_held,
            TouchButton::Jump => zone.jump_held,
            TouchButton::Pause => zone.pause_pressed,
        };
        let alpha = if pressed { TOUCH_BUTTON_PRESSED_ALPHA } else { TOUCH_BUTTON_ALPHA };
        bg.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
}
