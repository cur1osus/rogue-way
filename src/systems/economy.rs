use crate::components::{
    AnimationIndices, AnimationTimer, Experience, Gold, GoldHighlightTimer, GoldPickup, Pet,
    PetType, Player, XpGem,
};
use crate::constants::XP_PICKUP_RADIUS;
use crate::resources::{GoldSprites, MetaProgression, UiFonts};
use bevy::prelude::*;

const GOLD_PICKUP_RADIUS: f32 = 60.0; // Золото подбирается чуть дальше

/// Система авто-сбора XP гемов и золота
pub fn pickup_system(
    mut commands: Commands,
    mut player_query: Query<(&Transform, &mut Gold), With<Player>>,
    pet_query: Query<(&Transform, &Pet)>,
    gem_query: Query<(Entity, &Transform, &XpGem)>,
    gold_query: Query<(Entity, &Transform, &GoldPickup)>,
    mut gain_xp_events: MessageWriter<GainXpEvent>,
) {
    let Ok((player_transform, mut player_gold)) = player_query.single_mut() else {
        return;
    };

    let xp_collectors: Vec<Vec2> = pet_query
        .iter()
        .filter_map(|(transform, pet)| {
            if pet.pet_type == PetType::XpCollector {
                Some(transform.translation.truncate())
            } else {
                None
            }
        })
        .collect();

    // Подбор XP
    for (gem_entity, gem_transform, gem) in gem_query.iter() {
        let gem_pos = gem_transform.translation;
        let distance = player_transform.translation.distance(gem_pos);
        let picked_by_pet = distance > XP_PICKUP_RADIUS
            && xp_collectors
                .iter()
                .any(|pos| pos.distance(gem_pos.truncate()) <= XP_PICKUP_RADIUS);

        if distance <= XP_PICKUP_RADIUS || picked_by_pet {
            commands.entity(gem_entity).despawn();
            gain_xp_events.write(GainXpEvent { amount: gem.value });
        }
    }

    // Подбор золота
    for (gold_entity, gold_transform, gold_pickup) in gold_query.iter() {
        let distance = player_transform
            .translation
            .distance(gold_transform.translation);

        if distance <= GOLD_PICKUP_RADIUS {
            player_gold.add(gold_pickup.value);
            commands.entity(gold_entity).despawn();
        }
    }
}

/// Событие получения опыта
#[derive(Message)]
pub struct GainXpEvent {
    pub amount: u32,
}

/// Система обработки получения опыта
pub fn gain_xp_system(
    mut gain_xp_events: MessageReader<GainXpEvent>,
    mut player_query: Query<&mut Experience, With<Player>>,
    mut level_up_events: MessageWriter<LevelUpEvent>,
) {
    for event in gain_xp_events.read() {
        if let Ok(mut experience) = player_query.single_mut() {
            let leveled_up = experience.add_xp(event.amount);

            if leveled_up {
                level_up_events.write(LevelUpEvent {
                    new_level: experience.level,
                });
            }
        }
    }
}

/// Событие повышения уровня
#[derive(Message)]
pub struct LevelUpEvent {
    #[allow(dead_code)]
    pub new_level: u32,
}

/// Система обработки повышения уровня
pub fn level_up_system(
    commands: Commands,
    mut level_up_events: MessageReader<LevelUpEvent>,
    #[allow(unused_mut)] mut next_state: ResMut<NextState<crate::ui::GameState>>,
    ui_fonts: Res<UiFonts>,
    meta: Res<MetaProgression>,
) {
    let mut should_show_ui = false;

    for _event in level_up_events.read() {
        should_show_ui = true;
    }

    if should_show_ui {
        // Показываем UI выбора апгрейда с учётом разблокированных питомцев
        crate::ui::show_level_up_ui(commands, next_state, ui_fonts, meta);
    }
}

/// Система анимации подсветки золота
pub fn gold_highlight_system(
    mut commands: Commands,
    time: Res<Time>,
    mut gold_query: Query<(Entity, &mut GoldHighlightTimer, &mut Sprite)>,
    gold_sprites: Res<GoldSprites>,
) {
    for (entity, mut timer, mut sprite) in gold_query.iter_mut() {
        // Обновляем таймер
        timer.timer.tick(time.delta());

        // Если таймер истёк и еще не включена подсветка
        if timer.timer.just_finished() && !timer.is_highlighted {
            timer.is_highlighted = true;

            // Переключаем на анимированный спрайт
            sprite.image = gold_sprites.highlight_texture.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: gold_sprites.highlight_layout.clone(),
                index: gold_sprites.highlight_first,
            });

            // Добавляем компоненты анимации
            let highlight_first = gold_sprites.highlight_first;
            let highlight_last = gold_sprites.highlight_last;
            commands.entity(entity).queue_silenced(
                move |mut entity: bevy::ecs::world::EntityWorldMut| {
                    entity.insert((
                        AnimationIndices {
                            first: highlight_first,
                            last: highlight_last,
                        },
                        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                    ));
                },
            );
        }
    }
}

/// Система анимации сверкающего золота
pub fn gold_animation_system(
    time: Res<Time>,
    mut gold_query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite), With<GoldPickup>>,
) {
    for (indices, mut timer, mut sprite) in gold_query.iter_mut() {
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

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
