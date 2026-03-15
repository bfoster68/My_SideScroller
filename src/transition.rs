use bevy::prelude::*;

use crate::constants::FADE_DURATION;
use crate::state::GameState;

/// Drives a fade-to-black / fade-from-black screen transition.
#[derive(Resource)]
pub struct ScreenTransition {
    pub timer: Timer,
    pub phase: TransitionPhase,
    pub target_state: GameState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransitionPhase {
    FadingOut, // going to black
    FadingIn,  // coming from black
}

/// Marker for the full-screen fade overlay entity.
#[derive(Component)]
struct TransitionOverlay;

pub struct TransitionPlugin;

impl Plugin for TransitionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_transition);
    }
}

/// Call this to start a fade transition to a target state.
/// If a transition is already active, does nothing.
pub fn start_transition(commands: &mut Commands, target_state: GameState) {
    commands.insert_resource(ScreenTransition {
        timer: Timer::from_seconds(FADE_DURATION, TimerMode::Once),
        phase: TransitionPhase::FadingOut,
        target_state,
    });

    // Spawn the full-screen black overlay at alpha 0
    commands.spawn((
        TransitionOverlay,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            position_type: PositionType::Absolute,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
        GlobalZIndex(100), // on top of everything
    ));
}

fn update_transition(
    mut commands: Commands,
    time: Res<Time>,
    transition: Option<ResMut<ScreenTransition>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut overlay_query: Query<&mut BackgroundColor, With<TransitionOverlay>>,
    overlay_entities: Query<Entity, With<TransitionOverlay>>,
) {
    let Some(mut transition) = transition else {
        return;
    };

    transition.timer.tick(time.delta());
    let progress = transition.timer.fraction();

    // Update overlay alpha
    let alpha = match transition.phase {
        TransitionPhase::FadingOut => progress,       // 0 → 1
        TransitionPhase::FadingIn => 1.0 - progress,  // 1 → 0
    };

    for mut bg in &mut overlay_query {
        *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, alpha));
    }

    if transition.timer.just_finished() {
        match transition.phase {
            TransitionPhase::FadingOut => {
                // Midpoint: switch to target state, start fading in
                next_state.set(transition.target_state);
                transition.timer.reset();
                transition.phase = TransitionPhase::FadingIn;
            }
            TransitionPhase::FadingIn => {
                // Done: clean up
                for entity in &overlay_entities {
                    commands.entity(entity).despawn();
                }
                commands.remove_resource::<ScreenTransition>();
            }
        }
    }
}
