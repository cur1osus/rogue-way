use bevy::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackRange, AttackTarget, AttackTimer, CollisionLayer,
    Damage, DetectionRange, EffectSprite, Enemy, EnemyType, Health, Hitbox, Pet, PetAction,
    PetBlackboard, PetMovementSpeed, PetRole, PetState, PetType, PhysicsPosition, Player,
    Projectile, Team, TimedDespawn, Velocity, XpGem,
};
use crate::constants::{
    enemy_sizes, ENEMY_HITBOX_SCALE, PET_ENGAGE_SLOT_TOLERANCE, PET_HITBOX_SCALE,
    PET_MELEE_ENGAGE_RANGE_MULT,
};
use crate::resources::{PetSpriteSheet, UpgradeState};
use crate::systems::spawn_hit_particles;

#[derive(Clone, Copy)]
struct EnemyInfo {
    entity: Entity,
    pos: Vec2,
    radius: f32,
    enemy_type: EnemyType,
    distance_to_player: f32,
    max_pets: u32,
}

/// Система AI питомцев - поиск и преследование врагов
pub fn pet_ai_system(
    time: Res<Time>,
    mut commands: Commands,
    player_query: Query<(&Transform, &Velocity, &Health), With<Player>>,
    mut pet_query: Query<
        (
            Entity,
            &Pet,
            &Transform,
            &mut Velocity,
            &DetectionRange,
            Option<&AttackRange>,
            &PetMovementSpeed,
            &mut PetBlackboard,
        ),
        Without<Player>,
    >,
    pet_positions_query: Query<(Entity, &Pet, &Transform, Option<&AttackTarget>), With<Pet>>,
    enemy_query: Query<
        (Entity, &Transform, &Hitbox, &Enemy),
        (With<Enemy>, Without<Player>, Without<Pet>),
    >,
    obstacle_query: Query<(&PhysicsPosition, &Hitbox, &CollisionLayer), Without<Pet>>,
    xp_gem_query: Query<(Entity, &Transform), With<XpGem>>,
) {
    let mut player_count = 0usize;
    let mut pos_sum = Vec2::ZERO;
    let mut vel_sum = Vec2::ZERO;
    let mut health_ratio_sum = 0.0;
    for (transform, velocity, health) in player_query.iter() {
        player_count += 1;
        pos_sum += transform.translation.truncate();
        vel_sum += velocity.0;
        let ratio = if health.max > 0.0 {
            (health.current / health.max).clamp(0.0, 1.0)
        } else {
            1.0
        };
        health_ratio_sum += ratio;
    }
    if player_count == 0 {
        return;
    }
    let player_pos = pos_sum / player_count as f32;
    let avg_velocity = vel_sum / player_count as f32;
    let player_dir = if avg_velocity.length_squared() > 1.0 {
        avg_velocity.normalize_or_zero()
    } else {
        Vec2::Y
    };
    let player_right = Vec2::new(-player_dir.y, player_dir.x);
    let player_health_ratio = health_ratio_sum / player_count as f32;

    let mut enemies: Vec<EnemyInfo> = Vec::with_capacity(enemy_query.iter().size_hint().0);
    for (entity, transform, hitbox, enemy) in enemy_query.iter() {
        let pos = transform.translation.truncate();
        let radius = hitbox.half_size.x.max(hitbox.half_size.y);
        enemies.push(EnemyInfo {
            entity,
            pos,
            radius,
            enemy_type: enemy.enemy_type,
            distance_to_player: pos.distance(player_pos),
            max_pets: max_pets_for_enemy(hitbox),
        });
    }

    let mut enemy_lookup: HashMap<Entity, EnemyInfo> = HashMap::with_capacity(enemies.len());
    for enemy in enemies.iter() {
        enemy_lookup.insert(enemy.entity, *enemy);
    }

    let player_focus = enemies
        .iter()
        .min_by(|a, b| {
            a.distance_to_player
                .partial_cmp(&b.distance_to_player)
                .unwrap_or(Ordering::Equal)
        })
        .map(|enemy| enemy.entity);

    let mut obstacles: Vec<(Vec2, Vec2)> = Vec::with_capacity(obstacle_query.iter().size_hint().0);
    for (pos, hitbox, layer) in obstacle_query.iter() {
        if layer.group == CollisionLayer::OBSTACLE {
            obstacles.push((pos.0, hitbox.half_size));
        }
    }

    let mut xp_gems: Vec<Vec2> = Vec::with_capacity(xp_gem_query.iter().size_hint().0);
    for (_, transform) in xp_gem_query.iter() {
        xp_gems.push(transform.translation.truncate());
    }

    let pet_capacity = pet_positions_query.iter().size_hint().0;
    let mut pet_entities: Vec<Entity> = Vec::with_capacity(pet_capacity);
    let mut pet_positions: Vec<(Entity, Vec2)> = Vec::with_capacity(pet_capacity);
    let mut max_pet_size: f32 = 0.0;
    let mut role_groups: HashMap<PetRole, Vec<Entity>> = HashMap::new();
    let mut target_counts: HashMap<Entity, u32> = HashMap::with_capacity(pet_capacity);
    for (entity, pet, pet_transform, attack_target) in pet_positions_query.iter() {
        pet_entities.push(entity);
        pet_positions.push((entity, pet_transform.translation.truncate()));
        max_pet_size = max_pet_size.max(pet.pet_type.get_size());
        role_groups
            .entry(pet.pet_type.get_role())
            .or_default()
            .push(entity);
        if let Some(target) = attack_target {
            let entry = target_counts.entry(target.target_entity).or_insert(0);
            *entry += 1;
        }
    }
    pet_entities.sort_by_key(|entity| entity.index());
    for group in role_groups.values_mut() {
        group.sort_by_key(|entity| entity.index());
    }
    let pet_count = pet_entities.len().max(1);
    let mut pet_order_index: HashMap<Entity, usize> = HashMap::with_capacity(pet_entities.len());
    for (index, entity) in pet_entities.iter().enumerate() {
        pet_order_index.insert(*entity, index);
    }

    let base_spacing = (max_pet_size * 0.4).max(55.0);
    let spacing = base_spacing + (pet_count as f32).sqrt() * 1.5;
    let mut formation_targets: HashMap<Entity, Vec2> = HashMap::with_capacity(pet_entities.len());
    for (role, group) in role_groups.iter() {
        let anchor = player_pos + player_dir * role_anchor_offset(*role);
        for (index, entity) in group.iter().enumerate() {
            let offset = hex_spiral_offset(index + 1, spacing);
            formation_targets.insert(*entity, anchor + offset);
        }
    }
    let separation_radius = (spacing * 0.8).max(55.0);
    let separation_radius_sq = separation_radius * separation_radius;
    let separation_strength = 0.7;
    let enemy_slot_tolerance = PET_ENGAGE_SLOT_TOLERANCE;
    let enemy_slot_tolerance_sq = enemy_slot_tolerance * enemy_slot_tolerance;

    let enemies_near_player = enemies
        .iter()
        .filter(|enemy| enemy.distance_to_player <= 140.0)
        .count();
    let assist_requested = enemies_near_player > 0 && player_health_ratio < 0.45;

    for (
        pet_entity,
        pet,
        pet_transform,
        mut velocity,
        detection_range,
        attack_range,
        movement_speed,
        mut brain,
    ) in pet_query.iter_mut()
    {
        let pet_pos = pet_transform.translation.truncate();
        let role = pet.pet_type.get_role();
        let pet_radius = pet.pet_type.get_size() * PET_HITBOX_SCALE * 0.5;
        let side_hint = if pet_order_index.get(&pet_entity).unwrap_or(&0) % 2 == 0 {
            1.0
        } else {
            -1.0
        };

        let mut separation = Vec2::ZERO;
        for (other_entity, other_pos) in pet_positions.iter() {
            if *other_entity == pet_entity {
                continue;
            }
            let delta = pet_pos - *other_pos;
            let dist_sq = delta.length_squared();
            if dist_sq > 0.0 && dist_sq < separation_radius_sq {
                let dist = dist_sq.sqrt();
                let weight = (separation_radius - dist) / separation_radius;
                separation += (delta / dist) * weight;
            }
        }
        if separation.length_squared() > 1.0 {
            separation = separation.normalize_or_zero();
        }

        if role == PetRole::Collector {
            brain.decision_timer.tick(time.delta());
            brain.action_lock.tick(time.delta());
            brain.state = PetState::Follow;
            brain.action = PetAction::CollectXp;
            brain.target = None;
            commands.entity(pet_entity).remove::<AttackTarget>();

            let detection_range_sq = detection_range.0 * detection_range.0;
            let close_distance_sq = 20.0 * 20.0;
            let mut best_gem: Option<(Vec2, f32)> = None;
            for gem_pos in xp_gems.iter() {
                let distance_sq = pet_pos.distance_squared(*gem_pos);
                if detection_range.0 > 0.0 && distance_sq <= detection_range_sq {
                    if let Some((_, best_distance_sq)) = best_gem {
                        if distance_sq < best_distance_sq {
                            best_gem = Some((*gem_pos, distance_sq));
                        }
                    } else {
                        best_gem = Some((*gem_pos, distance_sq));
                    }
                }
            }

            let target_velocity = if let Some((gem_pos, distance_sq)) = best_gem {
                if distance_sq > close_distance_sq {
                    let direction = (gem_pos - pet_pos).normalize_or_zero();
                    direction * movement_speed.0
                } else {
                    Vec2::ZERO
                }
            } else {
                let target_pos = formation_targets
                    .get(&pet_entity)
                    .copied()
                    .unwrap_or(player_pos);
                let (follow_distance, stop_distance) = follow_distances(role, false);
                arrive_velocity(
                    pet_pos,
                    target_pos,
                    movement_speed.0,
                    follow_distance,
                    stop_distance,
                )
            };

            let mut target_velocity = target_velocity;
            let max_speed = movement_speed.0;
            let desired_dir = target_velocity.normalize_or_zero();
            let obstacle_avoidance = compute_obstacle_avoidance(
                pet_pos,
                desired_dir,
                pet_radius,
                &obstacles,
                side_hint,
                (max_speed * 0.4).max(45.0).min(120.0),
            );
            if separation != Vec2::ZERO {
                target_velocity += separation * max_speed * separation_strength;
            }
            let player_repulsion = compute_player_repulsion(pet_pos, player_pos, pet_radius);
            if player_repulsion != Vec2::ZERO {
                target_velocity += player_repulsion * max_speed * 0.8;
            }
            if obstacle_avoidance != Vec2::ZERO {
                target_velocity += obstacle_avoidance * max_speed * 0.9;
            }
            if target_velocity.length_squared() > max_speed * max_speed {
                target_velocity = target_velocity.normalize_or_zero() * max_speed;
            }

            let decay_rate = 10.0;
            velocity
                .0
                .smooth_nudge(&target_velocity, decay_rate, time.delta_secs());
            continue;
        }

        let Some(attack_range) = attack_range else {
            continue;
        };
        let attack_range_value = attack_range.0;

        if let Some(target) = brain.target {
            if !enemy_lookup.contains_key(&target) {
                brain.target = None;
            }
        }

        brain.decision_timer.tick(time.delta());
        brain.action_lock.tick(time.delta());

        let distance_to_player = pet_pos.distance(player_pos);
        let leash_distance = role_leash_distance(role, detection_range.0);
        let over_leash = distance_to_player > leash_distance;

        let detection_range_sq = detection_range.0 * detection_range.0;
        let mut nearest_enemy: Option<(EnemyInfo, f32)> = None;
        for enemy in enemies.iter() {
            let distance_sq = pet_pos.distance_squared(enemy.pos);
            if distance_sq > detection_range_sq {
                continue;
            }
            if let Some((_, nearest_distance_sq)) = nearest_enemy {
                if distance_sq < nearest_distance_sq {
                    nearest_enemy = Some((*enemy, distance_sq));
                }
            } else {
                nearest_enemy = Some((*enemy, distance_sq));
            }
        }

        let enemy_in_range = nearest_enemy.is_some();
        let ranged_min_range = (attack_range_value * 0.55).max(80.0);
        let ranged_opt_range = (attack_range_value * 0.85).max(ranged_min_range + 20.0);
        let ranged_min_range_sq = ranged_min_range * ranged_min_range;
        let ranged_opt_range_sq = ranged_opt_range * ranged_opt_range;
        let breach = matches!(role, PetRole::Ranged | PetRole::Support)
            && nearest_enemy.as_ref().map_or(false, |(_, distance_sq)| {
                distance_sq.sqrt() < ranged_min_range * 0.9
            });

        let allow_decision =
            brain.decision_timer.just_finished() && brain.action_lock.is_finished();
        if allow_decision {
            let mut desired_state = if over_leash {
                PetState::Regroup
            } else if assist_requested {
                PetState::Assist
            } else if enemy_in_range {
                PetState::Engage
            } else {
                PetState::Follow
            };

            if breach {
                desired_state = PetState::Retreat;
            }

            let (action, target) = match desired_state {
                PetState::Follow => (PetAction::FollowFormation, None),
                PetState::Regroup => (PetAction::Regroup, None),
                PetState::Retreat => (
                    PetAction::Fallback,
                    nearest_enemy.map(|(enemy, _)| enemy.entity),
                ),
                PetState::Assist | PetState::Engage => {
                    let assist_bias = desired_state == PetState::Assist;
                    if let Some(selected) = pick_enemy_for_pet(
                        role,
                        pet_pos,
                        detection_range.0,
                        leash_distance,
                        player_focus,
                        &enemies,
                        &mut target_counts,
                        brain.target,
                        assist_bias,
                    ) {
                        let action = match role {
                            PetRole::Melee => {
                                if assist_bias && selected.distance_to_player < 130.0 {
                                    PetAction::PeelThreat
                                } else {
                                    PetAction::EngageTarget
                                }
                            }
                            PetRole::Ranged | PetRole::Support => PetAction::KeepRange,
                            PetRole::Collector => PetAction::CollectXp,
                        };
                        (action, Some(selected.entity))
                    } else {
                        (PetAction::FollowFormation, None)
                    }
                }
            };

            brain.state = desired_state;
            brain.action = action;
            brain.target = target;
            brain.action_lock =
                Timer::from_seconds(action_lock_duration(role, action), TimerMode::Once);
        }

        match brain.target {
            Some(target) => {
                commands.entity(pet_entity).insert(AttackTarget {
                    target_entity: target,
                });
            }
            None => {
                commands.entity(pet_entity).remove::<AttackTarget>();
            }
        }

        let target_info = brain
            .target
            .and_then(|target| enemy_lookup.get(&target).copied());

        let mut target_velocity = match brain.action {
            PetAction::FollowFormation | PetAction::Regroup => {
                let target_pos = formation_targets
                    .get(&pet_entity)
                    .copied()
                    .unwrap_or(player_pos);
                let regrouping = brain.action == PetAction::Regroup;
                let (follow_distance, stop_distance) = follow_distances(role, regrouping);
                arrive_velocity(
                    pet_pos,
                    target_pos,
                    movement_speed.0,
                    follow_distance,
                    stop_distance,
                )
            }
            PetAction::EngageTarget | PetAction::PeelThreat => {
                if let Some(enemy) = target_info {
                    let slot_index = *pet_order_index.get(&pet_entity).unwrap_or(&0);
                    let base_angle = (slot_index as f32 / pet_count as f32) * std::f32::consts::TAU;
                    let engage_range =
                        pet_engage_range(attack_range_value, pet_radius, enemy.radius);
                    let close_range = (pet_radius + enemy.radius + 8.0).max(24.0);
                    let desired_range = if role == PetRole::Melee {
                        (engage_range * PET_MELEE_ENGAGE_RANGE_MULT).max(close_range)
                    } else {
                        engage_range
                    };
                    let slot_radius = (desired_range - enemy_slot_tolerance).max(close_range);
                    let mut slot_pos =
                        enemy.pos + Vec2::new(base_angle.cos(), base_angle.sin()) * slot_radius;
                    let mut desired_dir = (slot_pos - pet_pos).normalize_or_zero();

                    if separation != Vec2::ZERO {
                        let block = -desired_dir.dot(separation);
                        if block > 0.1 {
                            let cross = desired_dir.x * separation.y - desired_dir.y * separation.x;
                            let sign = if cross >= 0.0 { 1.0 } else { -1.0 };
                            let angle_offset = block.min(1.0) * 0.9;
                            let slot_angle = base_angle + (sign * angle_offset);
                            slot_pos = enemy.pos
                                + Vec2::new(slot_angle.cos(), slot_angle.sin()) * slot_radius;
                            desired_dir = (slot_pos - pet_pos).normalize_or_zero();
                        }
                    }

                    let distance_to_slot_sq = pet_pos.distance_squared(slot_pos);
                    if distance_to_slot_sq > enemy_slot_tolerance_sq {
                        desired_dir * movement_speed.0
                    } else {
                        Vec2::ZERO
                    }
                } else {
                    let target_pos = formation_targets
                        .get(&pet_entity)
                        .copied()
                        .unwrap_or(player_pos);
                    let (follow_distance, stop_distance) = follow_distances(role, false);
                    arrive_velocity(
                        pet_pos,
                        target_pos,
                        movement_speed.0,
                        follow_distance,
                        stop_distance,
                    )
                }
            }
            PetAction::KeepRange => {
                if let Some(enemy) = target_info {
                    let distance_sq = pet_pos.distance_squared(enemy.pos);
                    if distance_sq < ranged_min_range_sq {
                        let to_player = (player_pos - enemy.pos).normalize_or_zero();
                        let mut away_dir = (pet_pos - enemy.pos).normalize_or_zero();
                        if away_dir == Vec2::ZERO {
                            away_dir = Vec2::new(-to_player.y, to_player.x) * side_hint;
                        }
                        if away_dir.dot(to_player) > 0.5 {
                            away_dir = Vec2::new(-to_player.y, to_player.x) * side_hint;
                        }
                        away_dir * movement_speed.0
                    } else if distance_sq > ranged_opt_range_sq {
                        let desired_pos = enemy.pos
                            + (pet_pos - enemy.pos).normalize_or_zero() * ranged_opt_range;
                        let direction = (desired_pos - pet_pos).normalize_or_zero();
                        direction * movement_speed.0
                    } else {
                        Vec2::ZERO
                    }
                } else {
                    let target_pos = formation_targets
                        .get(&pet_entity)
                        .copied()
                        .unwrap_or(player_pos);
                    let (follow_distance, stop_distance) = follow_distances(role, false);
                    arrive_velocity(
                        pet_pos,
                        target_pos,
                        movement_speed.0,
                        follow_distance,
                        stop_distance,
                    )
                }
            }
            PetAction::Fallback => {
                let fallback_offset = role_fallback_offset(role);
                let target_pos =
                    player_pos - player_dir * fallback_offset + player_right * side_hint * 30.0;
                let (follow_distance, stop_distance) = follow_distances(role, true);
                let mut velocity_out = arrive_velocity(
                    pet_pos,
                    target_pos,
                    movement_speed.0,
                    follow_distance,
                    stop_distance,
                );
                if let Some(enemy) = target_info {
                    let distance_sq = pet_pos.distance_squared(enemy.pos);
                    if distance_sq < ranged_min_range_sq {
                        let to_player = (player_pos - enemy.pos).normalize_or_zero();
                        let mut away_dir = (pet_pos - enemy.pos).normalize_or_zero();
                        if away_dir == Vec2::ZERO {
                            away_dir = Vec2::new(-to_player.y, to_player.x) * side_hint;
                        }
                        if away_dir.dot(to_player) > 0.5 {
                            away_dir = Vec2::new(-to_player.y, to_player.x) * side_hint;
                        }
                        velocity_out = away_dir * movement_speed.0;
                    }
                }
                velocity_out
            }
            PetAction::HoldPosition => Vec2::ZERO,
            PetAction::CollectXp => Vec2::ZERO,
        };

        if matches!(role, PetRole::Ranged | PetRole::Support) {
            let forward_offset = (pet_pos - player_pos).dot(player_dir);
            if forward_offset > 20.0 {
                target_velocity += -player_dir * movement_speed.0 * 0.35;
            }
        }

        let max_speed = movement_speed.0;
        let desired_dir = target_velocity.normalize_or_zero();
        let obstacle_avoidance = compute_obstacle_avoidance(
            pet_pos,
            desired_dir,
            pet_radius,
            &obstacles,
            side_hint,
            (max_speed * 0.4).max(45.0).min(120.0),
        );
        if separation != Vec2::ZERO {
            target_velocity += separation * max_speed * separation_strength;
        }
        let player_repulsion = compute_player_repulsion(pet_pos, player_pos, pet_radius);
        if player_repulsion != Vec2::ZERO {
            target_velocity += player_repulsion * max_speed * 0.8;
        }
        if obstacle_avoidance != Vec2::ZERO {
            target_velocity += obstacle_avoidance * max_speed * 0.9;
        }
        if target_velocity.length_squared() > max_speed * max_speed {
            target_velocity = target_velocity.normalize_or_zero() * max_speed;
        }

        let decay_rate = 10.0;
        velocity
            .0
            .smooth_nudge(&target_velocity, decay_rate, time.delta_secs());
    }
}

