use bevy::prelude::*;

use crate::components::{
    AttackRange, AttackSpeed, AttackTimer, Damage, Health, MovementSpeed, Pet, PetMovementSpeed,
    PetType, PhysicsPosition, Player, PlayerId, PushbackAttack,
};
use crate::constants::{ui_colors, ui_text, AREA_DAMAGE_CONE_ANGLE_MAX_DEG, UI_FONT_SCALE};
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
    Paused,
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
    PetDamageBoost(f32),        // +X% урона всем питомцам
    PetAttackSpeedBoost(f32),   // +X% скорости атаки
    PetRangeBoost(f32),         // +X дальности
    PetMovementSpeedBoost(f32), // +X% скорости передвижения

    // Улучшения игрока
    MaxHpBoost(f32),         // +X максимального HP
    HealPlayer(f32),         // Восстановить X% HP
    MovementSpeedBoost(f32), // +X% скорости движения
    PushbackConeBoost(f32),  // +X° угла отталкивания
    PushbackForceBoost(f32), // +X силы отталкивания

    // Специальные способности
    MultiShot,       // Питомцы стреляют дополнительным снарядом
    Piercing,        // Снаряды пробивают +1 врага
    AreaDamage(f32), // Урон по площади: угол сплеша в градусах

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
            UpgradeType::PetMovementSpeedBoost(amount) => {
                format!("Скорость питомцев +{:.0}%", amount * 100.0)
            }
            UpgradeType::MaxHpBoost(amount) => format!("Макс. ОЗ +{:.0}", amount),
            UpgradeType::HealPlayer(amount) => format!("Лечение {:.0}%", amount * 100.0),
            UpgradeType::MovementSpeedBoost(amount) => format!("Скорость +{:.0}%", amount * 100.0),
            UpgradeType::PushbackConeBoost(amount) => {
                format!("Угол отталкивания +{:.0}°", amount)
            }
            UpgradeType::PushbackForceBoost(amount) => {
                format!("Сила отталкивания +{:.0}", amount)
            }
            UpgradeType::MultiShot => "Мульти-выстрел".to_string(),
            UpgradeType::Piercing => "Пробивание".to_string(),
            UpgradeType::AreaDamage(angle_deg) => {
                format!("Угол сплеша +{:.0}°", angle_deg)
            }
            UpgradeType::GoldDropBoost(amount) => format!("Золото +{:.0}%", amount * 100.0),
        }
    }

    /// Получить описание апгрейда
    pub fn get_description(&self) -> String {
        match self {
            UpgradeType::SummonPet(pet_type) => match pet_type {
                PetType::GuardDog => "Воин ближнего боя, держится рядом и рубит врагов".to_string(),
                PetType::FireSprite => "Лучник, выпускает стрелы на дальнюю дистанцию".to_string(),
                PetType::SlimeCompanion => "Монах поддержки, замедляет врагов ударами".to_string(),
                PetType::CrowScout => "Разведчик с копьем и большой дальностью".to_string(),
                PetType::XpCollector => "Пёс-собиратель, подбирает XP гемы рядом".to_string(),
            },
            UpgradeType::PetDamageBoost(_) => "Увеличивает урон всех питомцев".to_string(),
            UpgradeType::PetAttackSpeedBoost(_) => "Питомцы атакуют быстрее".to_string(),
            UpgradeType::PetRangeBoost(_) => "Увеличивает дальность атаки питомцев".to_string(),
            UpgradeType::PetMovementSpeedBoost(_) => "Питомцы перемещаются быстрее".to_string(),
            UpgradeType::MaxHpBoost(_) => "Увеличивает максимальное здоровье".to_string(),
            UpgradeType::HealPlayer(_) => "Мгновенно восстанавливает здоровье".to_string(),
            UpgradeType::MovementSpeedBoost(_) => "Вы двигаетесь быстрее".to_string(),
            UpgradeType::PushbackConeBoost(_) => "Расширяет угол отталкивающей атаки".to_string(),
            UpgradeType::PushbackForceBoost(_) => {
                "Отталкивающая атака становится сильнее".to_string()
            }
            UpgradeType::MultiShot => "Питомцы стреляют двойным залпом".to_string(),
            UpgradeType::Piercing => "Снаряды пробивают больше врагов".to_string(),
            UpgradeType::AreaDamage(_) => "Атаки наносят урон в секторе".to_string(),
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
            UpgradeType::SummonPet(PetType::XpCollector),
            // Улучшения питомцев
            UpgradeType::PetDamageBoost(0.15),
            UpgradeType::PetDamageBoost(0.25),
            UpgradeType::PetAttackSpeedBoost(0.15),
            UpgradeType::PetAttackSpeedBoost(0.25),
            UpgradeType::PetRangeBoost(30.0),
            UpgradeType::PetRangeBoost(50.0),
            UpgradeType::PetMovementSpeedBoost(0.15),
            UpgradeType::PetMovementSpeedBoost(0.25),
            // Улучшения игрока
            UpgradeType::MaxHpBoost(20.0),
            UpgradeType::MaxHpBoost(30.0),
            UpgradeType::HealPlayer(0.5),
            UpgradeType::HealPlayer(1.0),
            UpgradeType::MovementSpeedBoost(0.15),
            UpgradeType::MovementSpeedBoost(0.25),
            UpgradeType::PushbackConeBoost(45.0),
            UpgradeType::PushbackForceBoost(200.0),
            // Специальные
            UpgradeType::MultiShot,
            UpgradeType::Piercing,
            UpgradeType::AreaDamage(20.0),
            // Экономика
            UpgradeType::GoldDropBoost(0.2),
            UpgradeType::GoldDropBoost(0.35),
        ]
    }

    /// Выбрать 4 случайных апгрейда с учётом разблокированных питомцев (§3.2.1)
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
            .choose_multiple(&mut rng, 4.min(available_upgrades.len()))
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

    // Получаем 4 случайных апгрейда с учётом разблокированных питомцев
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
            BackgroundColor(ui_colors::OVERLAY_DARK),
            LevelUpUI,
        ))
        .with_children(|parent| {
            // Заголовок
            parent.spawn((
                Text::new(ui_text::LEVEL_UP_TITLE),
                TextFont {
                    font: font.clone(),
                    font_size: 32.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_YELLOW),
            ));

            // Контейнер для кнопок
            parent
                .spawn(Node {
                    width: Val::Percent(90.0),
                    height: Val::Px(280.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceAround,
                    flex_direction: FlexDirection::Row,
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|buttons_parent| {
                    // Создаем 4 кнопки выбора
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
                width: Val::Px(220.0),
                height: Val::Px(200.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(ui_colors::BUTTON_DARK),
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
                    font_size: 20.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_YELLOW_WARM),
            ));

            // Описание
            button_parent.spawn((
                Text::new(description),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GRAY_LIGHT),
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
    mut player_query: Query<
        (
            &PlayerId,
            &PhysicsPosition,
            &mut Health,
            &mut MovementSpeed,
            &mut PushbackAttack,
        ),
        With<Player>,
    >,
    mut pet_query: Query<
        (
            &mut Damage,
            &mut AttackSpeed,
            &mut AttackRange,
            &mut AttackTimer,
            &mut PetMovementSpeed,
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
    player_query: &mut Query<
        (
            &PlayerId,
            &PhysicsPosition,
            &mut Health,
            &mut MovementSpeed,
            &mut PushbackAttack,
        ),
        With<Player>,
    >,
    pet_query: &mut Query<
        (
            &mut Damage,
            &mut AttackSpeed,
            &mut AttackRange,
            &mut AttackTimer,
            &mut PetMovementSpeed,
        ),
        With<Pet>,
    >,
    pet_sprites: &PetSpriteSheet,
) {
    match upgrade {
        UpgradeType::SummonPet(pet_type) => {
            for (player_id, physics_pos, _, _, _) in player_query.iter_mut() {
                spawn_pet(
                    commands,
                    *pet_type,
                    physics_pos.0,
                    upgrade_state,
                    pet_sprites,
                    player_id.0,
                );
            }
        }
        UpgradeType::PetDamageBoost(amount) => {
            upgrade_state.pet_damage_mult *= 1.0 + amount;
            for (mut damage, _, _, _, _) in pet_query.iter_mut() {
                damage.0 *= 1.0 + amount;
            }
        }
        UpgradeType::PetAttackSpeedBoost(amount) => {
            upgrade_state.pet_attack_speed_mult *= 1.0 + amount;
            for (_, mut attack_speed, _, mut attack_timer, _) in pet_query.iter_mut() {
                attack_speed.0 *= 1.0 + amount;
                *attack_timer = AttackTimer::from_attack_speed(attack_speed.0);
            }
        }
        UpgradeType::PetRangeBoost(amount) => {
            upgrade_state.pet_range_bonus += amount;
            for (_, _, mut range, _, _) in pet_query.iter_mut() {
                range.0 += amount;
            }
        }
        UpgradeType::PetMovementSpeedBoost(amount) => {
            upgrade_state.pet_movement_speed_mult *= 1.0 + amount;
            for (_, _, _, _, mut movement_speed) in pet_query.iter_mut() {
                movement_speed.0 *= 1.0 + amount;
            }
        }
        UpgradeType::MaxHpBoost(amount) => {
            for (_, _, mut health, _, _) in player_query.iter_mut() {
                health.max += amount;
                health.current += amount;
            }
        }
        UpgradeType::HealPlayer(percent) => {
            for (_, _, mut health, _, _) in player_query.iter_mut() {
                let heal_amount = health.max * percent;
                health.current = (health.current + heal_amount).min(health.max);
            }
        }
        UpgradeType::MovementSpeedBoost(amount) => {
            for (_, _, _, mut speed, _) in player_query.iter_mut() {
                speed.0 *= 1.0 + amount;
            }
        }
        UpgradeType::PushbackConeBoost(amount) => {
            for (_, _, _, _, mut pushback) in player_query.iter_mut() {
                let new_angle = pushback.cone_angle + amount.to_radians();
                pushback.cone_angle = new_angle.min(std::f32::consts::TAU);
            }
        }
        UpgradeType::PushbackForceBoost(amount) => {
            for (_, _, _, _, mut pushback) in player_query.iter_mut() {
                pushback.pushback_force += amount;
            }
        }
        UpgradeType::MultiShot => {
            upgrade_state.projectile_extra_shots += 1;
        }
        UpgradeType::Piercing => {
            upgrade_state.projectile_pierce_bonus += 1;
        }
        UpgradeType::AreaDamage(angle_deg) => {
            upgrade_state.area_damage_cone_angle_deg = (upgrade_state.area_damage_cone_angle_deg
                + angle_deg)
                .min(AREA_DAMAGE_CONE_ANGLE_MAX_DEG);
        }
        UpgradeType::GoldDropBoost(amount) => {
            upgrade_state.gold_drop_mult *= 1.0 + amount;
        }
    }
}
