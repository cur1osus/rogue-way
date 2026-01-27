use bevy::prelude::*;

use crate::components::{
    AttackRange, AttackSpeed, AttackTimer, Damage, Health, MovementSpeed, Pet, PetType, Player,
};
use crate::resources::{MetaProgression, PetSpriteSheet, UiFonts, UpgradeState};
use crate::systems::player::spawn_pet;
use rand::seq::SliceRandom;

/// Состояние игры для паузы во время выбора апгрейда
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Shop,
    Playing,
    LevelUpChoice,
}

/// Маркер для UI выбора апгрейда
#[derive(Component)]
pub struct LevelUpUI;

/// Маркер для кнопки выбора апгрейда
#[derive(Component)]
pub struct UpgradeButton {
    pub upgrade: UpgradeType,
}

/// Типы апгрейдов
#[derive(Debug, Clone)]
pub enum UpgradeType {
    // Новые питомцы
    SummonPet(PetType),

    // Улучшения питомцев
    PetDamageBoost(f32),      // +X% урона всем питомцам
    PetAttackSpeedBoost(f32), // +X% скорости атаки
    PetRangeBoost(f32),       // +X дальности

    // Улучшения игрока
    MaxHpBoost(f32),         // +X максимального HP
    HealPlayer(f32),         // Восстановить X% HP
    MovementSpeedBoost(f32), // +X% скорости движения

    // Специальные способности
    MultiShot,       // Питомцы стреляют дополнительным снарядом
    Piercing,        // Снаряды пробивают +1 врага
    AreaDamage(f32), // Урон в радиусе X

    // Экономика
    GoldDropBoost(f32), // +X% золота с врагов
}

impl UpgradeType {
    /// Получить название апгрейда
    pub fn get_name(&self) -> String {
        match self {
            UpgradeType::SummonPet(pet_type) => format!("Призвать {}", pet_type.get_name()),
            UpgradeType::PetDamageBoost(amount) => format!("Урон питомцев +{:.0}%", amount * 100.0),
            UpgradeType::PetAttackSpeedBoost(amount) => {
                format!("Скорость атаки +{:.0}%", amount * 100.0)
            }
            UpgradeType::PetRangeBoost(amount) => format!("Дальность +{:.0}", amount),
            UpgradeType::MaxHpBoost(amount) => format!("Макс. ОЗ +{:.0}", amount),
            UpgradeType::HealPlayer(amount) => format!("Лечение {:.0}%", amount * 100.0),
            UpgradeType::MovementSpeedBoost(amount) => format!("Скорость +{:.0}%", amount * 100.0),
            UpgradeType::MultiShot => "Мульти-выстрел".to_string(),
            UpgradeType::Piercing => "Пробивание".to_string(),
            UpgradeType::AreaDamage(radius) => format!("Урон по площади ({:.0})", radius),
            UpgradeType::GoldDropBoost(amount) => format!("Золото +{:.0}%", amount * 100.0),
        }
    }

    /// Получить описание апгрейда
    pub fn get_description(&self) -> String {
        match self {
            UpgradeType::SummonPet(pet_type) => match pet_type {
                PetType::GuardDog => "Кружится вокруг вас и атакует врагов".to_string(),
                PetType::FireSprite => "Стреляет пробивающими снарядами".to_string(),
                PetType::SlimeCompanion => "Замедляет врагов при попадании".to_string(),
                PetType::CrowScout => "Быстрый питомец с большой дальностью".to_string(),
            },
            UpgradeType::PetDamageBoost(_) => "Увеличивает урон всех питомцев".to_string(),
            UpgradeType::PetAttackSpeedBoost(_) => "Питомцы атакуют быстрее".to_string(),
            UpgradeType::PetRangeBoost(_) => "Увеличивает дальность атаки питомцев".to_string(),
            UpgradeType::MaxHpBoost(_) => "Увеличивает максимальное здоровье".to_string(),
            UpgradeType::HealPlayer(_) => "Мгновенно восстанавливает здоровье".to_string(),
            UpgradeType::MovementSpeedBoost(_) => "Вы двигаетесь быстрее".to_string(),
            UpgradeType::MultiShot => "Питомцы стреляют двойным залпом".to_string(),
            UpgradeType::Piercing => "Снаряды пробивают больше врагов".to_string(),
            UpgradeType::AreaDamage(_) => "Атаки наносят урон в радиусе".to_string(),
            UpgradeType::GoldDropBoost(_) => "Больше золота с каждого врага".to_string(),
        }
    }

