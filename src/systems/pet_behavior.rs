use bevy::prelude::*;
use std::collections::HashMap;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackRange, AttackTarget, AttackTimer, Damage,
    DetectionRange, EffectSprite, Enemy, Pet, PetMovementSpeed, PetType, Player, Projectile, Team,
    TimedDespawn, Velocity,
};
use crate::resources::{PetSpriteSheet, UpgradeState};
use crate::systems::spawn_hit_particles;

/// Система AI питомцев - поиск и преследование врагов
pub fn pet_ai_system(
    time: Res<Time>,
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut pet_query: Query<
        (
            Entity,
            &Transform,
            &mut Velocity,
            &DetectionRange,
            &AttackRange,
            &PetMovementSpeed,
            Option<&AttackTarget>,
        ),
        Without<Player>,
    >,
    enemy_query: Query<(Entity, &Transform), (With<Enemy>, Without<Player>, Without<Pet>)>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let enemies: Vec<(Entity, Vec2)> = enemy_query
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();
    let mut target_counts: HashMap<Entity, u32> = HashMap::new();

    for (
        pet_entity,
        pet_transform,
        mut velocity,
        detection_range,
        attack_range,
        movement_speed,
        current_target,
    ) in pet_query.iter_mut()
    {
        let pet_pos = pet_transform.translation.truncate();

        // Ищем ближайшего врага в радиусе обнаружения, распределяя цели между питомцами
        let target_penalty = detection_range.0 * 0.4;
        let stickiness = target_penalty * 0.3;
        let mut best_enemy: Option<(Entity, Vec2, f32)> = None;
        let mut best_score = f32::INFINITY;

        for (enemy_entity, enemy_pos) in enemies.iter() {
            let distance = pet_pos.distance(*enemy_pos);

            if distance <= detection_range.0 {
                let assigned = *target_counts.get(enemy_entity).unwrap_or(&0) as f32;
                let mut score = distance + (assigned * target_penalty);

                if let Some(target) = current_target {
                    if target.target_entity == *enemy_entity {
                        score -= stickiness;
                    }
                }

                if score < best_score {
                    best_score = score;
                    best_enemy = Some((*enemy_entity, *enemy_pos, distance));
                }
            }
        }

        // ПРИОРИТЕТ: Враги > Следование за игроком
        let target_velocity = if let Some((enemy_entity, enemy_pos, distance)) = best_enemy {
            let entry = target_counts.entry(enemy_entity).or_insert(0);
            *entry += 1;
            commands.entity(pet_entity).insert(AttackTarget {
                target_entity: enemy_entity,
            });
            // Враг найден - преследуем до радиуса атаки, затем останавливаемся
            if distance > attack_range.0 {
                // Враг вне радиуса атаки - двигаемся к нему на полной скорости
                let direction = (enemy_pos - pet_pos).normalize_or_zero();
                direction * movement_speed.0
            } else {
                // В радиусе атаки - стоим на месте и атакуем
                Vec2::ZERO
            }
        } else {
            if current_target.is_some() {
                commands.entity(pet_entity).remove::<AttackTarget>();
            }
            // Врагов НЕТ - только тогда следуем за игроком
            let player_pos = player_transform.translation.truncate();
            let distance_to_player = pet_pos.distance(player_pos);

            let follow_distance = 120.0;
            let stop_distance = 50.0;

            if distance_to_player > follow_distance {
                // Далеко от игрока - движемся на полной скорости
                let direction = (player_pos - pet_pos).normalize_or_zero();
                direction * movement_speed.0
            } else if distance_to_player > stop_distance {
                // Близко к игроку - замедляемся плавно
                let direction = (player_pos - pet_pos).normalize_or_zero();
                let speed_factor =
                    (distance_to_player - stop_distance) / (follow_distance - stop_distance);
                direction * movement_speed.0 * speed_factor
            } else {
                // Очень близко - останавливаемся
                Vec2::ZERO
            }
        };

        // Используем smooth_nudge для плавного следования (лучшая практика Bevy)
        // decay_rate = ln(10) означает: через 1 секунду остается 1/10 расстояния
        let decay_rate = 10.0; // Быстрая реакция
        velocity
            .0
            .smooth_nudge(&target_velocity, decay_rate, time.delta_secs());
    }
}

/// Система обновления таймеров атаки питомцев
pub fn pet_attack_timer_system(time: Res<Time>, mut pet_query: Query<&mut AttackTimer, With<Pet>>) {
    for mut timer in pet_query.iter_mut() {
        timer.timer.tick(time.delta());
    }
}

