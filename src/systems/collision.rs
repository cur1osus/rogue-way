use bevy::prelude::*;
use std::collections::HashMap;

use bevy::prelude::*;

use crate::components::{CollisionLayer, Hitbox};

/// Система разделения сущностей по хитбоксам (без физики)
pub fn entity_collision_system(
    collider_query: Query<(Entity, &GlobalTransform, &Hitbox, &CollisionLayer)>,
    mut transform_query: Query<&mut Transform>,
) {
    #[derive(Clone, Copy)]
    struct Collider {
        entity: Entity,
        pos: Vec2,
        half: Vec2,
    }

    let mut players: Vec<Collider> = Vec::new();
    let mut pets: Vec<Collider> = Vec::new();
    let mut enemies: Vec<Collider> = Vec::new();
    let mut obstacles: Vec<Collider> = Vec::new();

    for (entity, global_transform, hitbox, layer) in collider_query.iter() {
        let pos = global_transform.translation().truncate();
        let collider = Collider {
            entity,
            pos,
            half: hitbox.half_size,
        };

        match layer.group {
            CollisionLayer::PLAYER => players.push(collider),
            CollisionLayer::PET => pets.push(collider),
            CollisionLayer::ENEMY => enemies.push(collider),
            CollisionLayer::OBSTACLE => obstacles.push(collider),
            _ => {}
        }
    }

    let mut corrections: HashMap<Entity, Vec2> = HashMap::new();

    let mut resolve_pair = |a: Collider, b: Collider, corrections: &mut HashMap<Entity, Vec2>| {
        let delta = b.pos - a.pos;
        let overlap_x = (a.half.x + b.half.x) - delta.x.abs();
        if overlap_x <= 0.0 {
            return;
        }

        let overlap_y = (a.half.y + b.half.y) - delta.y.abs();
        if overlap_y <= 0.0 {
            return;
        }

        let sign_x = if delta.x >= 0.0 { 1.0 } else { -1.0 };
        let sign_y = if delta.y >= 0.0 { 1.0 } else { -1.0 };

        let push = if overlap_x < overlap_y {
            Vec2::new(overlap_x * sign_x, 0.0)
        } else {
            Vec2::new(0.0, overlap_y * sign_y)
        };

        let half_push = push * 0.5;
        *corrections.entry(a.entity).or_insert(Vec2::ZERO) -= half_push;
        *corrections.entry(b.entity).or_insert(Vec2::ZERO) += half_push;
    };

    for player in players.iter().copied() {
        for enemy in enemies.iter().copied() {
            resolve_pair(player, enemy, &mut corrections);
        }
    }

    for pet in pets.iter().copied() {
        for enemy in enemies.iter().copied() {
            resolve_pair(pet, enemy, &mut corrections);
        }
    }

    let resolve_static =
        |static_collider: Collider, movable: Collider, corrections: &mut HashMap<Entity, Vec2>| {
            let delta = movable.pos - static_collider.pos;
            let overlap_x = (static_collider.half.x + movable.half.x) - delta.x.abs();
            if overlap_x <= 0.0 {
                return;
            }

            let overlap_y = (static_collider.half.y + movable.half.y) - delta.y.abs();
            if overlap_y <= 0.0 {
                return;
            }

            let sign_x = if delta.x >= 0.0 { 1.0 } else { -1.0 };
            let sign_y = if delta.y >= 0.0 { 1.0 } else { -1.0 };

            let push = if overlap_x < overlap_y {
                Vec2::new(overlap_x * sign_x, 0.0)
            } else {
                Vec2::new(0.0, overlap_y * sign_y)
            };

            *corrections.entry(movable.entity).or_insert(Vec2::ZERO) += push;
        };

    for enemy in enemies.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, enemy, &mut corrections);
        }
    }

    for player in players.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, player, &mut corrections);
        }
    }

    for pet in pets.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, pet, &mut corrections);
        }
    }

    for (entity, correction) in corrections.into_iter() {
        if correction == Vec2::ZERO {
            continue;
        }
        if let Ok(mut transform) = transform_query.get_mut(entity) {
            transform.translation.x += correction.x;
            transform.translation.y += correction.y;
        }
    }
}
