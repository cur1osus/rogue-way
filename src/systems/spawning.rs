use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackRange, AttackTimer, Boss, BossType, CollisionLayer,
    Damage, Enemy, EnemyType, Health, Hitbox, MovementSpeed, Player, Target, Team, Velocity,
};
use crate::constants::{ENEMY_HITBOX_SCALE, MAX_ACTIVE_ENEMIES};
use crate::resources::{EnemySpriteSheet, WaveConfig};
use rand::Rng;

/// Система спавна врагов волнами
pub fn spawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut wave_config: ResMut<WaveConfig>,
    player_query: Query<(Entity, &Transform), With<Player>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    enemy_query: Query<Entity, With<Enemy>>,
    enemy_sprites: Res<EnemySpriteSheet>,
) {
    // Обновляем игровое время
    wave_config.game_time += time.delta_secs();
    wave_config.update_difficulty();

    // Получаем игрока и его позицию
    let Ok((player_entity, player_transform)) = player_query.single() else {
        return;
    };

    // Получаем размер окна для спавна за пределами экрана
    let Ok(window) = windows.single() else {
        return;
    };
    let window_width = window.width();
    let window_height = window.height();

    // Проверяем спавн боссов на временных метках
    check_boss_spawn(
        &mut commands,
        &mut wave_config,
        player_entity,
        player_transform.translation.truncate(),
        window_width,
        window_height,
        &enemy_sprites,
    );

    // Тикаем таймер спавна
    wave_config.spawn_timer.tick(time.delta());

    if !wave_config.spawn_timer.just_finished() {
        return;
    }

    let current_enemies = enemy_query.iter().count();
    let available_slots = MAX_ACTIVE_ENEMIES.saturating_sub(current_enemies);
    if available_slots == 0 {
        return;
    }

    let spawn_count = wave_config.enemies_per_spawn.min(available_slots as u32);

    // Спавним врагов
    for _ in 0..spawn_count {
        let spawn_pos = get_spawn_position_outside_screen(
            player_transform.translation.truncate(),
            window_width,
            window_height,
        );

        // Выбираем тип врага на основе времени игры
        let enemy_type = choose_enemy_type(wave_config.game_time);
        spawn_enemy(
            &mut commands,
            enemy_type,
            spawn_pos,
            player_entity,
            wave_config.difficulty_multiplier,
            &enemy_sprites,
        );
    }
}

/// Проверка и спавн боссов на временных метках
fn check_boss_spawn(
    commands: &mut Commands,
    wave_config: &mut WaveConfig,
    player_entity: Entity,
    player_pos: Vec2,
    window_width: f32,
    window_height: f32,
    enemy_sprites: &EnemySpriteSheet,
) {
    // Босс на 5 минутах
    if wave_config.game_time >= 300.0 && !wave_config.boss_5min_spawned {
        wave_config.boss_5min_spawned = true;
        let spawn_pos = get_spawn_position_outside_screen(player_pos, window_width, window_height);
        spawn_boss(
            commands,
            BossType::BanditLeader,
            spawn_pos,
            player_entity,
            enemy_sprites,
        );
    }

    // Босс на 10 минутах
    if wave_config.game_time >= 600.0 && !wave_config.boss_10min_spawned {
        wave_config.boss_10min_spawned = true;
        let spawn_pos = get_spawn_position_outside_screen(player_pos, window_width, window_height);
        spawn_boss(
            commands,
            BossType::ThiefKing,
            spawn_pos,
            player_entity,
            enemy_sprites,
        );
    }

    // Босс на 15 минутах
    if wave_config.game_time >= 900.0 && !wave_config.boss_15min_spawned {
        wave_config.boss_15min_spawned = true;
        let spawn_pos = get_spawn_position_outside_screen(player_pos, window_width, window_height);
        spawn_boss(
            commands,
            BossType::BruteChieftain,
            spawn_pos,
            player_entity,
            enemy_sprites,
        );
    }
}

/// Создает босса
fn spawn_boss(
    commands: &mut Commands,
    boss_type: BossType,
    spawn_pos: Vec2,
    player_entity: Entity,
    enemy_sprites: &EnemySpriteSheet,
) {
    let (hp, speed, damage, _xp_reward) = boss_type.get_stats();
    let attack_range = boss_type.get_attack_range();
    let attack_speed = boss_type.get_attack_speed();
    let size = boss_type.get_size();
    let scale = size / enemy_sprites.run.frame_size.y;
    let hitbox_size = Vec2::splat(size * ENEMY_HITBOX_SCALE);

    commands
        .spawn((
            Sprite {
                image: enemy_sprites.run.texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: enemy_sprites.run.layout.clone(),
                    index: enemy_sprites.run.first,
                }),
                ..default()
            },
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 1.0).with_scale(Vec3::splat(scale)),
        ))
        .insert((
            Boss { boss_type },
            Enemy {
                enemy_type: EnemyType::Bandit,
            }, // Базовое поведение
            Hitbox::from_full_size(hitbox_size),
            CollisionLayer::enemy(),
            Health::new(hp),
            MovementSpeed(speed),
            Velocity::default(),
            Damage(damage),
            AttackRange(attack_range),
            AttackTimer::from_attack_speed(attack_speed),
            Team(Team::ENEMY),
            Target(player_entity),
            AnimationIndices {
                first: enemy_sprites.run.first,
                last: enemy_sprites.run.last,
            },
            AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
        ));
}

