use bevy::prelude::*;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackRange, AttackSpeed, AttackTimer, CollisionLayer,
    Damage, DetectionRange, Experience, Gold, Health, Hitbox, MovementSpeed, Pet, PetMovementSpeed,
    PetType, Player, PlayerAnimation, Team, Velocity,
};
use crate::constants::{PET_HITBOX_SCALE, PLAYER_HITBOX_SCALE, PLAYER_SCALE};
use crate::resources::{MetaProgression, UpgradeState};

const PLAYER_SPRITE_COLUMNS: u32 = 8;
const PLAYER_SPRITE_ROWS: u32 = 2;
const PLAYER_FRAME_SIZE: Vec2 = Vec2::new(108.0, 140.0);
const PET_ANIMATION_SECONDS: f32 = 0.12;

/// Система настройки игрока (запускается один раз при старте)
pub fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    meta: Res<MetaProgression>,
    existing_player: Query<Entity, With<Player>>,
) {
    // Если игрок уже существует, не создаем нового (возврат из LevelUpChoice)
    if !existing_player.is_empty() {
        return;
    }

    let texture = asset_server.load("sprites/sprite.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(PLAYER_FRAME_SIZE.x as u32, PLAYER_FRAME_SIZE.y as u32),
        PLAYER_SPRITE_COLUMNS,
        PLAYER_SPRITE_ROWS,
        None,
        None,
    );
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let right_start = 0;
    let left_start = (PLAYER_SPRITE_COLUMNS * (PLAYER_SPRITE_ROWS - 1)) as usize;

    // Применяем постоянные улучшения из метапрогрессии (§3.2.2, §3.2.3)
    let upgrades = &meta.save_data.permanent_upgrades;
    let base_hp = upgrades.get_max_hp();
    let base_speed = 100.0 * upgrades.get_movement_speed_multiplier();
    let starting_level = upgrades.get_starting_level();
    let starting_gold = upgrades.get_starting_gold();

    // Создаем игрока с улучшенными статами
    commands.spawn((
        Sprite {
            image: texture,
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas_layout,
                index: right_start,
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0).with_scale(Vec3::splat(PLAYER_SCALE)),
        Player,
        Hitbox::from_full_size(PLAYER_FRAME_SIZE * PLAYER_SCALE * PLAYER_HITBOX_SCALE),
        CollisionLayer::player(),
        Health::new(base_hp),      // HP с учетом улучшений
        MovementSpeed(base_speed), // Скорость с учетом улучшений
        Velocity::default(),
        Team(Team::PLAYER),
        Experience::new_with_level(starting_level), // Стартовый уровень из улучшений
        Gold::new(starting_gold),                   // Стартовое золото из улучшений
        AnimationIndices {
            first: right_start,
            last: right_start + (PLAYER_SPRITE_COLUMNS as usize) - 1,
        },
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        PlayerAnimation {
            right_start,
            left_start,
            frames: PLAYER_SPRITE_COLUMNS as usize,
            facing: 1,
        },
    ));
}

/// Система настройки камеры
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        IsDefaultUiCamera,
        Transform::from_xyz(0.0, 0.0, 1000.0),
    ));

    // Создаём визуальную сетку для ориентации в пространстве
    let grid_spacing = 100.0;
    let grid_size = 10; // 10x10 сетка

    for x in -grid_size..=grid_size {
        for y in -grid_size..=grid_size {
            let pos_x = x as f32 * grid_spacing;
            let pos_y = y as f32 * grid_spacing;

            // Делаем центральную точку ярче
            let brightness = if x == 0 && y == 0 { 0.8 } else { 0.2 };

            commands.spawn((
                Sprite::from_color(
                    Color::srgb(brightness, brightness, brightness),
                    Vec2::new(4.0, 4.0),
                ),
                Transform::from_xyz(pos_x, pos_y, 0.1),
            ));
        }
    }
}

