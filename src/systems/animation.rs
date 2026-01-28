use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackAnimation, DeathAnimation, EffectSprite, Enemy, Pet,
    PetType, Velocity,
};
use crate::resources::{EnemySpriteSheet, PetAnimationSheet, PetSpriteSheet};

pub fn enemy_animation_system(
    time: Res<Time>,
    enemy_sprites: Res<EnemySpriteSheet>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut Sprite,
            &mut AnimationIndices,
            &mut AnimationTimer,
            Option<&mut AttackAnimation>,
        ),
        (With<Enemy>, Without<DeathAnimation>),
    >,
) {
    for (entity, mut sprite, mut indices, mut timer, attack_animation) in query.iter_mut() {
        let mut attack_active = false;
        if let Some(mut attack_animation) = attack_animation {
            attack_animation.timer.tick(time.delta());
            if attack_animation.timer.is_finished() {
                commands.entity(entity).remove::<AttackAnimation>();
            } else {
                attack_active = true;
            }
        }

        let sheet = if attack_active {
            &enemy_sprites.attack
        } else {
            &enemy_sprites.run
        };

        if sprite.image != sheet.texture {
            sprite.image = sheet.texture.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: sheet.layout.clone(),
                index: sheet.first,
            });
            indices.first = sheet.first;
            indices.last = sheet.last;
            timer.reset();
        } else {
            indices.first = sheet.first;
            indices.last = sheet.last;
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.layout = sheet.layout.clone();
            } else {
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: sheet.first,
                });
            }
        }

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

pub fn pet_animation_system(
    time: Res<Time>,
    pet_sprites: Res<PetSpriteSheet>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Pet,
        &Velocity,
        &mut Sprite,
        &mut AnimationIndices,
        &mut AnimationTimer,
        Option<&mut AttackAnimation>,
    )>,
) {
    for (entity, pet, velocity, mut sprite, mut indices, mut timer, attack_animation) in
        query.iter_mut()
    {
        if pet.pet_type == PetType::XpCollector {
            let moving = velocity.0.length_squared() > 0.01;
            let idle_options = &pet_sprites.xp_dog_idle;
            if idle_options.is_empty() {
                continue;
            }
            let run_sheet = &pet_sprites.xp_dog_run;
            let mut rng = rand::thread_rng();
            let current_sheet = if moving {
                run_sheet
            } else {
                idle_options
                    .iter()
                    .find(|sheet| sheet.texture == sprite.image)
                    .unwrap_or_else(|| idle_options.choose(&mut rng).unwrap())
            };

            apply_pet_sheet(&mut sprite, &mut indices, &mut timer, current_sheet);

            if velocity.0.x < -0.05 {
                sprite.flip_x = true;
            } else if velocity.0.x > 0.05 {
                sprite.flip_x = false;
            }

            let mut pending_sheet: Option<&PetAnimationSheet> = None;
            let mut reset_to_first = false;

            {
                let Some(atlas) = sprite.texture_atlas.as_mut() else {
                    continue;
                };

                timer.tick(time.delta());

                if atlas.index < indices.first || atlas.index > indices.last {
                    atlas.index = indices.first;
                }

                if timer.just_finished() {
                    if atlas.index == indices.last {
                        reset_to_first = true;
                        if !moving {
                            let mut next_sheet = current_sheet;
                            if idle_options.len() > 1 {
                                while next_sheet.texture == current_sheet.texture {
                                    next_sheet = idle_options.choose(&mut rng).unwrap();
                                }
                            }
                            if next_sheet.texture != current_sheet.texture {
                                pending_sheet = Some(next_sheet);
                            }
                        }
                    } else {
                        atlas.index += 1;
                    }
                }
            }

            if let Some(next_sheet) = pending_sheet {
                apply_pet_sheet(&mut sprite, &mut indices, &mut timer, next_sheet);
            }

            if reset_to_first {
                if let Some(atlas) = sprite.texture_atlas.as_mut() {
                    atlas.index = indices.first;
                }
            }

            continue;
        }

        let mut attack_active = false;
        if let Some(mut attack_animation) = attack_animation {
            attack_animation.timer.tick(time.delta());
            if attack_animation.timer.is_finished() {
                commands.entity(entity).remove::<AttackAnimation>();
            } else if pet.pet_type == PetType::GuardDog {
                attack_active = true;
            }
        }

        let moving = velocity.0.length_squared() > 0.01;
        let sheet = if attack_active {
            &pet_sprites.guard_dog_attack
        } else if moving {
            pet_sprites.get_run_sheet(&pet.pet_type)
        } else {
            pet_sprites.get_idle_sheet(&pet.pet_type)
        };

        // Поворачиваем спрайт в направлении движения (flip по X)
        if velocity.0.x < -0.05 {
            sprite.flip_x = true;
        } else if velocity.0.x > 0.05 {
            sprite.flip_x = false;
        }

        // Проверяем, изменилась ли текстура
        if sprite.image != sheet.texture {
            sprite.image = sheet.texture.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: sheet.layout.clone(),
                index: sheet.first,
            });
            indices.first = sheet.first;
            indices.last = sheet.last;
            timer.reset();
        } else {
            indices.first = sheet.first;
            indices.last = sheet.last;
            if let Some(atlas) = sprite.texture_atlas.as_mut() {
                atlas.layout = sheet.layout.clone();
            } else {
                sprite.texture_atlas = Some(TextureAtlas {
                    layout: sheet.layout.clone(),
                    index: sheet.first,
                });
            }
        }

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