fn role_anchor_offset(role: PetRole) -> f32 {
    match role {
        PetRole::Melee => 70.0,
        PetRole::Ranged => -90.0,
        PetRole::Support => -120.0,
        PetRole::Collector => -110.0,
    }
}

fn role_fallback_offset(role: PetRole) -> f32 {
    match role {
        PetRole::Melee => 90.0,
        PetRole::Ranged => 140.0,
        PetRole::Support => 160.0,
        PetRole::Collector => 120.0,
    }
}

fn role_leash_distance(role: PetRole, detection_range: f32) -> f32 {
    let base = detection_range.max(200.0);
    match role {
        PetRole::Melee => base + 80.0,
        PetRole::Ranged => base + 120.0,
        PetRole::Support => base + 140.0,
        PetRole::Collector => base + 200.0,
    }
}

fn follow_distances(role: PetRole, regrouping: bool) -> (f32, f32) {
    let (follow_distance, stop_distance) = match role {
        PetRole::Melee => (90.0, 30.0),
        PetRole::Ranged => (110.0, 35.0),
        PetRole::Support => (120.0, 40.0),
        PetRole::Collector => (100.0, 30.0),
    };
    if regrouping {
        (follow_distance * 1.15, stop_distance)
    } else {
        (follow_distance, stop_distance)
    }
}