    /// Получить все возможные апгрейды
    pub fn get_all_upgrades() -> Vec<UpgradeType> {
        vec![
            // Питомцы
            UpgradeType::SummonPet(PetType::GuardDog),
            UpgradeType::SummonPet(PetType::FireSprite),
            UpgradeType::SummonPet(PetType::SlimeCompanion),
            UpgradeType::SummonPet(PetType::CrowScout),
            // Улучшения питомцев
            UpgradeType::PetDamageBoost(0.15),
            UpgradeType::PetDamageBoost(0.25),
            UpgradeType::PetAttackSpeedBoost(0.15),
            UpgradeType::PetAttackSpeedBoost(0.25),
            UpgradeType::PetRangeBoost(30.0),
            UpgradeType::PetRangeBoost(50.0),
            // Улучшения игрока
            UpgradeType::MaxHpBoost(20.0),
            UpgradeType::MaxHpBoost(30.0),
            UpgradeType::HealPlayer(0.5),
            UpgradeType::HealPlayer(1.0),
            UpgradeType::MovementSpeedBoost(0.15),
            UpgradeType::MovementSpeedBoost(0.25),
            // Специальные
            UpgradeType::MultiShot,
            UpgradeType::Piercing,
            UpgradeType::AreaDamage(90.0),
            // Экономика
            UpgradeType::GoldDropBoost(0.2),
            UpgradeType::GoldDropBoost(0.35),
        ]
    }

    /// Выбрать 3 случайных апгрейда с учётом разблокированных питомцев (§3.2.1)
    pub fn choose_random_upgrades(meta: &MetaProgression) -> Vec<UpgradeType> {
        let mut rng = rand::thread_rng();
        let all_upgrades = Self::get_all_upgrades();

        // Фильтруем апгрейды - только разблокированные питомцы
        let available_upgrades: Vec<UpgradeType> = all_upgrades
            .into_iter()
            .filter(|upgrade| {
                if let UpgradeType::SummonPet(pet_type) = upgrade {
                    meta.save_data.is_pet_unlocked(&pet_type.to_string())
                } else {
                    true
                }
            })
            .collect();

        available_upgrades
            .choose_multiple(&mut rng, 3.min(available_upgrades.len()))
            .cloned()
            .collect()
    }
}

/// Показать UI выбора апгрейда
pub fn show_level_up_ui(
    mut commands: Commands,
    mut next_state: ResMut<NextState<GameState>>,
    ui_fonts: Res<UiFonts>,
    meta: Res<MetaProgression>,
) {
    // Ставим игру на паузу
    next_state.set(GameState::LevelUpChoice);

    // Получаем 3 случайных апгрейда с учётом разблокированных питомцев
    let upgrades = UpgradeType::choose_random_upgrades(&meta);
    let font = ui_fonts.main.clone();

    // Создаем UI
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            LevelUpUI,
        ))
        .with_children(|parent| {
            // Заголовок
            parent.spawn((
                Text::new("УРОВЕНЬ ПОВЫШЕН!"),
                TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
            ));

            // Контейнер для кнопок
            parent
                .spawn(Node {
                    width: Val::Percent(80.0),
                    height: Val::Px(300.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|buttons_parent| {
                    // Создаем 3 кнопки выбора
                    for upgrade in upgrades {
                        create_upgrade_button(buttons_parent, upgrade, font.clone());
                    }
                });
        });
}

/// Создать кнопку выбора апгрейда
fn create_upgrade_button(
    parent: &mut ChildSpawnerCommands,
    upgrade: UpgradeType,
    font: Handle<Font>,
) {
    let name = upgrade.get_name();
    let description = upgrade.get_description();

    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(250.0),
                height: Val::Px(200.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
            UpgradeButton {
                upgrade: upgrade.clone(),
            },
        ))
        .with_children(|button_parent| {
            // Название
            button_parent.spawn((
                Text::new(name),
                TextFont {
                    font: font.clone(),
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.5)),
            ));

            // Описание
            button_parent.spawn((
                Text::new(description),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
            ));
        });
}

