use crate::components::{
    AttackAnimation, AttackRange, AttackTimer, Boss, Damage, DeathAnimation, Enemy, FloatingText,
    Gold, GoldHighlightTimer, GoldPickup, Health, HitFlash, Hitbox, MovementSpeed, PendingAttack,
    Pet, PetType, Player, SlowEffect, Target, Team, Velocity, XpGem,
};
use crate::constants::{
    AREA_DAMAGE_CONE_ANGLE, AREA_DAMAGE_CONE_ANGLE_MAX, DAMAGE_TEXT_DURATION, GOLD_SCALE,
    UI_FONT_SCALE, XP_GEM_SCALE,
};
use crate::resources::{
    GoldSprites, MetaProgression, PetSpriteSheet, PlayerDamageFlash, ScreenShake, UiFonts,
    UpgradeState, XpGemSprites,
};
use crate::systems::{
    is_within_cone, spawn_area_damage_visual, spawn_hit_particles, spawn_slime_heal_effect,
};
use crate::ui::{AreaDamageVisualAssets, HitboxVisualsVisible};
use bevy::prelude::*;
use rand::Rng;

/// Генерирует случайное смещение от центральной позиции используя полярные координаты
fn random_circular_offset(rng: &mut impl Rng, min_distance: f32, max_distance: f32) -> Vec2 {
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let distance = rng.gen_range(min_distance..max_distance);
    Vec2::new(angle.cos(), angle.sin()) * distance
}

/// Событие урона
#[derive(Message)]
pub struct DamageEvent {
    pub target: Entity,
    pub damage: f32,
}

/// Система обнаружения столкновений - питомцы атакуют врагов (для Guard Dog и Slime)
pub fn collision_system(
    mut commands: Commands,
    mut pet_query: Query<(
        Entity,
        &Transform,
        &Damage,
        &AttackRange,
        &mut AttackTimer,
        &Team,
        &Pet,
        Option<&PendingAttack>,
    )>,
    enemy_query: Query<(Entity, &Transform, &Team), (With<Enemy>, Without<DeathAnimation>)>,
    upgrade_state: Res<UpgradeState>,
) {
    for (
        pet_entity,
        pet_transform,
        damage,
        attack_range,
        mut attack_timer,
        pet_team,
        pet,
        pending_attack,
    ) in pet_query.iter_mut()
    {
        if pending_attack.is_some() {
            continue;
        }
        // Только Guard Dog и Slime атакуют напрямую (остальные используют снаряды)
        match pet.pet_type {
            PetType::GuardDog | PetType::SlimeCompanion => {}
            _ => continue,
        }
        // Проверяем, готова ли атака
        if !attack_timer.timer.just_finished() {
            continue;
        }

        // Ищем ближайшего врага в радиусе атаки
        let mut closest_enemy: Option<(Entity, Vec2, f32)> = None;

        for (enemy_entity, enemy_transform, enemy_team) in enemy_query.iter() {
            // Проверяем, что это враждебная команда
            if pet_team.0 == enemy_team.0 {
                continue;
            }

            // Вычисляем расстояние
            let distance = pet_transform
                .translation
                .distance(enemy_transform.translation);

            if distance <= attack_range.0 {
                if let Some((_, _, closest_dist)) = closest_enemy {
                    if distance < closest_dist {
                        closest_enemy = Some((
                            enemy_entity,
                            enemy_transform.translation.truncate(),
                            distance,
                        ));
                    }
                } else {
                    closest_enemy = Some((
                        enemy_entity,
                        enemy_transform.translation.truncate(),
                        distance,
                    ));
                }
            }
        }

        // Если нашли врага - атакуем
        if let Some((enemy_entity, _enemy_pos, _)) = closest_enemy {
            let attack_duration = attack_timer.timer.duration().as_secs_f32();
            if pet.pet_type == PetType::GuardDog {
                commands
                    .entity(pet_entity)
                    .insert(AttackAnimation::new(attack_duration));
            }
            commands.entity(pet_entity).insert(PendingAttack {
                target: enemy_entity,
                damage: damage.0,
                timer: Timer::from_seconds(attack_duration, TimerMode::Once),
                area_radius: upgrade_state.area_damage_radius,
                attack_range: attack_range.0,
                team: *pet_team,
                hit_particles: 5,
                hit_color: Color::srgb(0.6, 0.9, 0.6),
                apply_slow: pet.pet_type == PetType::SlimeCompanion,
                slime_heal_effect: pet.pet_type == PetType::SlimeCompanion,
                screen_shake: None,
            });

            // КРИТИЧЕСКИ ВАЖНО: Сбрасываем таймер атаки!
            attack_timer.timer.reset();
        }
    }
}