/// Выбирает тип врага на основе времени игры
fn choose_enemy_type(game_time: f32) -> EnemyType {
    let mut rng = rand::thread_rng();

    // В первые 60 секунд - только бандиты
    if game_time < 60.0 {
        return EnemyType::Bandit;
    }

    // После 60 секунд добавляются воры
    if game_time < 120.0 {
        let roll: f32 = rng.gen();
        if roll < 0.3 {
            EnemyType::Thief
        } else {
            EnemyType::Bandit
        }
    }
    // После 120 секунд добавляются громилы
    else if game_time < 240.0 {
        let roll: f32 = rng.gen();
        if roll < 0.2 {
            EnemyType::Brute
        } else if roll < 0.5 {
            EnemyType::Thief
        } else {
            EnemyType::Bandit
        }
    }
    // После 240 секунд - полная смесь с акцентом на сложных врагов
    else {
        let roll: f32 = rng.gen();
        if roll < 0.3 {
            EnemyType::Brute
        } else if roll < 0.6 {
            EnemyType::Thief
        } else {
            EnemyType::Bandit
        }
    }
}

/// Создает врага определенного типа
fn spawn_enemy(
    commands: &mut Commands,
    enemy_type: EnemyType,
    spawn_pos: Vec2,
    player_entity: Entity,
    difficulty_multiplier: f32,
    enemy_sprites: &EnemySpriteSheet,
) {
    let (base_hp, speed, base_damage, _xp_reward) = enemy_type.get_stats();
    let attack_range = enemy_type.get_attack_range();
    let attack_speed = enemy_type.get_attack_speed();
    let size = enemy_type.get_size();
    let scale = size / enemy_sprites.run.frame_size.y;
    let hitbox_size = Vec2::splat(size * ENEMY_HITBOX_SCALE);

    commands
        .spawn((
            Sprite {
                image: enemy_sprites.run.texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: enemy_sprites.run.layout.clone(),
                    index: enemy_sprites.run.first,
                }),
                ..default()
            },
            Transform::from_xyz(spawn_pos.x, spawn_pos.y, 1.0).with_scale(Vec3::splat(scale)),
        ))
        .insert((
            Enemy { enemy_type },
            Hitbox::from_full_size(hitbox_size),
            CollisionLayer::enemy(),
            Health::new(base_hp * difficulty_multiplier),
            MovementSpeed(speed),
            Velocity::default(),
            Damage(base_damage * difficulty_multiplier),
            AttackRange(attack_range),
            AttackTimer::from_attack_speed(attack_speed),
            Team(Team::ENEMY),
            Target(player_entity),
            AnimationIndices {
                first: enemy_sprites.run.first,
                last: enemy_sprites.run.last,
            },
            AnimationTimer(Timer::from_seconds(0.12, TimerMode::Repeating)),
        ));
}

/// Вычисляет позицию спавна за пределами экрана
fn get_spawn_position_outside_screen(
    player_pos: Vec2,
    window_width: f32,
    window_height: f32,
) -> Vec2 {
    let mut rng = rand::thread_rng();

    // Отступ от края экрана
    let margin = 100.0;
    let half_width = window_width / 2.0 + margin;
    let half_height = window_height / 2.0 + margin;

    // Выбираем случайную сторону экрана (0=верх, 1=право, 2=низ, 3=лево)
    let side = rng.gen_range(0..4);

    match side {
        0 => {
            // Верх
            Vec2::new(
                player_pos.x + rng.gen_range(-half_width..half_width),
                player_pos.y + half_height,
            )
        }
        1 => {
            // Право
            Vec2::new(
                player_pos.x + half_width,
                player_pos.y + rng.gen_range(-half_height..half_height),
            )
        }
        2 => {
            // Низ
            Vec2::new(
                player_pos.x + rng.gen_range(-half_width..half_width),
                player_pos.y - half_height,
            )
        }
        _ => {
            // Лево
            Vec2::new(
                player_pos.x - half_width,
                player_pos.y + rng.gen_range(-half_height..half_height),
            )
        }
    }
}