/// Система атаки питомцев снарядами (для всех типов)
pub fn pet_projectile_attack_system(
    mut commands: Commands,
    mut pet_query: Query<(
        &Transform,
        &Pet,
        &Damage,
        &AttackRange,
        &mut AttackTimer,
        &Team,
        Option<&AttackTarget>,
    )>,
    enemy_query: Query<(Entity, &Transform, &Team), With<Enemy>>,
    upgrade_state: Res<UpgradeState>,
    pet_sprites: Res<PetSpriteSheet>,
) {
    let mut target_counts: HashMap<Entity, u32> = HashMap::new();

    for (pet_transform, pet, damage, attack_range, mut attack_timer, pet_team, attack_target) in
        pet_query.iter_mut()
    {
        // Проверяем, готов ли питомец к атаке
        if !attack_timer.timer.is_finished() {
            continue;
        }

        if matches!(pet.pet_type, PetType::GuardDog | PetType::SlimeCompanion) {
            continue;
        }

        let mut chosen_target: Option<(Entity, Vec2)> = None;

        if let Some(target) = attack_target {
            if let Ok((enemy_entity, enemy_transform, enemy_team)) =
                enemy_query.get(target.target_entity)
            {
                if pet_team.0 != enemy_team.0 {
                    let distance = pet_transform
                        .translation
                        .distance(enemy_transform.translation);
                    if distance <= attack_range.0 {
                        chosen_target =
                            Some((enemy_entity, enemy_transform.translation.truncate()));
                    }
                }
            }
        }

        if chosen_target.is_none() {
            // Ищем врага в радиусе атаки, распределяя цели между питомцами
            let target_penalty = attack_range.0 * 0.75;
            let mut best_enemy: Option<(Entity, Vec2, f32)> = None;
            let mut best_score = f32::INFINITY;

            for (enemy_entity, enemy_transform, enemy_team) in enemy_query.iter() {
                if pet_team.0 == enemy_team.0 {
                    continue;
                }

                let distance = pet_transform
                    .translation
                    .distance(enemy_transform.translation);

                if distance <= attack_range.0 {
                    let enemy_pos = enemy_transform.translation.truncate();
                    let assigned = *target_counts.get(&enemy_entity).unwrap_or(&0) as f32;
                    let score = distance + (assigned * target_penalty);

                    if score < best_score {
                        best_score = score;
                        best_enemy = Some((enemy_entity, enemy_pos, distance));
                    }
                }
            }

            if let Some((enemy_entity, enemy_pos, _)) = best_enemy {
                chosen_target = Some((enemy_entity, enemy_pos));
            }
        }

        // Если нашли врага - атакуем
        if let Some((enemy_entity, enemy_pos)) = chosen_target {
            let entry = target_counts.entry(enemy_entity).or_insert(0);
            *entry += 1;
            // Дальнобойные питомцы стреляют снарядами
            let pet_pos = pet_transform.translation.truncate();
            let direction = (enemy_pos - pet_pos).normalize();
            let projectile_speed = 300.0;

            let base_pierce = match pet.pet_type {
                PetType::FireSprite => 1, // Базовое пробивание
                _ => 0,
            };
            let max_pierce = base_pierce + upgrade_state.projectile_pierce_bonus;
            let piercing = max_pierce > 0;

            let projectile_color = match pet.pet_type {
                PetType::FireSprite => Color::srgb(1.0, 0.6, 0.2),
                PetType::CrowScout => Color::srgb(0.8, 0.8, 0.8),
                _ => Color::WHITE,
            };
            let projectile_sheet = &pet_sprites.arrow;
            let projectile_scale = 0.15;

            let total_shots = 1 + upgrade_state.projectile_extra_shots;
            let spread = 0.35;

            for shot_index in 0..total_shots {
                let angle = if total_shots == 1 {
                    0.0
                } else {
                    let t = shot_index as f32 / (total_shots - 1) as f32;
                    (-spread * 0.5) + (spread * t)
                };
                let shot_direction = rotate_vec2(direction, angle);
                let rotation = shot_direction.y.atan2(shot_direction.x);

                commands.spawn((
                    Projectile {
                        velocity: shot_direction * projectile_speed,
                        damage: damage.0,
                        lifetime: Timer::from_seconds(2.0, TimerMode::Once),
                        piercing,
                        pierced_count: 0,
                        max_pierce,
                        area_radius: upgrade_state.area_damage_radius,
                    },
                    *pet_team,
                    Sprite {
                        image: projectile_sheet.texture.clone(),
                        texture_atlas: Some(TextureAtlas {
                            layout: projectile_sheet.layout.clone(),
                            index: projectile_sheet.first,
                        }),
                        color: projectile_color,
                        ..default()
                    },
                    Transform::from_xyz(pet_pos.x, pet_pos.y, 0.5)
                        .with_rotation(Quat::from_rotation_z(rotation))
                        .with_scale(Vec3::splat(projectile_scale)),
                ));
            }

            // Сбрасываем таймер
            attack_timer.timer.reset();
        }
    }
}