fn action_lock_duration(role: PetRole, action: PetAction) -> f32 {
    let base = role.action_hold_time();
    match action {
        PetAction::Regroup => base * 0.7,
        PetAction::Fallback => base * 0.6,
        PetAction::FollowFormation => base * 0.55,
        PetAction::HoldPosition => base * 0.8,
        PetAction::CollectXp => base,
        _ => base,
    }
}

fn enemy_threat_weight(enemy_type: EnemyType) -> f32 {
    match enemy_type {
        EnemyType::Bandit => 1.0,
        EnemyType::Thief => 1.4,
        EnemyType::Brute => 1.2,
    }
}

fn pick_enemy_for_pet(
    role: PetRole,
    pet_pos: Vec2,
    detection_range: f32,
    leash_distance: f32,
    player_focus: Option<Entity>,
    enemies: &[EnemyInfo],
    target_counts: &mut HashMap<Entity, u32>,
    current_target: Option<Entity>,
    assist_bias: bool,
) -> Option<EnemyInfo> {
    let mut best: Option<(EnemyInfo, f32)> = None;
    let range_scale = detection_range.max(1.0);
    let detection_range_sq = detection_range * detection_range;
    let target_penalty = range_scale * 0.35;
    let focus_bonus = range_scale * 0.25;
    let stickiness = range_scale * 0.2;
    let peel_bonus = range_scale * 0.2;
    let role_bias = match role {
        PetRole::Melee => 0.18,
        PetRole::Ranged | PetRole::Support => 0.08,
        PetRole::Collector => 0.0,
    };

    for enemy in enemies.iter() {
        let distance_sq = pet_pos.distance_squared(enemy.pos);
        if distance_sq > detection_range_sq {
            continue;
        }
        if enemy.distance_to_player > leash_distance {
            continue;
        }

        let distance_to_pet = distance_sq.sqrt();

        let assigned = *target_counts.get(&enemy.entity).unwrap_or(&0) as f32;
        if assigned >= enemy.max_pets as f32 {
            continue;
        }

        let mut score = distance_to_pet + (assigned * target_penalty);
        if Some(enemy.entity) == current_target {
            score -= stickiness;
        }
        if Some(enemy.entity) == player_focus {
            score -= focus_bonus;
        }
        let threat = enemy_threat_weight(enemy.enemy_type);
        score -= threat * range_scale * 0.18;
        let proximity = (1.0 - (enemy.distance_to_player / range_scale)).clamp(0.0, 1.0);
        score -= proximity * range_scale * role_bias;
        if assist_bias {
            score -= proximity * range_scale * 0.3;
        }
        if enemy.distance_to_player < 120.0 {
            score -= peel_bonus;
        }

        if let Some((_, best_score)) = best {
            if score < best_score {
                best = Some((*enemy, score));
            }
        } else {
            best = Some((*enemy, score));
        }
    }

    if let Some((enemy, _)) = best {
        if Some(enemy.entity) != current_target {
            let entry = target_counts.entry(enemy.entity).or_insert(0);
            *entry += 1;
        }
        return Some(enemy);
    }

    None
}

