use crate::components::{
    DeathAnimation, Enemy, MovementSpeed, Player, SlowEffect, Target, Velocity,
};
use bevy::prelude::*;

/// Система обработки ввода игрока (WASD/Arrow keys)
pub fn input_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Velocity, &MovementSpeed), With<Player>>,
) {
    for (mut velocity, speed) in query.iter_mut() {
        let mut direction = Vec2::ZERO;

        // WASD
        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }

        // Нормализуем направление и применяем скорость
        if direction != Vec2::ZERO {
            direction = direction.normalize();
        }

        velocity.0 = direction * speed.0;
    }
}

// УДАЛЕНО: Система применения скорости к позиции
// Функциональность перенесена в physics_update_system (src/systems/physics.rs)
// который использует fixed timestep и работает с PhysicsPosition вместо Transform
// pub fn movement_system(time: Res<Time>, mut query: Query<(&mut Transform, &Velocity)>) {
//     for (mut transform, velocity) in query.iter_mut() {
//         transform.translation.x += velocity.0.x * time.delta_secs();
//         transform.translation.y += velocity.0.y * time.delta_secs();
//     }
// }

/// Система AI врагов - преследование цели (игрока)
pub fn enemy_ai_system(
    target_query: Query<&Transform, With<Player>>,
    mut enemy_query: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &MovementSpeed,
            &Target,
            Option<&SlowEffect>,
        ),
        (With<Enemy>, Without<Player>, Without<DeathAnimation>),
    >,
) {
    // Получаем позицию игрока
    if let Ok(player_transform) = target_query.single() {
        for (_entity, enemy_transform, mut velocity, speed, _target, slow_opt) in
            enemy_query.iter_mut()
        {
            // Вычисляем направление к игроку
            let direction = Vec2::new(
                player_transform.translation.x - enemy_transform.translation.x,
                player_transform.translation.y - enemy_transform.translation.y,
            );

            // Применяем эффект замедления, если есть
            let effective_speed = if let Some(slow_effect) = slow_opt {
                speed.0 * slow_effect.slow_amount
            } else {
                speed.0
            };

            // Нормализуем и применяем скорость
            if direction.length() > 0.0 {
                velocity.0 = direction.normalize() * effective_speed;
            } else {
                velocity.0 = Vec2::ZERO;
            }
        }
    }
}

/// Разворот врагов в сторону движения
pub fn enemy_facing_system(mut enemy_query: Query<(&Velocity, &mut Sprite), With<Enemy>>) {
    for (velocity, mut sprite) in enemy_query.iter_mut() {
        if velocity.0.x < -0.05 {
            sprite.flip_x = false;
        } else if velocity.0.x > 0.05 {
            sprite.flip_x = true;
        }
    }
}