/// Система следования камеры за игроком
pub fn camera_follow_system(
    player_query: Query<&Transform, With<Player>>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    shake: Res<crate::resources::ScreenShake>,
) {
    if let Ok(player_transform) = player_query.single() {
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            camera_transform.translation.x = player_transform.translation.x + shake.offset.x;
            camera_transform.translation.y = player_transform.translation.y + shake.offset.y;
        }
    }
}

/// Анимация игрока на основе скорости.
pub fn player_animation_system(
    time: Res<Time>,
    mut query: Query<
        (
            &Velocity,
            &mut Sprite,
            &mut AnimationTimer,
            &mut AnimationIndices,
            &mut PlayerAnimation,
        ),
        With<Player>,
    >,
) {
    for (velocity, mut sprite, mut timer, mut indices, mut animation) in query.iter_mut() {
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };
        let moving = velocity.0.length_squared() > 0.01;

        if velocity.0.x < -0.05 {
            animation.facing = -1;
        } else if velocity.0.x > 0.05 {
            animation.facing = 1;
        }

        let (first, last, idle) = if animation.facing >= 0 {
            let start = animation.right_start;
            (start, start + animation.frames - 1, start)
        } else {
            let start = animation.left_start;
            (start, start + animation.frames - 1, start)
        };

        indices.first = first;
        indices.last = last;

        if !moving {
            atlas.index = idle;
            continue;
        }

        timer.tick(time.delta());

        if atlas.index < indices.first || atlas.index > indices.last {
            atlas.index = indices.first;
        }

        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}

/// Система настройки питомцев (запускается один раз при старте)
pub fn setup_pets(
    mut commands: Commands,
    upgrade_state: Res<UpgradeState>,
    pet_sprites: Res<crate::resources::PetSpriteSheet>,
    existing_pets: Query<Entity, With<Pet>>,
) {
    // Если есть питомцы, значит это не первый запуск (возврат из LevelUpChoice)
    // В этом случае сохраняем существующих питомцев
    let pet_count = existing_pets.iter().count();
    if pet_count > 0 {
        return;
    }

    // Только если это первый запуск (нет питомцев), создаем стартового
    spawn_pet(
        &mut commands,
        PetType::GuardDog,
        Vec2::ZERO,
        &upgrade_state,
        &pet_sprites,
    );
}

/// Функция для спавна питомца определенного типа
pub fn spawn_pet(
    commands: &mut Commands,
    pet_type: PetType,
    position: Vec2,
    upgrades: &UpgradeState,
    pet_sprites: &crate::resources::PetSpriteSheet,
) {
    let (
        base_damage,
        base_attack_speed,
        base_attack_range,
        base_detection_range,
        base_movement_speed,
    ) = pet_type.get_stats();
    let damage = base_damage * upgrades.pet_damage_mult;
    let attack_speed = base_attack_speed * upgrades.pet_attack_speed_mult;
    let attack_range = base_attack_range + upgrades.pet_range_bonus;
    let detection_range = base_detection_range + upgrades.pet_detection_range_bonus;
    let movement_speed = base_movement_speed;

    // Получаем спрайт для питомца
    let sprite_sheet = pet_sprites.get_idle_sheet(&pet_type);
    let size = pet_type.get_size();
    let pet_scale = size / sprite_sheet.frame_size.y;
    let hitbox_size = Vec2::splat(size * PET_HITBOX_SCALE);

    commands.spawn((
        Sprite {
            image: sprite_sheet.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: sprite_sheet.layout.clone(),
                index: sprite_sheet.first,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 1.0).with_scale(Vec3::splat(pet_scale)),
        Pet { pet_type },
        Hitbox::from_full_size(hitbox_size),
        CollisionLayer::pet(),
        AnimationIndices {
            first: sprite_sheet.first,
            last: sprite_sheet.last,
        },
        AnimationTimer(Timer::from_seconds(
            PET_ANIMATION_SECONDS,
            TimerMode::Repeating,
        )),
        Damage(damage),
        AttackSpeed(attack_speed),
        AttackRange(attack_range),
        DetectionRange(detection_range),
        PetMovementSpeed(movement_speed),
        AttackTimer::from_attack_speed(attack_speed),
        Team(Team::PLAYER),
        Velocity::default(),
    ));
}
