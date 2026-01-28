use bevy::prelude::*;

use crate::components::{Gold, GoldHighlightTimer, GoldPickup, Health, PetType, Player, XpGem};
use crate::constants::{GOLD_SCALE, XP_GEM_SCALE};
use crate::resources::{GoldSprites, PetSpriteSheet, UpgradeState, XpGemSprites};
use crate::systems::economy::GainXpEvent;
use crate::systems::player::spawn_pet;
use crate::ui::DevPanelVisible;

/// Ресурс для управления отображением хитбоксов
#[allow(dead_code)]
#[derive(Resource, Default)]
pub struct ShowHitboxes(pub bool);

pub fn dev_panel_actions_system(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    dev_visible: Res<DevPanelVisible>,
    mut player_query: Query<(&Transform, &mut Gold, &mut Health), With<Player>>,
    mut gain_xp_events: MessageWriter<GainXpEvent>,
    xp_gem_sprites: Res<XpGemSprites>,
    gold_sprites: Res<GoldSprites>,
    upgrade_state: Res<UpgradeState>,
    pet_sprites: Res<PetSpriteSheet>,
    mut spawn_index: Local<u32>,
) {
    if !dev_visible.0 {
        return;
    }

    let Ok((player_transform, mut player_gold, mut player_health)) = player_query.single_mut()
    else {
        return;
    };

    let player_pos = player_transform.translation.truncate();

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        spawn_xp_gem(
            &mut commands,
            &xp_gem_sprites,
            player_pos + next_spawn_offset(&mut spawn_index, 24.0),
            25,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        spawn_xp_gem(
            &mut commands,
            &xp_gem_sprites,
            player_pos + next_spawn_offset(&mut spawn_index, 24.0),
            100,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit3) {
        spawn_gold_coin(
            &mut commands,
            &gold_sprites,
            player_pos + next_spawn_offset(&mut spawn_index, 26.0),
            50,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit4) {
        player_gold.add(500);
    }

    if keyboard_input.just_pressed(KeyCode::Digit5) {
        player_health.current = player_health.max;
    }

    if keyboard_input.just_pressed(KeyCode::Digit6) {
        gain_xp_events.write(GainXpEvent { amount: 100 });
    }

    if keyboard_input.just_pressed(KeyCode::Digit7) {
        spawn_pet(
            &mut commands,
            PetType::GuardDog,
            player_pos + next_spawn_offset(&mut spawn_index, 32.0),
            &upgrade_state,
            &pet_sprites,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit8) {
        spawn_pet(
            &mut commands,
            PetType::FireSprite,
            player_pos + next_spawn_offset(&mut spawn_index, 32.0),
            &upgrade_state,
            &pet_sprites,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit9) {
        spawn_pet(
            &mut commands,
            PetType::SlimeCompanion,
            player_pos + next_spawn_offset(&mut spawn_index, 32.0),
            &upgrade_state,
            &pet_sprites,
        );
    }

    if keyboard_input.just_pressed(KeyCode::Digit0) {
        spawn_pet(
            &mut commands,
            PetType::CrowScout,
            player_pos + next_spawn_offset(&mut spawn_index, 32.0),
            &upgrade_state,
            &pet_sprites,
        );
    }
}

fn spawn_xp_gem(commands: &mut Commands, sprites: &XpGemSprites, position: Vec2, value: u32) {
    commands.spawn((
        XpGem { value },
        Sprite {
            image: sprites.texture.clone(),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.5).with_scale(Vec3::splat(XP_GEM_SCALE)),
    ));
}

fn spawn_gold_coin(commands: &mut Commands, sprites: &GoldSprites, position: Vec2, value: u32) {
    commands.spawn((
        GoldPickup { value },
        Sprite {
            image: sprites.normal_texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: sprites.normal_layout.clone(),
                index: sprites.normal_index,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, 0.5).with_scale(Vec3::splat(GOLD_SCALE)),
        GoldHighlightTimer {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
            is_highlighted: false,
        },
    ));
}

fn next_spawn_offset(spawn_index: &mut u32, radius: f32) -> Vec2 {
    let angle = (*spawn_index as f32) * 0.7;
    *spawn_index = spawn_index.wrapping_add(1);
    Vec2::new(angle.cos(), angle.sin()) * radius
}
