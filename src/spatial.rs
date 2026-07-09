use bevy::prelude::*;
use std::collections::HashMap;

use crate::collectibles::Coin;
use crate::enemies::Enemy;
use crate::enemy_types::Projectile;
use crate::hazards::{FallingBoulder, Lava, Saw, Spike, TimedTrap};
use crate::player::{Player, PlayerMovementSet};
use crate::state::GameState;

const BUCKET_WIDTH: f32 = 200.0;

/// A 1D spatial hash grid bucketed by X coordinate.
#[derive(Default)]
pub struct SpatialGrid {
    buckets: HashMap<i32, Vec<(Entity, Vec2)>>,
}

impl SpatialGrid {
    fn clear(&mut self) {
        for bucket in self.buckets.values_mut() {
            bucket.clear();
        }
    }

    fn insert(&mut self, entity: Entity, pos: Vec2) {
        let bx = (pos.x / BUCKET_WIDTH).floor() as i32;
        self.buckets.entry(bx).or_default().push((entity, pos));
    }

    /// Return all entities whose bucket overlaps the range [center_x - radius, center_x + radius].
    pub fn query_nearby(&self, center_x: f32, radius: f32) -> impl Iterator<Item = &(Entity, Vec2)> {
        let min_bucket = ((center_x - radius) / BUCKET_WIDTH).floor() as i32;
        let max_bucket = ((center_x + radius) / BUCKET_WIDTH).floor() as i32;
        (min_bucket..=max_bucket)
            .flat_map(move |bx| self.buckets.get(&bx).into_iter().flatten())
    }
}

/// Typed spatial grids for different entity categories.
#[derive(Resource, Default)]
pub struct SpatialGrids {
    pub enemies: SpatialGrid,
    pub projectiles: SpatialGrid,
    pub coins: SpatialGrid,
    pub hazards: SpatialGrid,
}

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpatialGrids>()
            .add_systems(
                Update,
                rebuild_spatial_grids
                    .before(PlayerMovementSet)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

/// Rebuild all spatial grids each frame from current entity positions.
fn rebuild_spatial_grids(
    mut grids: ResMut<SpatialGrids>,
    enemy_query: Query<(Entity, &GlobalTransform), (With<Enemy>, Without<Projectile>)>,
    projectile_query: Query<(Entity, &Transform), With<Projectile>>,
    coin_query: Query<(Entity, &GlobalTransform), With<Coin>>,
    hazard_query: Query<
        (Entity, &GlobalTransform),
        Or<(With<Spike>, With<Saw>, With<Lava>, With<FallingBoulder>, With<TimedTrap>)>,
    >,
    player_query: Query<&Transform, With<Player>>,
) {
    // Only rebuild if player exists (avoids work on menu screens)
    let Ok(player_tf) = player_query.single() else {
        return;
    };

    // Only populate grids for entities within a generous window around the player
    let cull_range = 800.0;
    let px = player_tf.translation.x;

    grids.enemies.clear();
    for (entity, gtf) in &enemy_query {
        let pos = gtf.translation();
        if (pos.x - px).abs() < cull_range {
            grids.enemies.insert(entity, Vec2::new(pos.x, pos.y));
        }
    }

    grids.projectiles.clear();
    for (entity, tf) in &projectile_query {
        let pos = tf.translation;
        if (pos.x - px).abs() < cull_range {
            grids.projectiles.insert(entity, Vec2::new(pos.x, pos.y));
        }
    }

    grids.coins.clear();
    for (entity, gtf) in &coin_query {
        let pos = gtf.translation();
        if (pos.x - px).abs() < cull_range {
            grids.coins.insert(entity, Vec2::new(pos.x, pos.y));
        }
    }

    grids.hazards.clear();
    for (entity, gtf) in &hazard_query {
        let pos = gtf.translation();
        if (pos.x - px).abs() < cull_range {
            grids.hazards.insert(entity, Vec2::new(pos.x, pos.y));
        }
    }
}