/// Враги наносят урон игроку вблизи
pub fn enemy_attack_system(
    mut commands: Commands,
    player_query: Query<(Entity, &Transform), With<Player>>,
    enemy_query: Query<
        (
            Entity,
            &Transform,
            &Damage,
            &AttackRange,
            &AttackTimer,
            &Team,
            Option<&Boss>,
            Option<&PendingAttack>,
        ),
        (With<Enemy>, Without<DeathAnimation>),
    >,
) {
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    for (
        enemy_entity,
        enemy_transform,
        damage,
        attack_range,
        attack_timer,
        team,
        boss_opt,
        pending_attack,
    ) in enemy_query.iter()
    {
        if pending_attack.is_some() {
            continue;
        }
        if !attack_timer.timer.just_finished() {
            continue;
        }

        let distance = enemy_transform
            .translation
            .distance(player_transform.translation);
        if distance <= attack_range.0 {
            let attack_duration = attack_timer.timer.duration().as_secs_f32();
            commands
                .entity(enemy_entity)
                .insert(AttackAnimation::new(attack_duration));
            let screen_shake = if boss_opt.is_some() {
                Some((12.0, 0.25))
            } else {
                None
            };
            commands.entity(enemy_entity).insert(PendingAttack {
                target: player_entity,
                damage: damage.0,
                timer: Timer::from_seconds(attack_duration, TimerMode::Once),
                area_radius: 0.0,
                attack_range: attack_range.0,
                team: *team,
                hit_particles: 0,
                hit_color: Color::WHITE,
                apply_slow: false,
                slime_heal_effect: false,
                screen_shake,
            });
        }
    }
}

pub fn pending_attack_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    mut pending_query: Query<(Entity, &mut PendingAttack)>,
    mut damage_events: MessageWriter<DamageEvent>,
    transform_query: Query<&Transform>,
    enemy_query: Query<(Entity, &Transform, &Team), With<Enemy>>,
    pet_query: Query<&Pet>,
    mut screen_shake: ResMut<ScreenShake>,
    pet_sprites: Res<PetSpriteSheet>,
    hitbox_visuals: Res<HitboxVisualsVisible>,
    area_visuals: Res<AreaDamageVisualAssets>,
) {
    for (attacker_entity, mut pending) in pending_query.iter_mut() {
        pending.timer.tick(time.delta());
        if !pending.timer.just_finished() {
            continue;
        }

        let Ok(attacker_transform) = transform_query.get(attacker_entity) else {
            commands.entity(attacker_entity).remove::<PendingAttack>();
            continue;
        };
        let Ok(target_transform) = transform_query.get(pending.target) else {
            commands.entity(attacker_entity).remove::<PendingAttack>();
            continue;
        };
        let distance = attacker_transform
            .translation
            .distance(target_transform.translation);
        if distance > pending.attack_range {
            commands.entity(attacker_entity).remove::<PendingAttack>();
            continue;
        }

        damage_events.write(DamageEvent {
            target: pending.target,
            damage: pending.damage,
        });

        let attacker_pos = attacker_transform.translation.truncate();
        let target_pos = target_transform.translation.truncate();
        let attack_direction = (target_pos - attacker_pos).normalize_or_zero();

        if pending.hit_particles > 0 {
            spawn_hit_particles(
                &mut commands,
                target_pos,
                pending.hit_color,
                pending.hit_particles,
            );
        }

        if pending.slime_heal_effect {
            spawn_slime_heal_effect(&mut commands, target_pos, &pet_sprites);
        }

        if pending.area_radius > 0.0 {
            let base_attack_range = pet_query
                .get(attacker_entity)
                .map(|pet| {
                    let (_, _, range, _, _) = pet.pet_type.get_stats();
                    range
                })
                .unwrap_or(pending.attack_range);
            let scale = if base_attack_range > 0.0 {
                pending.attack_range / base_attack_range
            } else {
                1.0
            };
            let scaled_area = pending.area_radius * scale;
            let cone_length = scaled_area.min(pending.attack_range);
            let cone_angle = if base_attack_range > 0.0 {
                (AREA_DAMAGE_CONE_ANGLE * (1.0 + (pending.area_radius / base_attack_range)))
                    .min(AREA_DAMAGE_CONE_ANGLE_MAX)
            } else {
                AREA_DAMAGE_CONE_ANGLE
            };
            if hitbox_visuals.0 {
                spawn_area_damage_visual(
                    &mut commands,
                    &mut meshes,
                    &area_visuals,
                    attacker_pos,
                    cone_length,
                    attack_direction,
                    cone_angle,
                );
            }
            for (splash_entity, splash_transform, splash_team) in enemy_query.iter() {
                if splash_entity == pending.target || pending.team.0 == splash_team.0 {
                    continue;
                }
                let splash_offset = splash_transform.translation.truncate() - attacker_pos;
                let splash_distance = splash_offset.length();
                if splash_distance <= cone_length
                    && is_within_cone(attack_direction, splash_offset, cone_angle)
                {
                    damage_events.write(DamageEvent {
                        target: splash_entity,
                        damage: pending.damage,
                    });
                }
            }
        }

        if pending.apply_slow {
            commands.entity(pending.target).queue_silenced(
                |mut entity: bevy::ecs::world::EntityWorldMut| {
                    entity.insert(SlowEffect {
                        slow_amount: 0.5,
                        duration: Timer::from_seconds(2.0, TimerMode::Once),
                    });
                },
            );
        }

        if let Some((intensity, duration)) = pending.screen_shake {
            screen_shake.trigger(intensity, duration);
        }

        commands.entity(attacker_entity).remove::<PendingAttack>();
    }
}

