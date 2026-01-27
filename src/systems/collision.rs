use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{CollisionLayer, Hitbox};

/// Система разделения сущностей по хитбоксам (без физики)
pub fn entity_collision_system(
    mut query: Query<(Entity, &mut Transform, &Hitbox, &CollisionLayer)>,
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

    for (entity, transform, hitbox, layer) in query.iter() {
        let collider = Collider {
            entity,
            pos: transform.translation.truncate(),
            half: hitbox.half_size,
        };

        match layer.group {
            CollisionLayer::PLAYER => players.push(collider),
            CollisionLayer::PET => pets.push(collider),
            CollisionLayer::ENEMY => enemies.push(collider),
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

    for (entity, mut transform, _, _) in query.iter_mut() {
        if let Some(correction) = corrections.get(&entity) {
            if *correction != Vec2::ZERO {
                transform.translation.x += correction.x;
                transform.translation.y += correction.y;
            }
        }
    }
}
