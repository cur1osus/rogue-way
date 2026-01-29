use bevy::math::URect;
use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackRange, AttackSpeed, AttackTimer, CollisionLayer,
    Damage, DetectionRange, Experience, Gold, Health, Hitbox, LocalPlayer, MovementSpeed, Pet,
    PetBlackboard, PetMovementSpeed, PetOwner, PetType, PhysicsPosition, PickupRadius, Player,
    PlayerAnimation, PlayerId, PlayerInputState, PreviousPhysicsPosition, PushbackAttack,
    PushbackAttackCooldown, PushbackReadyGlow, PushbackReadyGlowPending, Team, Velocity,
};
use crate::constants::{
    PET_HITBOX_SCALE, PLAYER_HITBOX_SCALE, PLAYER_SCALE, PUSHBACK_READY_GLOW_SCALE,
    PUSHBACK_READY_GLOW_TINT, XP_PICKUP_RADIUS,
};
use crate::resources::{MetaProgression, UpgradeState};

const PLAYER_SPRITE_FRAMES: usize = 8;
const PLAYER_SPRITE_SHEET_WIDTH: u32 = 838;
const PLAYER_SPRITE_SHEET_HEIGHT: u32 = 135;
const PLAYER_FRAME_SIZE: Vec2 = Vec2::new(106.0, 135.0);
// Границы кадров (min_x, max_x) для текущего спрайта
const PLAYER_FRAME_X_RANGES: [(u32, u32); PLAYER_SPRITE_FRAMES] = [
    (2, 76),
    (102, 188),
    (192, 298),
    (312, 404),
    (426, 504),
    (528, 610),
    (626, 726),
    (744, 836),
];
const PET_ANIMATION_SECONDS: f32 = 0.12;

pub const LOCAL_PLAYER_ID: u32 = 1;

pub fn spawn_player_entity(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    meta: &Res<MetaProgression>,
    player_id: PlayerId,
    is_local: bool,
) -> Entity {
    let texture = asset_server.load("sprites/sprite.png");
    let mut layout = TextureAtlasLayout::new_empty(UVec2::new(
        PLAYER_SPRITE_SHEET_WIDTH,
        PLAYER_SPRITE_SHEET_HEIGHT,
    ));
    for (min_x, max_x) in PLAYER_FRAME_X_RANGES {
        layout.add_texture(URect {
            min: UVec2::new(min_x, 0),
            max: UVec2::new(max_x, PLAYER_SPRITE_SHEET_HEIGHT),
        });
    }
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let right_start = 0;
    let left_start = 0;

    let upgrades = &meta.save_data.permanent_upgrades;
    let base_hp = upgrades.get_max_hp();
    let base_speed = 200.0 * upgrades.get_movement_speed_multiplier();
    let starting_level = upgrades.get_starting_level();
    let starting_gold = upgrades.get_starting_gold();

    let mut entity_commands = commands.spawn((
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
        player_id,
        PlayerInputState::default(),
        Hitbox::from_full_size(PLAYER_FRAME_SIZE * PLAYER_SCALE * PLAYER_HITBOX_SCALE),
        CollisionLayer::player(),
        Health::new(base_hp),
        MovementSpeed(base_speed),
        Velocity::default(),
        Team(Team::PLAYER),
        Experience::new_with_level(starting_level),
        Gold::new(starting_gold),
    ));

    entity_commands.insert(AnimationIndices {
        first: right_start,
        last: right_start + PLAYER_SPRITE_FRAMES - 1,
    });
    entity_commands.insert(AnimationTimer(Timer::from_seconds(
        0.1,
        TimerMode::Repeating,
    )));
    entity_commands.insert(PlayerAnimation {
        right_start,
        left_start,
        frames: PLAYER_SPRITE_FRAMES,
        facing: 1,
    });

    if is_local {
        entity_commands.insert(LocalPlayer);
    }

    let player_entity = entity_commands
        .insert(PushbackAttack::default())
        .insert(PushbackAttackCooldown::default())
        .insert(PhysicsPosition(Vec2::ZERO))
        .insert(PreviousPhysicsPosition(Vec2::ZERO))
        .id();

    let tint = PUSHBACK_READY_GLOW_TINT.to_srgba();
    let glow_entity = commands
        .spawn((
            Sprite {
                color: Color::srgba(tint.red, tint.green, tint.blue, 0.0),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.3).with_scale(Vec3::splat(PUSHBACK_READY_GLOW_SCALE)),
            PushbackReadyGlow,
            PushbackReadyGlowPending,
        ))
        .id();

    commands.entity(player_entity).add_child(glow_entity);
    player_entity
}