/// Система применения урона и проверки смертей
pub fn damage_system(
    mut commands: Commands,
    mut damage_events: MessageReader<DamageEvent>,
    mut health_query: Query<(&mut Health, &Transform, Option<&Hitbox>)>,
    enemy_query: Query<(&Enemy, Entity, Option<&Boss>), With<Enemy>>,
    player_query: Query<(Entity, &Gold), With<Player>>,
    mut sprite_query: Query<(Entity, &mut Sprite, Option<&mut HitFlash>)>,
    upgrade_state: Res<UpgradeState>,
    ui_fonts: Res<UiFonts>,
    mut meta: ResMut<MetaProgression>,
    gold_sprites: Res<GoldSprites>,
    xp_gem_sprites: Res<XpGemSprites>,
    mut player_damage_flash: ResMut<PlayerDamageFlash>,
) {
    for event in damage_events.read() {
        if let Ok((mut health, transform, hitbox)) = health_query.get_mut(event.target) {
            let is_player = player_query.get(event.target).is_ok();
            let enemy_info = enemy_query.get(event.target).ok();

            if health.current <= 0.0 {
                continue;
            }
            health.current -= event.damage;
            if health.current < 0.0 {
                health.current = 0.0;
            }

            if is_player && event.damage > 0.0 {
                player_damage_flash.trigger(1.0, 0.35);
            }
            let target_pos = transform.translation;

            let damage_color = if is_player {
                Color::srgb(1.0, 0.1, 0.1)
            } else {
                Color::srgb(1.0, 0.9, 0.2)
            };
            let text_offset_y = if is_player {
                hitbox.map_or(40.0, |hitbox| hitbox.half_size.y + 12.0)
            } else {
                12.0
            };
            let font_size = if is_player { 20.0 } else { 16.0 } * UI_FONT_SCALE;

            commands.spawn((
                Text2d::new(format!("-{:.0}", event.damage)),
                TextFont {
                    font: ui_fonts.main.clone(),
                    font_size,
                    ..default()
                },
                TextColor(damage_color),
                TextLayout::new_with_justify(Justify::Center),
                Transform::from_xyz(target_pos.x, target_pos.y + text_offset_y, 5.0),
                FloatingText {
                    timer: Timer::from_seconds(DAMAGE_TEXT_DURATION, TimerMode::Once),
                    velocity: Vec2::new(0.0, 40.0),
                },
            ));

            if let Ok((entity, mut sprite, hit_flash_opt)) = sprite_query.get_mut(event.target) {
                if let Some(mut hit_flash) = hit_flash_opt {
                    sprite.color = Color::WHITE;
                    hit_flash.timer.reset();
                } else {
                    let original_color = sprite.color;
                    sprite.color = Color::WHITE;
                    commands
                        .entity(entity)
                        .insert(HitFlash::new(0.1, original_color));
                }
            }

            // Проверка смерти
            if health.current <= 0.0 {
                // Если это враг - удаляем и дропаем XP
                if let Some((enemy, _entity, boss_opt)) = enemy_info {
                    // Если это босс - используем награды босса
                    let (xp_reward, gold_reward) = if let Some(boss) = boss_opt {
                        let (_, _, _, boss_xp) = boss.boss_type.get_stats();
                        let boss_gold = boss.boss_type.get_gold_reward();
                        (boss_xp, boss_gold)
                    } else {
                        let (_, _, _, enemy_xp) = enemy.enemy_type.get_stats();
                        let enemy_gold = enemy.enemy_type.get_gold_reward();
                        (enemy_xp, enemy_gold)
                    };

                    // Применяем множители золота: из апгрейдов и постоянных улучшений (§3.2.3)
                    let total_gold_mult = upgrade_state.gold_drop_mult
                        * meta.save_data.permanent_upgrades.get_gold_multiplier();
                    let gold_reward = ((gold_reward as f32) * total_gold_mult).round() as u32;

                    // Инициализация RNG для случайных позиций дропа
                    let mut rng = rand::thread_rng();

                    // Базовая позиция смерти
                    let death_pos = transform.translation.truncate();

                    // Генерация случайных смещений для XP и золота (разные позиции)
                    let xp_offset = random_circular_offset(&mut rng, 20.0, 50.0);
                    let gold_offset = random_circular_offset(&mut rng, 20.0, 50.0);

                    // Дроп XP гема на месте смерти врага (со случайным смещением)
                    commands.spawn((
                        XpGem { value: xp_reward },
                        Sprite {
                            image: xp_gem_sprites.texture.clone(),
                            ..default()
                        },
                        Transform::from_xyz(
                            death_pos.x + xp_offset.x,
                            death_pos.y + xp_offset.y,
                            0.5,
                        )
                        .with_scale(Vec3::splat(XP_GEM_SCALE)),
                    ));

                    // Дроп золота на месте смерти врага (со случайным смещением)
                    commands.spawn((
                        GoldPickup { value: gold_reward },
                        Sprite {
                            image: gold_sprites.normal_texture.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: gold_sprites.normal_layout.clone(),
                                index: gold_sprites.normal_index,
                            }),
                            ..default()
                        },
                        Transform::from_xyz(
                            death_pos.x + gold_offset.x,
                            death_pos.y + gold_offset.y,
                            0.5,
                        )
                        .with_scale(Vec3::splat(GOLD_SCALE)),
                        GoldHighlightTimer {
                            timer: Timer::from_seconds(3.0, TimerMode::Once),
                            is_highlighted: false,
                        },
                    ));

                    let death_particle_count = if boss_opt.is_some() { 24 } else { 12 };
                    spawn_hit_particles(
                        &mut commands,
                        transform.translation.truncate(),
                        Color::srgb(0.9, 0.1, 0.1),
                        death_particle_count,
                    );

                    // Добавляем компонент анимации смерти (0.8 сек для 8 кадров)
                    commands
                        .entity(event.target)
                        .insert(DeathAnimation::new(0.8));

                    // Удаляем компоненты движения и атаки, чтобы враг "замер"
                    commands.entity(event.target).queue_silenced(
                        |mut entity: bevy::ecs::world::EntityWorldMut| {
                            entity.remove::<Velocity>();
                            entity.remove::<MovementSpeed>();
                            entity.remove::<AttackTimer>();
                            entity.remove::<AttackRange>();
                            entity.remove::<Target>();
                            entity.remove::<Damage>();
                        },
                    );
                }

                // Если это игрок - game over
                if is_player {
                    let Ok((_player_entity, player_gold)) = player_query.get(event.target) else {
                        continue;
                    };

                    // Сохраняем золото в метапрогрессию (§3.2.3)
                    meta.save_data.add_gold(player_gold.amount);
                    meta.save_data.total_runs += 1;
                    meta.mark_dirty();

                    // TODO: Реализовать экран game over и возврат в магазин
                    // Пока просто выводим сообщение
                }
            }
        }
    }
}

/// Система тика таймеров атаки
pub fn attack_timer_tick_system(time: Res<Time>, mut query: Query<&mut AttackTimer>) {
    for mut attack_timer in query.iter_mut() {
        attack_timer.timer.tick(time.delta());
    }
}

/// Система обработки эффектов замедления
pub fn slow_effect_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut SlowEffect)>,
) {
    for (entity, mut slow_effect) in query.iter_mut() {
        slow_effect.duration.tick(time.delta());

        if slow_effect.duration.is_finished() {
            // Удаляем компонент замедления
            commands.entity(entity).queue_silenced(
                |mut entity: bevy::ecs::world::EntityWorldMut| {
                    entity.remove::<SlowEffect>();
                },
            );
        }
    }
}
