use crate::components::{
    AttackRange, DeathAnimation, Enemy, EnemyAIConfig, EnemyAction, EnemyBlackboard,
    EnemyPerception, EnemyState, Health, MovementSpeed, Player, SlowEffect, Target, Velocity,
};
use bevy::prelude::*;
use rand::Rng;
use std::cmp::Ordering;

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

fn clamp01(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn score_distance_far(distance: f32, preferred: f32, max: f32) -> f32 {
    if distance <= preferred {
        0.0
    } else {
        let span = (max - preferred).max(1.0);
        clamp01((distance - preferred) / span)
    }
}

fn score_distance_close(distance: f32, preferred: f32) -> f32 {
    if distance >= preferred {
        0.0
    } else {
        clamp01((preferred - distance) / preferred.max(1.0))
    }
}

fn choose_weighted_action(rng: &mut impl Rng, candidates: &[(EnemyAction, f32)]) -> EnemyAction {
    let total: f32 = candidates.iter().map(|(_, score)| *score).sum();
    if total <= 0.0 {
        return candidates[0].0;
    }

    let mut roll = rng.gen::<f32>() * total;
    for (action, score) in candidates.iter() {
        if roll <= *score {
            return *action;
        }
        roll -= *score;
    }

    candidates[0].0
}

fn pick_combat_action(
    rng: &mut impl Rng,
    distance: f32,
    attack_range: f32,
    vision_range: f32,
    config: &EnemyAIConfig,
    memory: &EnemyBlackboard,
) -> (EnemyAction, f32) {
    let preferred = config
        .preferred_distance
        .clamp(attack_range * 0.85, attack_range * 1.35);
    let approach_score =
        config.approach_bias * score_distance_far(distance, preferred, vision_range);
    let keep_score = if distance < attack_range * 0.9 {
        config.keep_distance_bias * score_distance_close(distance, preferred)
    } else {
        0.0
    };
    let basic_score = if distance <= attack_range { 0.9 } else { 0.0 };
    let heavy_score = if config.heavy_attack_weight > 0.0
        && memory.heavy_cooldown.is_finished()
        && distance <= attack_range * config.heavy_attack_range_mult
    {
        config.heavy_attack_weight
    } else {
        0.0
    };

    let mut candidates = vec![
        (EnemyAction::BasicAttack, basic_score),
        (EnemyAction::HeavyAttack, heavy_score),
        (EnemyAction::ApproachTarget, approach_score),
        (EnemyAction::KeepDistance, keep_score),
    ];
    candidates.retain(|(_, score)| *score > 0.01);

    if candidates.is_empty() {
        return (EnemyAction::ApproachTarget, 0.1);
    }

    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
    let best_score = candidates[0].1;
    let current_score = candidates
        .iter()
        .find(|(action, _)| *action == memory.current_action)
        .map(|(_, score)| *score)
        .unwrap_or(0.0);

    let switch_threshold = 1.15;
    if current_score > 0.0 {
        if !memory.action_lock.is_finished() {
            return (memory.current_action, current_score);
        }
        if best_score < current_score * switch_threshold {
            return (memory.current_action, current_score);
        }
    }

    let top_n = 2.min(candidates.len());
    let chosen = choose_weighted_action(rng, &candidates[..top_n]);
    let chosen_score = candidates
        .iter()
        .find(|(action, _)| *action == chosen)
        .map(|(_, score)| *score)
        .unwrap_or(best_score);

    (chosen, chosen_score)
}

/// Система AI врагов (FSM + Utility AI)
pub fn enemy_ai_system(
    time: Res<Time>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    mut enemy_sets: ParamSet<(
        Query<
            (Entity, &Transform, &EnemyPerception),
            (With<Enemy>, Without<Player>, Without<DeathAnimation>),
        >,
        Query<
            (
                Entity,
                &Transform,
                &mut Velocity,
                &MovementSpeed,
                &Target,
                Option<&SlowEffect>,
                &AttackRange,
                &Health,
                &EnemyPerception,
                &EnemyAIConfig,
                &mut EnemyState,
                &mut EnemyBlackboard,
            ),
            (With<Enemy>, Without<Player>, Without<DeathAnimation>),
        >,
    )>,
) {
    let Ok((_player_entity, player_transform)) = player_query.single() else {
        return;
    };

    let player_pos = player_transform.translation.truncate();
    let now = time.elapsed_secs();
    let dt = time.delta();

    let query = enemy_sets.p0();
    let mut alert_sources: Vec<Vec2> = Vec::with_capacity(query.iter().size_hint().0);
    for (_entity, transform, perception) in query.iter() {
        let vision_range = perception.vision_range;
        if vision_range <= 0.0 {
            continue;
        }
        let distance_sq = (transform.translation.truncate() - player_pos).length_squared();
        if distance_sq <= vision_range * vision_range {
            alert_sources.push(transform.translation.truncate());
        }
    }

    let mut rng = rand::thread_rng();
    let mut enemy_query = enemy_sets.p1();
    for (
        entity,
        transform,
        mut velocity,
        speed,
        _target,
        slow_opt,
        attack_range,
        health,
        perception,
        config,
        mut state,
        mut memory,
    ) in enemy_query.iter_mut()
    {
        memory.decision_timer.tick(dt);
        memory.action_lock.tick(dt);
        memory.heavy_cooldown.tick(dt);
        if *state == EnemyState::Alert {
            memory.alert_timer.tick(dt);
        }
        if *state == EnemyState::Search {
            memory.search_timer.tick(dt);
        }

        let enemy_pos = transform.translation.truncate();
        let distance = enemy_pos.distance(player_pos);

        let player_visible = perception.vision_range > 0.0 && distance <= perception.vision_range;
        let alert_range_sq = perception.alert_range * perception.alert_range;
        let alerted_by_ally = !player_visible
            && alert_sources
                .iter()
                .any(|pos| pos.distance_squared(enemy_pos) <= alert_range_sq);

        if player_visible || alerted_by_ally {
            memory.last_seen_pos = Some(player_pos);
            memory.last_seen_time = now;
        }

        let seen_recently = if memory.last_seen_pos.is_some() {
            now - memory.last_seen_time <= perception.lose_sight_after
        } else {
            false
        };

        let hp_ratio = if health.max > 0.0 {
            (health.current / health.max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let flee_recover = (config.flee_hp_ratio + 0.15).min(0.9);

        let mut new_state = *state;
        if *state == EnemyState::Flee {
            if hp_ratio > flee_recover {
                new_state = if player_visible {
                    EnemyState::Combat
                } else if seen_recently {
                    EnemyState::Search
                } else {
                    EnemyState::Idle
                };
            }
        } else if hp_ratio <= config.flee_hp_ratio {
            new_state = EnemyState::Flee;
        } else if player_visible {
            new_state = match *state {
                EnemyState::Combat => EnemyState::Combat,
                EnemyState::Alert => {
                    if memory.alert_timer.is_finished() {
                        EnemyState::Combat
                    } else {
                        EnemyState::Alert
                    }
                }
                _ => EnemyState::Alert,
            };
        } else if alerted_by_ally {
            new_state = match *state {
                EnemyState::Alert => {
                    if memory.alert_timer.is_finished() {
                        EnemyState::Search
                    } else {
                        EnemyState::Alert
                    }
                }
                _ => EnemyState::Alert,
            };
        } else if seen_recently {
            new_state = match *state {
                EnemyState::Search => {
                    if memory.search_timer.is_finished() {
                        EnemyState::Idle
                    } else {
                        EnemyState::Search
                    }
                }
                _ => EnemyState::Search,
            };
        } else {
            new_state = EnemyState::Idle;
        }

        let state_changed = new_state != *state;
        if state_changed {
            *state = new_state;
            match new_state {
                EnemyState::Alert => memory.alert_timer.reset(),
                EnemyState::Search => memory.search_timer.reset(),
                _ => {}
            }
        }

        let should_decide = state_changed || memory.decision_timer.just_finished();

        if *state != EnemyState::Combat {
            let (desired_action, desired_score) = match *state {
                EnemyState::Idle => (EnemyAction::Idle, 0.0),
                EnemyState::Alert => (EnemyAction::ApproachTarget, 0.4),
                EnemyState::Search => (EnemyAction::SearchLastSeen, 0.4),
                EnemyState::Flee => (EnemyAction::Retreat, 1.0),
                EnemyState::Combat => (EnemyAction::Idle, 0.0),
            };
            if memory.current_action != desired_action {
                memory.current_action = desired_action;
                memory.current_score = desired_score;
                memory.action_lock.reset();
            } else {
                memory.current_score = desired_score;
            }
        } else if should_decide {
            let (chosen_action, chosen_score) = pick_combat_action(
                &mut rng,
                distance,
                attack_range.0,
                perception.vision_range,
                config,
                &memory,
            );
            if chosen_action != memory.current_action {
                memory.current_action = chosen_action;
                memory.current_score = chosen_score;
                memory.action_lock.reset();
            } else {
                memory.current_score = chosen_score;
            }
        }

        let mut direction = Vec2::ZERO;
        let mut action_speed_scale = 1.0;

        match memory.current_action {
            EnemyAction::ApproachTarget => {
                direction = (player_pos - enemy_pos).normalize_or_zero();
            }
            EnemyAction::KeepDistance | EnemyAction::Retreat => {
                direction = (enemy_pos - player_pos).normalize_or_zero();
                action_speed_scale = if memory.current_action == EnemyAction::Retreat {
                    1.15
                } else {
                    0.9
                };
            }
            EnemyAction::SearchLastSeen => {
                if let Some(last_seen) = memory.last_seen_pos {
                    let to_last = last_seen - enemy_pos;
                    if to_last.length() > 15.0 {
                        direction = to_last.normalize_or_zero();
                    }
                    action_speed_scale = 0.85;
                }
            }
            EnemyAction::BasicAttack | EnemyAction::HeavyAttack => {
                let effective_range = if memory.current_action == EnemyAction::HeavyAttack {
                    attack_range.0 * config.heavy_attack_range_mult
                } else {
                    attack_range.0
                };
                if distance > effective_range * 0.9 {
                    direction = (player_pos - enemy_pos).normalize_or_zero();
                    action_speed_scale = 0.6;
                } else {
                    direction = Vec2::ZERO;
                    action_speed_scale = 0.0;
                }
            }
            EnemyAction::Idle => {
                direction = Vec2::ZERO;
                action_speed_scale = 0.0;
            }
        }

        if matches!(
            memory.current_action,
            EnemyAction::ApproachTarget | EnemyAction::KeepDistance
        ) && direction != Vec2::ZERO
        {
            let side = if entity.index().index() % 2 == 0 {
                1.0
            } else {
                -1.0
            };
            let flank = Vec2::new(-direction.y, direction.x) * config.flank_bias * side;
            direction = (direction + flank).normalize_or_zero();
        }

        let state_speed_scale = match *state {
            EnemyState::Alert => 0.6,
            EnemyState::Search => 0.85,
            EnemyState::Flee => 1.2,
            _ => 1.0,
        };

        let slow_multiplier = slow_opt.map(|slow| slow.slow_amount).unwrap_or(1.0);
        let final_speed = speed.0 * slow_multiplier * action_speed_scale * state_speed_scale;
        if final_speed > 0.0 && direction != Vec2::ZERO {
            velocity.0 = direction * final_speed;
        } else {
            velocity.0 = Vec2::ZERO;
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