/// Система настройки игрока (запускается один раз при старте)
pub fn setup_player(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    meta: Res<MetaProgression>,
    existing_player: Query<Entity, (With<Player>, With<LocalPlayer>)>,
) {
    // Если игрок уже существует, не создаем нового (возврат из LevelUpChoice)
    if !existing_player.is_empty() {
        return;
    }
    spawn_player_entity(
        &mut commands,
        &asset_server,
        &mut texture_atlas_layouts,
        &meta,
        PlayerId(LOCAL_PLAYER_ID),
        true,
    );
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
                Transform::from_xyz(pos_x, pos_y, -10.0),
            ));
        }
    }
}

/// Система следования камеры за игроком
pub fn camera_follow_system(
    time: Res<Time>,
    player_query: Query<&Transform, (With<Player>, With<LocalPlayer>)>,
    mut camera_query: Query<&mut Transform, (With<Camera>, Without<Player>)>,
    shake: Res<crate::resources::ScreenShake>,
) {
    const CAMERA_SMOOTHING: f32 = 20.0; // Более высокое значение = более резкая камера

    if let Ok(player_transform) = player_query.single() {
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            let target_x = player_transform.translation.x + shake.offset.x;
            let target_y = player_transform.translation.y + shake.offset.y;

            // Экспоненциальное сглаживание
            let t = 1.0 - (-CAMERA_SMOOTHING * time.delta_secs()).exp();

            camera_transform.translation.x += (target_x - camera_transform.translation.x) * t;
            camera_transform.translation.y += (target_y - camera_transform.translation.y) * t;
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
        let moving = velocity.0.length_squared() > 0.01;

        if velocity.0.x < -0.05 {
            animation.facing = -1;
        } else if velocity.0.x > 0.05 {
            animation.facing = 1;
        }

        sprite.flip_x = animation.facing < 0;

        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

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
    existing_pets: Query<&PetOwner, With<Pet>>,
    local_player: Query<&PlayerId, With<LocalPlayer>>,
) {
    // Если есть питомцы, значит это не первый запуск (возврат из LevelUpChoice)
    // В этом случае сохраняем существующих питомцев
    let Ok(local_id) = local_player.single() else {
        return;
    };
    let has_local_pets = existing_pets.iter().any(|owner| owner.0 == local_id.0);
    if has_local_pets {
        return;
    }

    // Только если это первый запуск (нет питомцев), создаем стартового
    spawn_pet(
        &mut commands,
        PetType::GuardDog,
        Vec2::ZERO,
        &upgrade_state,
        &pet_sprites,
        local_id.0,
    );
}

/// Функция для спавна питомца определенного типа
pub fn spawn_pet(
    commands: &mut Commands,
    pet_type: PetType,
    position: Vec2,
    upgrades: &UpgradeState,
    pet_sprites: &crate::resources::PetSpriteSheet,
    owner_id: u32,
) {
    let (
        base_damage,
        base_attack_speed,
        base_attack_range,
        base_detection_range,
        base_movement_speed,
    ) = pet_type.get_stats();
    let detection_range = base_detection_range + upgrades.pet_detection_range_bonus;
    let movement_speed = base_movement_speed * upgrades.pet_movement_speed_mult;

    // Получаем спрайт для питомца
    let sprite_sheet = if pet_type == PetType::XpCollector {
        let mut rng = rand::thread_rng();
        pet_sprites
            .xp_dog_idle
            .choose(&mut rng)
            .unwrap_or(&pet_sprites.xp_dog_run)
    } else {
        pet_sprites.get_idle_sheet(&pet_type)
    };
    let size = pet_type.get_size();
    let pet_scale = size / sprite_sheet.frame_size.y;
    let hitbox_size = Vec2::splat(size * PET_HITBOX_SCALE);

    let mut entity_commands = commands.spawn((
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
        PetOwner(owner_id),
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
        DetectionRange(detection_range),
        PetMovementSpeed(movement_speed),
        Team(Team::PLAYER),
        Velocity::default(),
        PhysicsPosition(position),
        PreviousPhysicsPosition(position),
        PetBlackboard::new(pet_type.get_role()),
    ));

    if pet_type == PetType::XpCollector {
        entity_commands.insert(PickupRadius(XP_PICKUP_RADIUS));
    } else {
        let damage = base_damage * upgrades.pet_damage_mult;
        let attack_speed = base_attack_speed * upgrades.pet_attack_speed_mult;
        let attack_range = base_attack_range + upgrades.pet_range_bonus;
        entity_commands.insert((
            Damage(damage),
            AttackSpeed(attack_speed),
            AttackRange(attack_range),
            AttackTimer::from_attack_speed(attack_speed),
        ));
    }
}
