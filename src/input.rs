use bevy::prelude::*;

use crate::constants::*;

/// Unified input state from keyboard and gamepad.
/// Updated every frame in PreUpdate so all other systems can read it.
#[derive(Resource, Default)]
pub struct GameInput {
    pub move_x: f32,           // -1.0 to 1.0 (analog-ready)
    pub jump_pressed: bool,    // just pressed this frame
    pub jump_released: bool,   // just released this frame
    pub jump_held: bool,       // currently held
    pub pause_pressed: bool,   // Escape / Start
    pub confirm_pressed: bool, // Enter/Space / South button
    pub up_pressed: bool,      // menu navigation
    pub down_pressed: bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
}

/// Timer for analog stick menu navigation repeat.
#[derive(Resource)]
struct StickNavTimer {
    up: Timer,
    down: Timer,
    left: Timer,
    right: Timer,
    up_active: bool,
    down_active: bool,
    left_active: bool,
    right_active: bool,
}

impl Default for StickNavTimer {
    fn default() -> Self {
        Self {
            up: Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once),
            down: Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once),
            left: Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once),
            right: Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once),
            up_active: false,
            down_active: false,
            left_active: false,
            right_active: false,
        }
    }
}

/// System set for input processing — touch systems run before this.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameInput>()
            .init_resource::<StickNavTimer>()
            .add_systems(PreUpdate, update_game_input.in_set(InputSet));
    }
}

fn update_game_input(
    mut input: ResMut<GameInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    time: Res<Time>,
    mut stick_nav: ResMut<StickNavTimer>,
    touch_state: Res<crate::touch::TouchZoneState>,
) {
    // Reset all fields
    *input = GameInput::default();

    // --- Keyboard ---
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        input.move_x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        input.move_x += 1.0;
    }
    input.jump_pressed = keyboard.just_pressed(KeyCode::Space);
    input.jump_released = keyboard.just_released(KeyCode::Space);
    input.jump_held = keyboard.pressed(KeyCode::Space);
    input.pause_pressed = keyboard.just_pressed(KeyCode::Escape);
    input.confirm_pressed = keyboard.just_pressed(KeyCode::Enter) || keyboard.just_pressed(KeyCode::Space);
    input.up_pressed = keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::KeyW);
    input.down_pressed = keyboard.just_pressed(KeyCode::ArrowDown) || keyboard.just_pressed(KeyCode::KeyS);
    input.left_pressed = keyboard.just_pressed(KeyCode::ArrowLeft) || keyboard.just_pressed(KeyCode::KeyA);
    input.right_pressed = keyboard.just_pressed(KeyCode::ArrowRight) || keyboard.just_pressed(KeyCode::KeyD);

    // --- Gamepad ---
    if let Some(gamepad) = gamepads.iter().next() {
        // Left stick for movement (with deadzone rescaling)
        let raw_x = gamepad.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
        if raw_x.abs() > GAMEPAD_DEADZONE {
            // Rescale so values just above deadzone start near 0 instead of jumping
            let sign = raw_x.signum();
            let rescaled = (raw_x.abs() - GAMEPAD_DEADZONE) / (1.0 - GAMEPAD_DEADZONE);
            input.move_x += sign * rescaled;
        }

        // Buttons
        input.jump_pressed |= gamepad.just_pressed(GamepadButton::South);
        input.jump_released |= gamepad.just_released(GamepadButton::South);
        input.jump_held |= gamepad.pressed(GamepadButton::South);
        input.pause_pressed |= gamepad.just_pressed(GamepadButton::Start);
        input.confirm_pressed |= gamepad.just_pressed(GamepadButton::South);

        // D-pad for menu navigation
        input.up_pressed |= gamepad.just_pressed(GamepadButton::DPadUp);
        input.down_pressed |= gamepad.just_pressed(GamepadButton::DPadDown);
        input.left_pressed |= gamepad.just_pressed(GamepadButton::DPadLeft);
        input.right_pressed |= gamepad.just_pressed(GamepadButton::DPadRight);

        // Analog stick menu navigation with repeat timer
        let stick_x = raw_x; // use raw value for menu nav thresholds
        let stick_y = gamepad.get(GamepadAxis::LeftStickY).unwrap_or(0.0);
        let threshold = 0.6;

        // Up
        if stick_y > threshold {
            if !stick_nav.up_active {
                stick_nav.up_active = true;
                stick_nav.up = Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once);
                input.up_pressed = true;
            } else {
                stick_nav.up.tick(time.delta());
                if stick_nav.up.is_finished() {
                    input.up_pressed = true;
                    stick_nav.up = Timer::from_seconds(STICK_NAV_REPEAT_DELAY, TimerMode::Once);
                }
            }
        } else {
            stick_nav.up_active = false;
        }

        // Down
        if stick_y < -threshold {
            if !stick_nav.down_active {
                stick_nav.down_active = true;
                stick_nav.down = Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once);
                input.down_pressed = true;
            } else {
                stick_nav.down.tick(time.delta());
                if stick_nav.down.is_finished() {
                    input.down_pressed = true;
                    stick_nav.down = Timer::from_seconds(STICK_NAV_REPEAT_DELAY, TimerMode::Once);
                }
            }
        } else {
            stick_nav.down_active = false;
        }

        // Left
        if stick_x < -threshold {
            if !stick_nav.left_active {
                stick_nav.left_active = true;
                stick_nav.left = Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once);
                input.left_pressed = true;
            } else {
                stick_nav.left.tick(time.delta());
                if stick_nav.left.is_finished() {
                    input.left_pressed = true;
                    stick_nav.left = Timer::from_seconds(STICK_NAV_REPEAT_DELAY, TimerMode::Once);
                }
            }
        } else {
            stick_nav.left_active = false;
        }

        // Right
        if stick_x > threshold {
            if !stick_nav.right_active {
                stick_nav.right_active = true;
                stick_nav.right = Timer::from_seconds(STICK_NAV_INITIAL_DELAY, TimerMode::Once);
                input.right_pressed = true;
            } else {
                stick_nav.right.tick(time.delta());
                if stick_nav.right.is_finished() {
                    input.right_pressed = true;
                    stick_nav.right = Timer::from_seconds(STICK_NAV_REPEAT_DELAY, TimerMode::Once);
                }
            }
        } else {
            stick_nav.right_active = false;
        }
    }

    // --- Touch ---
    if touch_state.left_held {
        input.move_x -= 1.0;
    }
    if touch_state.right_held {
        input.move_x += 1.0;
    }
    input.jump_pressed |= touch_state.jump_pressed;
    input.jump_released |= touch_state.jump_released;
    input.jump_held |= touch_state.jump_held;
    input.pause_pressed |= touch_state.pause_pressed;
    input.confirm_pressed |= touch_state.jump_pressed;

    input.move_x = input.move_x.clamp(-1.0, 1.0);
}
