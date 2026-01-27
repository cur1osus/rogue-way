use bevy::prelude::*;

use crate::components::{
    AnimationIndices, AnimationTimer, AttackAnimation, EffectSprite, Enemy, Pet, PetType, Velocity,
};
use crate::resources::{EnemySpriteSheet, PetSpriteSheet};

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
        With<Enemy>,
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
