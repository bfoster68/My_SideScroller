use bevy::prelude::*;

/// Handles for all game entity sprites loaded at startup.
#[derive(Resource, Default)]
pub struct GameSprites {
    pub enemy_walk: Handle<Image>,
    pub enemy_fly: Handle<Image>,
    pub enemy_shooter: Handle<Image>,
    pub enemy_charging: Handle<Image>,
    pub enemy_flying_ranged: Handle<Image>,
    pub projectile: Handle<Image>,
    pub coin: Handle<Image>,
    pub spike: Handle<Image>,
    pub saw: Handle<Image>,
    pub lava: Handle<Image>,
    pub lava_top: Handle<Image>,
    pub powerup_speed: Handle<Image>,
    pub powerup_jump: Handle<Image>,
    pub powerup_shield: Handle<Image>,
    pub boulder: Handle<Image>,
    pub boulder_warning: Handle<Image>,
    pub timed_trap: Handle<Image>,
}

pub struct SpritesPlugin;

impl Plugin for SpritesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameSprites>()
            .add_systems(Startup, load_game_sprites);
    }
}

fn load_game_sprites(asset_server: Res<AssetServer>, mut sprites: ResMut<GameSprites>) {
    sprites.enemy_walk = asset_server.load("sprites/enemy_walk.png");
    sprites.enemy_fly = asset_server.load("sprites/enemy_fly.png");
    sprites.enemy_shooter = asset_server.load("sprites/enemy_shooter.png");
    sprites.enemy_charging = asset_server.load("sprites/enemy_charging.png");
    sprites.enemy_flying_ranged = asset_server.load("sprites/enemy_flying_ranged.png");
    sprites.projectile = asset_server.load("sprites/star.png");
    sprites.coin = asset_server.load("sprites/coin.png");
    sprites.spike = asset_server.load("sprites/spike.png");
    sprites.saw = asset_server.load("sprites/saw.png");
    sprites.lava = asset_server.load("sprites/lava.png");
    sprites.lava_top = asset_server.load("sprites/lava_top.png");
    sprites.powerup_speed = asset_server.load("sprites/powerup_speed.png");
    sprites.powerup_jump = asset_server.load("sprites/powerup_jump.png");
    sprites.powerup_shield = asset_server.load("sprites/powerup_shield.png");
    sprites.boulder = asset_server.load("sprites/boulder.png");
    sprites.boulder_warning = asset_server.load("sprites/boulder_warning.png");
    sprites.timed_trap = asset_server.load("sprites/timed_trap.png");
}