pub fn effect_animation_system(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite), With<EffectSprite>>,
) {
    for (indices, mut timer, mut sprite) in query.iter_mut() {
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

fn apply_pet_sheet(
    sprite: &mut Sprite,
    indices: &mut AnimationIndices,
    timer: &mut AnimationTimer,
    sheet: &PetAnimationSheet,
) {
    if sprite.image != sheet.texture {
        sprite.image = sheet.texture.clone();
        sprite.texture_atlas = Some(TextureAtlas {
            layout: sheet.layout.clone(),
            index: sheet.first,
        });
        indices.first = sheet.first;
        indices.last = sheet.last;
        timer.reset();
    } else {
        indices.first = sheet.first;
        indices.last = sheet.last;
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.layout = sheet.layout.clone();
        } else {
            sprite.texture_atlas = Some(TextureAtlas {
                layout: sheet.layout.clone(),
                index: sheet.first,
            });
        }
    }
}

/// Система анимации смерти врага
pub fn enemy_death_animation_system(
    time: Res<Time>,
    enemy_sprites: Res<EnemySpriteSheet>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut Sprite,
            &mut AnimationIndices,
            &mut AnimationTimer,
            &mut DeathAnimation,
        ),
        With<Enemy>,
    >,
) {
    for (entity, mut sprite, mut indices, mut timer, mut death_anim) in query.iter_mut() {
        // Переключаем на спрайт смерти при первом кадре
        if !death_anim.animation_started {
            death_anim.animation_started = true;

            let sheet = &enemy_sprites.death;
            sprite.image = sheet.texture.clone();
            sprite.texture_atlas = Some(TextureAtlas {
                layout: sheet.layout.clone(),
                index: sheet.first,
            });
            indices.first = sheet.first;
            indices.last = sheet.last;
            timer.reset();
        }

        // Анимируем кадры смерти
        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

        timer.tick(time.delta());

        if timer.just_finished() {
            if atlas.index < indices.last {
                atlas.index += 1;
            }
            // НЕ зацикливаем - останавливаемся на последнем кадре
        }

        // Тикаем таймер смерти
        death_anim.timer.tick(time.delta());

        // Удаляем сущность когда анимация закончилась
        if death_anim.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