fn arrive_velocity(
    current_pos: Vec2,
    target_pos: Vec2,
    movement_speed: f32,
    follow_distance: f32,
    stop_distance: f32,
) -> Vec2 {
    let distance_to_target = current_pos.distance(target_pos);
    if distance_to_target > follow_distance {
        let direction = (target_pos - current_pos).normalize_or_zero();
        direction * movement_speed
    } else if distance_to_target > stop_distance {
        let direction = (target_pos - current_pos).normalize_or_zero();
        let speed_factor = (distance_to_target - stop_distance) / (follow_distance - stop_distance);
        direction * movement_speed * speed_factor
    } else {
        Vec2::ZERO
    }
}

fn compute_player_repulsion(pet_pos: Vec2, player_pos: Vec2, pet_radius: f32) -> Vec2 {
    let avoid_radius = (pet_radius + 30.0).max(40.0);
    let delta = pet_pos - player_pos;
    let dist = delta.length();
    if dist > 0.0 && dist < avoid_radius {
        let strength = (avoid_radius - dist) / avoid_radius;
        (delta / dist) * strength
    } else {
        Vec2::ZERO
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
    enemy_query: Query<(Entity, &Transform, &Team, &Hitbox), With<Enemy>>,
    upgrade_state: Res<UpgradeState>,
    pet_sprites: Res<PetSpriteSheet>,
) {
    let mut target_counts: HashMap<Entity, u32> =
        HashMap::with_capacity(enemy_query.iter().size_hint().0);

    for (pet_transform, pet, damage, attack_range, mut attack_timer, pet_team, attack_target) in
        pet_query.iter_mut()
    {
        // Проверяем, готов ли питомец к атаке
        if !attack_timer.timer.is_finished() {
            continue;
        }

        if matches!(
            pet.pet_type,
            PetType::GuardDog | PetType::SlimeCompanion | PetType::XpCollector
        ) {
            continue;
        }

        let mut chosen_target: Option<(Entity, Vec2)> = None;
        let attack_range_sq = attack_range.0 * attack_range.0;

        if let Some(target) = attack_target {
            if let Ok((enemy_entity, enemy_transform, enemy_team, hitbox)) =
                enemy_query.get(target.target_entity)
            {
                if pet_team.0 != enemy_team.0 {
                    let distance_sq = pet_transform
                        .translation
                        .distance_squared(enemy_transform.translation);
                    let assigned = *target_counts.get(&enemy_entity).unwrap_or(&0) as f32;
                    let max_pets = max_pets_for_enemy(hitbox) as f32;
                    if distance_sq <= attack_range_sq && assigned < max_pets {
                        chosen_target =
                            Some((enemy_entity, enemy_transform.translation.truncate()));
                    }
                }
            }
        }

        if chosen_target.is_none() {
            // Ищем врага в радиусе атаки, распределяя цели между питомцами
            let target_penalty = attack_range.0 * 0.75;
            let mut best_enemy: Option<(Entity, Vec2)> = None;
            let mut best_score = f32::INFINITY;

            for (enemy_entity, enemy_transform, enemy_team, hitbox) in enemy_query.iter() {
                if pet_team.0 == enemy_team.0 {
                    continue;
                }

                let distance_sq = pet_transform
                    .translation
                    .distance_squared(enemy_transform.translation);

                if distance_sq <= attack_range_sq {
                    let distance = distance_sq.sqrt();
                    let enemy_pos = enemy_transform.translation.truncate();
                    let assigned = *target_counts.get(&enemy_entity).unwrap_or(&0) as f32;
                    let max_pets = max_pets_for_enemy(hitbox) as f32;
                    if assigned >= max_pets {
                        continue;
                    }
                    let score = distance + (assigned * target_penalty);

                    if score < best_score {
                        best_score = score;
                        best_enemy = Some((enemy_entity, enemy_pos));
                    }
                }
            }

            if let Some((enemy_entity, enemy_pos)) = best_enemy {
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
                        area_radius: 0.0,
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

fn hex_spiral_offset(index: usize, spacing: f32) -> Vec2 {
    if index == 0 {
        return Vec2::ZERO;
    }

    let mut ring = 1usize;
    let mut count = 1usize;
    while count + (6 * ring) <= index {
        count += 6 * ring;
        ring += 1;
    }

    let mut steps_left = index - count;
    let mut x = ring as i32;
    let mut _y = -(ring as i32);
    let mut z = 0i32;
    let directions = [
        (0, 1, -1),
        (-1, 1, 0),
        (-1, 0, 1),
        (0, -1, 1),
        (1, -1, 0),
        (1, 0, -1),
    ];

    for (dx, dy, dz) in directions {
        if steps_left == 0 {
            break;
        }
        let steps = ring.min(steps_left);
        x += dx * steps as i32;
        _y += dy * steps as i32;
        z += dz * steps as i32;
        steps_left -= steps;
    }

    let q = x as f32;
    let r = z as f32;
    let sqrt3 = 3.0_f32.sqrt();
    Vec2::new(spacing * sqrt3 * (q + r * 0.5), spacing * 1.5 * r)
}

fn compute_obstacle_avoidance(
    pet_pos: Vec2,
    desired_dir: Vec2,
    pet_radius: f32,
    obstacles: &[(Vec2, Vec2)],
    side_hint: f32,
    look_ahead: f32,
) -> Vec2 {
    if desired_dir == Vec2::ZERO {
        return Vec2::ZERO;
    }

    let mut avoidance = Vec2::ZERO;
    for (obstacle_pos, half_size) in obstacles.iter() {
        let to_obstacle = *obstacle_pos - pet_pos;
        let forward_dist = to_obstacle.dot(desired_dir);
        if forward_dist < 0.0 || forward_dist > look_ahead {
            continue;
        }
        let lateral = to_obstacle - desired_dir * forward_dist;
        let lateral_dist = lateral.length();
        let obstacle_radius = half_size.x.max(half_size.y);
        let clearance = obstacle_radius + pet_radius + 8.0;
        if lateral_dist < clearance {
            let strength = (clearance - lateral_dist) / clearance;
            let away = if lateral_dist > 0.001 {
                -lateral / lateral_dist
            } else {
                let perp = Vec2::new(-desired_dir.y, desired_dir.x);
                perp * side_hint
            };
            avoidance += away * strength;
        }
    }

    if avoidance.length_squared() > 1.0 {
        avoidance = avoidance.normalize_or_zero();
    }
    avoidance
}

fn rotate_vec2(vec: Vec2, angle: f32) -> Vec2 {
    let cos = angle.cos();
    let sin = angle.sin();
    Vec2::new(vec.x * cos - vec.y * sin, vec.x * sin + vec.y * cos)
}

pub fn pet_engage_range(attack_range: f32, pet_radius: f32, enemy_radius: f32) -> f32 {
    let close_range = (pet_radius + enemy_radius + 8.0).max(24.0);
    attack_range.max(close_range)
}

fn max_pets_for_enemy(hitbox: &Hitbox) -> u32 {
    let full_size = hitbox.half_size.x * 2.0;
    let small_hitbox = enemy_sizes::THIEF * ENEMY_HITBOX_SCALE;
    let ratio = (full_size / small_hitbox).max(1.0);
    let max_pets = (3.0 * ratio).round() as u32;
    max_pets.max(3)
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
            let distance_sq = projectile_transform
                .translation
                .distance_squared(enemy_transform.translation);
            let collision_radius = 20.0;
            let collision_radius_sq = collision_radius * collision_radius;

            if distance_sq <= collision_radius_sq {
                // Наносим урон
                damage_events.write(crate::systems::combat::DamageEvent {
                    target: enemy_entity,
                    damage: projectile.damage,
                });

                let impact_pos = enemy_transform.translation.truncate();
                spawn_hit_particles(&mut commands, impact_pos, Color::srgb(1.0, 0.8, 0.4), 6);

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
