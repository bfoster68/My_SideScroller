use bevy::diagnostic::{DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

/// Marker for debug overlay root node.
#[derive(Component)]
struct DebugRoot;

/// Marker for the FPS text element.
#[derive(Component)]
struct DebugFpsText;

/// Whether the debug FPS overlay is currently visible.
#[derive(Resource, Default)]
struct DebugVisible(bool);

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_plugins(EntityCountDiagnosticsPlugin::default())
            .init_resource::<DebugVisible>()
            .add_systems(Update, (toggle_debug_overlay, update_fps_text));
    }
}

/// Toggle the debug overlay with backtick (`) key.
fn toggle_debug_overlay(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<DebugVisible>,
    overlay_query: Query<Entity, With<DebugRoot>>,
) {
    if !keyboard.just_pressed(KeyCode::Backquote) {
        return;
    }

    visible.0 = !visible.0;
    info!("Debug overlay toggled: {}", visible.0);

    if visible.0 {
        // Spawn FPS overlay in top-right corner
        commands
            .spawn((
                DebugRoot,
                Node {
                    position_type: PositionType::Absolute,
                    right: Val::Px(16.0),
                    top: Val::Px(48.0),
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                ZIndex(99),
            ))
            .with_children(|parent| {
                parent.spawn((
                    DebugFpsText,
                    Text::new("FPS: --"),
                    TextFont { font_size: 18.0, ..default() },
                    TextColor(Color::srgb(0.0, 1.0, 0.0)),
                ));
            });
    } else {
        // Despawn overlay and children
        for entity in &overlay_query {
            commands.entity(entity).despawn();
        }
    }
}

/// Update FPS text from diagnostics.
fn update_fps_text(
    visible: Res<DebugVisible>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<DebugFpsText>>,
) {
    if !visible.0 {
        return;
    }

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let entity_count = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as u32;

    for mut text in &mut query {
        **text = format!("FPS: {:.0}  |  Entities: {}", fps, entity_count);
    }
}
