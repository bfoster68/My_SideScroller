use bevy::prelude::*;

use crate::constants::*;
use crate::player::{Grounded, Player, Sliding, Velocity};
use crate::health::{DeathTimer, HurtTimer};
use crate::state::GameState;

/// Which animation state the player is in.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PlayerAnimState {
    #[default]
    Idle,
    Run,
    Jump,
    Fall,
    Hurt,
    Faint,
    Slide,
}

/// Which direction the player is facing.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy)]
pub enum FacingDirection {
    Left,
    #[default]
    Right,
}

/// Drives the frame animation timer.
#[derive(Component)]
pub struct AnimationTimer {
    pub timer: Timer,
}

/// Tracks which animation is currently loaded so we know when to swap sprite sheets.
#[derive(Component, Default, PartialEq, Eq, Clone, Copy)]
pub struct CurrentAnim(pub PlayerAnimState);

/// Holds all loaded sprite sheet data for each animation.
#[derive(Resource)]
pub struct SpriteSheets {
    pub idle: AnimSheet,
    pub run: AnimSheet,
    pub jump: AnimSheet,
    pub hurt: AnimSheet,
    pub faint: AnimSheet,
    pub slide: AnimSheet,
    // Fall reuses the later frames of jump
}

/// One animation's sprite sheet data.
pub struct AnimSheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub frame_count: usize,
    /// Width/height of a single frame in pixels (for computing scale).
    pub frame_size: Vec2,
}

pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_sprite_sheets)
            .add_systems(
                Update,
                (
                    update_anim_state,
                    swap_sprite_sheet,
                    animate_frames,
                    apply_sprite_flip,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn load_sprite_sheets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Idle: 6 frames @ 89x182
    let idle_layout = TextureAtlasLayout::from_grid(UVec2::new(89, 182), 6, 1, None, None);
    let idle = AnimSheet {
        image: asset_server.load("sprites/idle.png"),
        layout: layouts.add(idle_layout),
        frame_count: 6,
        frame_size: Vec2::new(89.0, 182.0),
    };

    // Run: 6 frames @ 123x175
    let run_layout = TextureAtlasLayout::from_grid(UVec2::new(123, 175), 6, 1, None, None);
    let run = AnimSheet {
        image: asset_server.load("sprites/run.png"),
        layout: layouts.add(run_layout),
        frame_count: 6,
        frame_size: Vec2::new(123.0, 175.0),
    };

    // Jump: 7 frames @ 126x197 (first ~4 = rising, last ~3 = falling)
    let jump_layout = TextureAtlasLayout::from_grid(UVec2::new(126, 197), 7, 1, None, None);
    let jump = AnimSheet {
        image: asset_server.load("sprites/jump.png"),
        layout: layouts.add(jump_layout),
        frame_count: 7,
        frame_size: Vec2::new(126.0, 197.0),
    };

    // Hurt: 3 frames @ 131x181
    let hurt_layout = TextureAtlasLayout::from_grid(UVec2::new(131, 181), 3, 1, None, None);
    let hurt = AnimSheet {
        image: asset_server.load("sprites/hurt.png"),
        layout: layouts.add(hurt_layout),
        frame_count: 3,
        frame_size: Vec2::new(131.0, 181.0),
    };

    // Faint: 7 frames @ 175x181
    let faint_layout = TextureAtlasLayout::from_grid(UVec2::new(175, 181), 7, 1, None, None);
    let faint = AnimSheet {
        image: asset_server.load("sprites/faint.png"),
        layout: layouts.add(faint_layout),
        frame_count: 7,
        frame_size: Vec2::new(175.0, 181.0),
    };

    // Slide: 7 frames @ 400x157
    let slide_layout = TextureAtlasLayout::from_grid(UVec2::new(400, 157), 7, 1, None, None);
    let slide = AnimSheet {
        image: asset_server.load("sprites/slide.png"),
        layout: layouts.add(slide_layout),
        frame_count: 7,
        frame_size: Vec2::new(400.0, 157.0),
    };

    commands.insert_resource(SpriteSheets {
        idle,
        run,
        jump,
        hurt,
        faint,
        slide,
    });
}

/// Determine animation state from velocity, grounded status, and special states.
fn update_anim_state(
    mut query: Query<
        (
            &Velocity,
            &Grounded,
            &mut PlayerAnimState,
            &mut FacingDirection,
            Option<&HurtTimer>,
            Option<&DeathTimer>,
            Option<&Sliding>,
        ),
        With<Player>,
    >,
) {
    let Ok((velocity, grounded, mut anim_state, mut facing, hurt, death, sliding)) =
        query.single_mut()
    else {
        return;
    };

    // Death animation takes highest priority — lock in Faint state
    if death.is_some() {
        *anim_state = PlayerAnimState::Faint;
        return;
    }

    // Hurt animation takes next priority
    if hurt.is_some() {
        *anim_state = PlayerAnimState::Hurt;
        return;
    }

    // Slide animation
    if sliding.is_some() {
        *anim_state = PlayerAnimState::Slide;
        return;
    }

    if velocity.0.x < -0.1 {
        *facing = FacingDirection::Left;
    } else if velocity.0.x > 0.1 {
        *facing = FacingDirection::Right;
    }

    let new_state = if !grounded.on_ground {
        if velocity.0.y > 0.0 {
            PlayerAnimState::Jump
        } else {
            PlayerAnimState::Fall
        }
    } else if velocity.0.x.abs() > 0.1 {
        PlayerAnimState::Run
    } else {
        PlayerAnimState::Idle
    };

    *anim_state = new_state;
}

/// When the animation state changes, swap to the correct sprite sheet.
fn swap_sprite_sheet(
    sheets: Option<Res<SpriteSheets>>,
    mut query: Query<
        (
            &PlayerAnimState,
            &mut CurrentAnim,
            &mut Sprite,
            &mut Transform,
            &mut AnimationTimer,
        ),
        With<Player>,
    >,
) {
    let Some(sheets) = sheets else { return };
    let Ok((anim_state, mut current, mut sprite, mut transform, mut anim_timer)) =
        query.single_mut()
    else {
        return;
    };

    if current.0 == *anim_state {
        return;
    }

    // Determine which sheet to use
    let (sheet, start_frame) = match anim_state {
        PlayerAnimState::Idle => (&sheets.idle, 0),
        PlayerAnimState::Run => (&sheets.run, 0),
        PlayerAnimState::Jump => (&sheets.jump, 0),
        PlayerAnimState::Fall => (&sheets.jump, 4),
        PlayerAnimState::Hurt => (&sheets.hurt, 0),
        PlayerAnimState::Faint => (&sheets.faint, 0),
        PlayerAnimState::Slide => (&sheets.slide, 0),
    };

    sprite.image = sheet.image.clone();
    if let Some(ref mut atlas) = sprite.texture_atlas {
        atlas.layout = sheet.layout.clone();
        atlas.index = start_frame;
    }
    current.0 = *anim_state;

    // Update scale to keep the player at PLAYER_WIDTH x PLAYER_HEIGHT
    transform.scale.x = PLAYER_WIDTH / sheet.frame_size.x;
    transform.scale.y = PLAYER_HEIGHT / sheet.frame_size.y;

    // Reset the animation timer
    anim_timer.timer.reset();
}

/// Advance sprite sheet frames on a timer.
fn animate_frames(
    time: Res<Time>,
    sheets: Option<Res<SpriteSheets>>,
    mut query: Query<
        (&PlayerAnimState, &mut Sprite, &mut AnimationTimer),
        With<Player>,
    >,
) {
    let Some(sheets) = sheets else { return };
    let Ok((anim_state, mut sprite, mut anim_timer)) = query.single_mut() else {
        return;
    };

    anim_timer.timer.tick(time.delta());
    if !anim_timer.timer.just_finished() {
        return;
    }

    let Some(ref mut atlas) = sprite.texture_atlas else {
        return;
    };

    match anim_state {
        PlayerAnimState::Idle => {
            atlas.index = (atlas.index + 1) % sheets.idle.frame_count;
        }
        PlayerAnimState::Run => {
            atlas.index = (atlas.index + 1) % sheets.run.frame_count;
        }
        PlayerAnimState::Jump => {
            // Play frames 0-3, hold on last rising frame
            if atlas.index < 3 {
                atlas.index += 1;
            }
        }
        PlayerAnimState::Fall => {
            // Play frames 4-6, hold on last falling frame
            if atlas.index < 6 {
                atlas.index += 1;
            }
        }
        PlayerAnimState::Hurt => {
            // Play 3 frames, hold on last
            if atlas.index < sheets.hurt.frame_count - 1 {
                atlas.index += 1;
            }
        }
        PlayerAnimState::Faint => {
            // Play 7 frames, hold on last
            if atlas.index < sheets.faint.frame_count - 1 {
                atlas.index += 1;
            }
        }
        PlayerAnimState::Slide => {
            // Play 7 frames, hold on last
            if atlas.index < sheets.slide.frame_count - 1 {
                atlas.index += 1;
            }
        }
    }
}

/// Flip the sprite horizontally based on facing direction.
fn apply_sprite_flip(
    mut query: Query<(&FacingDirection, &mut Sprite), With<Player>>,
) {
    let Ok((facing, mut sprite)) = query.single_mut() else {
        return;
    };

    sprite.flip_x = *facing == FacingDirection::Left;
}