fn rotate_vec2(vec: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(vec.x * cos - vec.y * sin, vec.x * sin + vec.y * cos)
}

pub fn spawn_slime_heal_effect(
    commands: &mut Commands,
    position: Vec2,
    pet_sprites: &PetSpriteSheet,
) {
    let sheet = &pet_sprites.heal_effect;
    let frame_time = 0.08;
    let frames = (sheet.last - sheet.first + 1) as f32;
    let duration = frame_time * frames;

    commands.spawn((
        EffectSprite,
        TimedDespawn {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        },
        AnimationIndices {
            first: sheet.first,
            last: sheet.last,
        },
        AnimationTimer(Timer::from_seconds(frame_time, TimerMode::Repeating)),
        Sprite {
            image: sheet.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: sheet.layout.clone(),
                index: sheet.first,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.7).with_scale(Vec3::splat(0.35)),
    ));
}

/// Система движения снарядов
pub fn projectile_movement_system(
    mut commands: Commands,
    time: Res<Time>,
    mut projectile_query: Query<(Entity, &mut Transform, &mut Projectile)>,
) {
    for (entity, mut transform, mut projectile) in projectile_query.iter_mut() {
        // Двигаем снаряд
        transform.translation += projectile.velocity.extend(0.0) * time.delta_secs();

        // Тикаем таймер жизни
        projectile.lifetime.tick(time.delta());

        // Удаляем снаряд, если время вышло
        if projectile.lifetime.is_finished() {
            commands
                .entity(entity)
                .queue_silenced(|entity: bevy::ecs::world::EntityWorldMut| {
                    entity.despawn();
                });
        }
    }
}

/// Система столкновений снарядов с врагами
pub fn projectile_collision_system(
    mut commands: Commands,
    mut damage_events: MessageWriter<crate::systems::combat::DamageEvent>,
    mut projectile_query: Query<(Entity, &Transform, &mut Projectile, &Team)>,
    enemy_query: Query<(Entity, &Transform, &Team), With<Enemy>>,
) {
    for (projectile_entity, projectile_transform, mut projectile, projectile_team) in
        projectile_query.iter_mut()
    {
        for (enemy_entity, enemy_transform, enemy_team) in enemy_query.iter() {
            // Проверяем команды
            if projectile_team.0 == enemy_team.0 {
                continue;
            }

            // Проверяем столкновение (простая проверка расстояния)
            let distance = projectile_transform
                .translation
                .distance(enemy_transform.translation);
            let collision_radius = 20.0; // Радиус столкновения

            if distance <= collision_radius {
                // Наносим урон
                damage_events.write(crate::systems::combat::DamageEvent {
                    target: enemy_entity,
                    damage: projectile.damage,
                });

                let impact_pos = enemy_transform.translation.truncate();
                spawn_hit_particles(&mut commands, impact_pos, Color::srgb(1.0, 0.8, 0.4), 6);

                if projectile.area_radius > 0.0 {
                    for (splash_entity, splash_transform, splash_team) in enemy_query.iter() {
                        if splash_entity == enemy_entity || projectile_team.0 == splash_team.0 {
                            continue;
                        }
                        let splash_distance =
                            impact_pos.distance(splash_transform.translation.truncate());
                        if splash_distance <= projectile.area_radius {
                            damage_events.write(crate::systems::combat::DamageEvent {
                                target: splash_entity,
                                damage: projectile.damage,
                            });
                        }
                    }
                }

                // Проверяем пробивание
                if projectile.piercing && projectile.pierced_count < projectile.max_pierce {
                    projectile.pierced_count += 1;
                } else {
                    // Удаляем снаряд
                    commands.entity(projectile_entity).queue_silenced(
                        |entity: bevy::ecs::world::EntityWorldMut| {
                            entity.despawn();
                        },
                    );
                    break; // Выходим из цикла, так как снаряд уничтожен
                }
            }
        }
    }
}
