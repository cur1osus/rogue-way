use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::*;
use crate::constants::{ENEMY_SOFT_COLLISION_SCALE, PET_SOFT_COLLISION_SCALE};
use crate::resources::{PhysicsAccumulator, FIXED_TIMESTEP};

/// Основная физическая система с fixed timestep
pub fn physics_update_system(
    time: Res<Time>,
    mut accumulator: ResMut<PhysicsAccumulator>,
    mut query: Query<(
        Entity,
        &mut PhysicsPosition,
        &mut PreviousPhysicsPosition,
        &Velocity,
        &Hitbox,
        &CollisionLayer,
    )>,
) {
    // Накапливаем время
    accumulator.accumulator += time.delta_secs();

    // Выполняем fixed timestep обновления
    while accumulator.accumulator >= FIXED_TIMESTEP {
        accumulator.accumulator -= FIXED_TIMESTEP;

        // Шаг 1: Сохраняем предыдущие позиции для интерполяции
        for (_, physics_pos, mut prev_pos, _, _, _) in query.iter_mut() {
            prev_pos.0 = physics_pos.0;
        }

        // Шаг 2: Применяем velocity к физической позиции
        for (_, mut physics_pos, _, velocity, _, _) in query.iter_mut() {
            physics_pos.0 += velocity.0 * FIXED_TIMESTEP;
        }

        // Шаг 3: Собираем коллайдеры для проверки (read-only проход)
        let colliders: Vec<(Entity, Vec2, Vec2, u32)> = query
            .iter()
            .map(|(entity, physics_pos, _, _, hitbox, layer)| {
                (entity, physics_pos.0, hitbox.half_size, layer.group)
            })
            .collect();

        // Шаг 4: Вычисляем коррекции коллизий
        let corrections = compute_collision_corrections(&colliders);

        // Шаг 5: Применяем коррекции
        for (entity, mut physics_pos, _, _, _, _) in query.iter_mut() {
            if let Some(&correction) = corrections.get(&entity) {
                if correction != Vec2::ZERO {
                    physics_pos.0 += correction;
                }
            }
        }
    }
}

/// Вычисляет коррекции для коллизий на основе списка коллайдеров
/// Возвращает HashMap с коррекциями для каждой сущности
fn compute_collision_corrections(colliders: &[(Entity, Vec2, Vec2, u32)]) -> HashMap<Entity, Vec2> {
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

    // Собираем коллайдеры по категориям
    for &(entity, pos, half_size, layer_group) in colliders.iter() {
        let collider = Collider {
            entity,
            pos,
            half: half_size,
        };

        match layer_group {
            CollisionLayer::PLAYER => players.push(collider),
            CollisionLayer::PET => pets.push(collider),
            CollisionLayer::ENEMY => enemies.push(collider),
            CollisionLayer::OBSTACLE => obstacles.push(collider),
            _ => {}
        }
    }

    let mut corrections: HashMap<Entity, Vec2> = HashMap::new();

    // Функция разрешения пары коллизий
    let resolve_pair = |a: Collider,
                        b: Collider,
                        scale_a: f32,
                        scale_b: f32,
                        corrections: &mut HashMap<Entity, Vec2>| {
        let delta = b.pos - a.pos;
        let half_a = a.half * scale_a;
        let half_b = b.half * scale_b;
        let overlap_x = (half_a.x + half_b.x) - delta.x.abs();
        if overlap_x <= 0.0 {
            return;
        }

        let overlap_y = (half_a.y + half_b.y) - delta.y.abs();
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

    // Коллизии игрок-враги
    for player in players.iter().copied() {
        for enemy in enemies.iter().copied() {
            resolve_pair(player, enemy, 1.0, 1.0, &mut corrections);
        }
    }

    // Коллизии питомец-враги
    for pet in pets.iter().copied() {
        for enemy in enemies.iter().copied() {
            resolve_pair(pet, enemy, 1.0, 1.0, &mut corrections);
        }
    }

    // Коллизии враг-враг (мягкие)
    for i in 0..enemies.len() {
        for j in (i + 1)..enemies.len() {
            resolve_pair(
                enemies[i],
                enemies[j],
                ENEMY_SOFT_COLLISION_SCALE,
                ENEMY_SOFT_COLLISION_SCALE,
                &mut corrections,
            );
        }
    }

    // Коллизии питомец-питомец (мягкие)
    for i in 0..pets.len() {
        for j in (i + 1)..pets.len() {
            resolve_pair(
                pets[i],
                pets[j],
                PET_SOFT_COLLISION_SCALE,
                PET_SOFT_COLLISION_SCALE,
                &mut corrections,
            );
        }
    }

    // Функция разрешения статических коллизий (препятствия)
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

    // Коллизии враги-препятствия
    for enemy in enemies.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, enemy, &mut corrections);
        }
    }

    // Коллизии игрок-препятствия
    for player in players.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, player, &mut corrections);
        }
    }

    // Коллизии питомцы-препятствия
    for pet in pets.iter().copied() {
        for obstacle in obstacles.iter().copied() {
            resolve_static(obstacle, pet, &mut corrections);
        }
    }

    // Возвращаем коррекции
    corrections
}