/// Система обработки нажатий на кнопки апгрейдов
pub fn handle_upgrade_button(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &UpgradeButton), (Changed<Interaction>, With<Button>)>,
    ui_query: Query<Entity, With<LevelUpUI>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut upgrade_state: ResMut<UpgradeState>,
    mut player_query: Query<(&mut Health, &mut MovementSpeed), With<Player>>,
    mut pet_query: Query<
        (
            &mut Damage,
            &mut AttackSpeed,
            &mut AttackRange,
            &mut AttackTimer,
        ),
        With<Pet>,
    >,
    pet_sprites: Res<PetSpriteSheet>,
) {
    for (interaction, upgrade_button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            // Применяем апгрейд
            apply_upgrade(
                &mut commands,
                &upgrade_button.upgrade,
                &mut upgrade_state,
                &mut player_query,
                &mut pet_query,
                &pet_sprites,
            );

            // Удаляем UI (Bevy автоматически обработает дочерние элементы)
            for entity in ui_query.iter() {
                commands.entity(entity).despawn();
            }

            // Возвращаемся к игре
            next_state.set(GameState::Playing);
        }
    }
}

/// Применить апгрейд
fn apply_upgrade(
    commands: &mut Commands,
    upgrade: &UpgradeType,
    upgrade_state: &mut UpgradeState,
    player_query: &mut Query<(&mut Health, &mut MovementSpeed), With<Player>>,
    pet_query: &mut Query<
        (
            &mut Damage,
            &mut AttackSpeed,
            &mut AttackRange,
            &mut AttackTimer,
        ),
        With<Pet>,
    >,
    pet_sprites: &PetSpriteSheet,
) {
    match upgrade {
        UpgradeType::SummonPet(pet_type) => {
            spawn_pet(commands, *pet_type, Vec2::ZERO, upgrade_state, pet_sprites);
        }
        UpgradeType::PetDamageBoost(amount) => {
            upgrade_state.pet_damage_mult *= 1.0 + amount;
            for (mut damage, _, _, _) in pet_query.iter_mut() {
                damage.0 *= 1.0 + amount;
            }
        }
        UpgradeType::PetAttackSpeedBoost(amount) => {
            upgrade_state.pet_attack_speed_mult *= 1.0 + amount;
            for (_, mut attack_speed, _, mut attack_timer) in pet_query.iter_mut() {
                attack_speed.0 *= 1.0 + amount;
                *attack_timer = AttackTimer::from_attack_speed(attack_speed.0);
            }
        }
        UpgradeType::PetRangeBoost(amount) => {
            upgrade_state.pet_range_bonus += amount;
            for (_, _, mut range, _) in pet_query.iter_mut() {
                range.0 += amount;
            }
        }
        UpgradeType::MaxHpBoost(amount) => {
            if let Ok((mut health, _)) = player_query.single_mut() {
                health.max += amount;
                health.current += amount; // Также восстанавливаем HP
            }
        }
        UpgradeType::HealPlayer(percent) => {
            if let Ok((mut health, _)) = player_query.single_mut() {
                let heal_amount = health.max * percent;
                health.current = (health.current + heal_amount).min(health.max);
            }
        }
        UpgradeType::MovementSpeedBoost(amount) => {
            if let Ok((_, mut speed)) = player_query.single_mut() {
                speed.0 *= 1.0 + amount;
            }
        }
        UpgradeType::MultiShot => {
            upgrade_state.projectile_extra_shots += 1;
        }
        UpgradeType::Piercing => {
            upgrade_state.projectile_pierce_bonus += 1;
        }
        UpgradeType::AreaDamage(radius) => {
            upgrade_state.area_damage_radius += radius;
        }
        UpgradeType::GoldDropBoost(amount) => {
            upgrade_state.gold_drop_mult *= 1.0 + amount;
        }
    }
}
