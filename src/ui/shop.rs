use bevy::prelude::*;

use crate::components::PetType;
use crate::resources::{MetaProgression, UiFonts};
use crate::ui::{GameState, ScrollContainer, ScrollContent};
use bevy::ui::OverflowAxis;

/// Маркер для UI магазина
#[derive(Component)]
pub struct ShopUI;

/// Маркер для кнопки покупки в магазине
#[derive(Component)]
pub struct ShopPurchaseButton {
    pub item: ShopItem,
}

/// Маркер для кнопки "Начать забег"
#[derive(Component)]
pub struct StartRunButton;

/// Маркер для кнопки "Назад" (возврат в главное меню)
#[derive(Component)]
pub struct BackToMenuButton;

/// Маркер для текста с балансом золота
#[derive(Component)]
pub struct GoldBalanceText;

/// Типы предметов в магазине
#[derive(Debug, Clone, PartialEq)]
pub enum ShopItem {
    // Разблокировка питомцев (§3.2.1)
    UnlockPet(PetType, u32), // тип питомца, стоимость

    // Улучшения базовых статов (§3.2.2)
    MaxHpUpgrade(u32),         // стоимость: 100 золота
    MovementSpeedUpgrade(u32), // стоимость: 150 золота
    StartingXpUpgrade(u32),    // стоимость: 500 золота
    PetSlotUpgrade(u32),       // стоимость: 1000 золота

    // Экономические улучшения (§3.2.3)
    GoldMultiplierUpgrade(u32), // стоимость: 300 золота
    LuckUpgrade(u32),           // стоимость: 400 золота
    StartingGoldUpgrade(u32),   // стоимость: 250 золота
}

impl ShopItem {
    /// Получить название предмета
    pub fn get_name(&self) -> String {
        match self {
            ShopItem::UnlockPet(pet_type, _) => format!("Разблокировать {}", pet_type.get_name()),
            ShopItem::MaxHpUpgrade(_) => "Макс. ОЗ +10".to_string(),
            ShopItem::MovementSpeedUpgrade(_) => "Скорость +5%".to_string(),
            ShopItem::StartingXpUpgrade(_) => "Начальный уровень +1".to_string(),
            ShopItem::PetSlotUpgrade(_) => "Слот питомца +1".to_string(),
            ShopItem::GoldMultiplierUpgrade(_) => "Золото +10%".to_string(),
            ShopItem::LuckUpgrade(_) => "Удача +5%".to_string(),
            ShopItem::StartingGoldUpgrade(_) => "Стартовое золото +50".to_string(),
        }
    }

    /// Получить описание предмета
    pub fn get_description(&self) -> String {
        match self {
            ShopItem::UnlockPet(pet_type, _) => match pet_type {
                PetType::GuardDog => "Верный пёс, кружится вокруг вас".to_string(),
                PetType::FireSprite => "Огненный дух с пробивающими снарядами".to_string(),
                PetType::SlimeCompanion => "Слизь, замедляющая врагов".to_string(),
                PetType::CrowScout => "Быстрая ворона-разведчик".to_string(),
            },
            ShopItem::MaxHpUpgrade(_) => "Постоянно +10 к макс. здоровью".to_string(),
            ShopItem::MovementSpeedUpgrade(_) => "Постоянно +5% к скорости движения".to_string(),
            ShopItem::StartingXpUpgrade(_) => "Начинайте забег с более высоким уровнем".to_string(),
            ShopItem::PetSlotUpgrade(_) => {
                "Можно иметь на 1 питомца больше одновременно".to_string()
            }
            ShopItem::GoldMultiplierUpgrade(_) => "Постоянно +10% золота с врагов".to_string(),
            ShopItem::LuckUpgrade(_) => "Лучшие варианты при повышении уровня".to_string(),
            ShopItem::StartingGoldUpgrade(_) => "Начинайте забег с бонусным золотом".to_string(),
        }
    }

    /// Получить стоимость предмета
    pub fn get_cost(&self) -> u32 {
        match self {
            ShopItem::UnlockPet(_, cost) => *cost,
            ShopItem::MaxHpUpgrade(cost) => *cost,
            ShopItem::MovementSpeedUpgrade(cost) => *cost,
            ShopItem::StartingXpUpgrade(cost) => *cost,
            ShopItem::PetSlotUpgrade(cost) => *cost,
            ShopItem::GoldMultiplierUpgrade(cost) => *cost,
            ShopItem::LuckUpgrade(cost) => *cost,
            ShopItem::StartingGoldUpgrade(cost) => *cost,
        }
    }

    /// Проверить доступен ли предмет для покупки
    pub fn is_available(&self, meta: &MetaProgression) -> bool {
        match self {
            ShopItem::UnlockPet(pet_type, _) => {
                !meta.save_data.is_pet_unlocked(&pet_type.to_string())
            }
            ShopItem::MaxHpUpgrade(_) => meta.save_data.permanent_upgrades.max_hp_level < 20,
            ShopItem::MovementSpeedUpgrade(_) => {
                meta.save_data.permanent_upgrades.movement_speed_level < 20
            }
            ShopItem::StartingXpUpgrade(_) => {
                meta.save_data.permanent_upgrades.starting_xp_level < 10
            }
            ShopItem::PetSlotUpgrade(_) => meta.save_data.permanent_upgrades.pet_slots < 3,
            ShopItem::GoldMultiplierUpgrade(_) => {
                meta.save_data.permanent_upgrades.gold_multiplier_level < 20
            }
            ShopItem::LuckUpgrade(_) => meta.save_data.permanent_upgrades.luck_level < 20,
            ShopItem::StartingGoldUpgrade(_) => {
                meta.save_data.permanent_upgrades.starting_gold_level < 20
            }
        }
    }

    /// Применить покупку к мета-прогрессии
    pub fn apply_purchase(&self, meta: &mut MetaProgression) {
        match self {
            ShopItem::UnlockPet(pet_type, _) => {
                meta.save_data.unlock_pet(&pet_type.to_string());
            }
            ShopItem::MaxHpUpgrade(_) => {
                meta.save_data.permanent_upgrades.max_hp_level += 1;
            }
            ShopItem::MovementSpeedUpgrade(_) => {
                meta.save_data.permanent_upgrades.movement_speed_level += 1;
            }
            ShopItem::StartingXpUpgrade(_) => {
                meta.save_data.permanent_upgrades.starting_xp_level += 1;
            }
            ShopItem::PetSlotUpgrade(_) => {
                meta.save_data.permanent_upgrades.pet_slots += 1;
            }
            ShopItem::GoldMultiplierUpgrade(_) => {
                meta.save_data.permanent_upgrades.gold_multiplier_level += 1;
            }
            ShopItem::LuckUpgrade(_) => {
                meta.save_data.permanent_upgrades.luck_level += 1;
            }
            ShopItem::StartingGoldUpgrade(_) => {
                meta.save_data.permanent_upgrades.starting_gold_level += 1;
            }
        }
        meta.mark_dirty();
    }

    /// Получить все доступные предметы магазина
    pub fn get_all_items() -> Vec<ShopItem> {
        vec![
            // Питомцы (§3.2.1)
            ShopItem::UnlockPet(PetType::GuardDog, 500),
            ShopItem::UnlockPet(PetType::FireSprite, 800),
            ShopItem::UnlockPet(PetType::SlimeCompanion, 1200),
            ShopItem::UnlockPet(PetType::CrowScout, 2000),
            // Базовые статы (§3.2.2)
            ShopItem::MaxHpUpgrade(100),
            ShopItem::MovementSpeedUpgrade(150),
            ShopItem::StartingXpUpgrade(500),
            ShopItem::PetSlotUpgrade(1000),
            // Экономика (§3.2.3)
            ShopItem::GoldMultiplierUpgrade(300),
            ShopItem::LuckUpgrade(400),
            ShopItem::StartingGoldUpgrade(250),
        ]
    }
}

/// Настройка UI магазина
pub fn setup_shop_ui(
    mut commands: Commands,
    ui_fonts: Res<UiFonts>,
    meta: Res<MetaProgression>,
    existing_shop: Query<Entity, With<ShopUI>>,
) {
    // Если UI магазина уже существует, не создаём новый
    if !existing_shop.is_empty() {
        return;
    }

    let font = ui_fonts.main.clone();

    // Основной контейнер
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::all(Val::Px(20.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
            ShopUI,
        ))
        .with_children(|parent| {
            // Заголовок
            parent.spawn((
                Text::new("МАГАЗИН ТОРГОВЦА"),
                TextFont {
                    font: font.clone(),
                    font_size: 48.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
            ));

            // Баланс золота
            parent.spawn((
                Text::new(format!("Золото: {}", meta.save_data.total_gold)),
                TextFont {
                    font: font.clone(),
                    font_size: 32.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.0)),
                GoldBalanceText,
            ));

            // Скроллируемый контейнер для предметов
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(70.0),
                        overflow: Overflow {
                            x: OverflowAxis::Visible,
                            y: OverflowAxis::Clip,
                        },
                        flex_direction: FlexDirection::Column,
                        position_type: PositionType::Relative,
                        ..default()
                    },
                    ScrollContainer::default(),
                ))
                .with_children(|scroll_parent| {
                    // Сетка предметов
                    scroll_parent
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                align_items: AlignItems::FlexStart,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::all(Val::Px(20.0)),
                                row_gap: Val::Px(15.0),
                                column_gap: Val::Px(15.0),
                                position_type: PositionType::Relative,
                                ..default()
                            },
                            ScrollContent,
                        ))
                        .with_children(|grid_parent| {
                            // Создаем карточки предметов
                            for item in ShopItem::get_all_items() {
                                create_shop_item_card(grid_parent, item, font.clone(), &meta);
                            }
                        });
                });

            // Контейнер для кнопок внизу
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    column_gap: Val::Px(20.0),
                    margin: UiRect::all(Val::Px(20.0)),
                    ..default()
                })
                .with_children(|buttons_parent| {
                    // Кнопка "Назад"
                    buttons_parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.4, 0.4, 0.4)),
                            BackToMenuButton,
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new("← НАЗАД"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 24.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            ));
                        });

                    // Кнопка "Начать забег"
                    buttons_parent
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(300.0),
                                height: Val::Px(80.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.2, 0.6, 0.2)),
                            StartRunButton,
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new("НАЧАТЬ ЗАБЕГ"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 28.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            ));
                        });
                });
        });
}

/// Создать карточку предмета в магазине
fn create_shop_item_card(
    parent: &mut ChildSpawnerCommands,
    item: ShopItem,
    font: Handle<Font>,
    meta: &MetaProgression,
) {
    let is_available = item.is_available(meta);
    let can_afford = meta.save_data.total_gold >= item.get_cost();
    let is_purchasable = is_available && can_afford;

    let card_color = if !is_available {
        Color::srgb(0.3, 0.3, 0.3) // Серый - уже куплено
    } else if can_afford {
        Color::srgb(0.2, 0.3, 0.4) // Синеватый - можно купить
    } else {
        Color::srgb(0.4, 0.2, 0.2) // Красноватый - не хватает золота
    };

    let mut entity_commands = parent.spawn((
        Button,
        Node {
            width: Val::Px(280.0),
            height: Val::Px(200.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceBetween,
            padding: UiRect::all(Val::Px(15.0)),
            ..default()
        },
        BackgroundColor(card_color),
    ));

    // Добавляем компонент только если предмет можно купить
    if is_purchasable {
        entity_commands.insert(ShopPurchaseButton { item: item.clone() });
    }

    entity_commands.with_children(|card_parent| {
        // Название
        card_parent.spawn((
            Text::new(item.get_name()),
            TextFont {
                font: font.clone(),
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 1.0, 1.0)),
        ));

        // Описание
        card_parent.spawn((
            Text::new(item.get_description()),
            TextFont {
                font: font.clone(),
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                max_width: Val::Px(250.0),
                ..default()
            },
        ));

        // Стоимость
        let cost_text = if !is_available {
            "КУПЛЕНО".to_string()
        } else {
            format!("{} золота", item.get_cost())
        };

        let cost_color = if !is_available {
            Color::srgb(0.5, 0.5, 0.5)
        } else if can_afford {
            Color::srgb(1.0, 1.0, 0.0)
        } else {
            Color::srgb(1.0, 0.3, 0.3)
        };

        card_parent.spawn((
            Text::new(cost_text),
            TextFont {
                font: font.clone(),
                font_size: 18.0,
                ..default()
            },
            TextColor(cost_color),
        ));
    });
}

/// Обработка нажатий на кнопки магазина
pub fn handle_shop_buttons(
    mut commands: Commands,
    interaction_query: Query<
        (&Interaction, &ShopPurchaseButton),
        (Changed<Interaction>, With<Button>),
    >,
    start_run_query: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<StartRunButton>,
            Without<ShopPurchaseButton>,
            Without<BackToMenuButton>,
        ),
    >,
    back_to_menu_query: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<BackToMenuButton>,
            Without<ShopPurchaseButton>,
            Without<StartRunButton>,
        ),
    >,
    mut meta: ResMut<MetaProgression>,
    mut next_state: ResMut<NextState<GameState>>,
    shop_ui_query: Query<Entity, With<ShopUI>>,
) {
    // Обработка покупок
    for (interaction, purchase_button) in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            let cost = purchase_button.item.get_cost();
            if meta.save_data.spend_gold(cost) {
                purchase_button.item.apply_purchase(&mut meta);

                // Пересоздаем UI магазина
                for entity in shop_ui_query.iter() {
                    commands.entity(entity).despawn();
                }
                // UI будет пересоздан через систему setup
            }
        }
    }

    // Обработка кнопки "Начать забег"
    for interaction in start_run_query.iter() {
        if *interaction == Interaction::Pressed {
            // Удаляем UI магазина
            for entity in shop_ui_query.iter() {
                commands.entity(entity).despawn();
            }
            // Переключаемся в игровое состояние
            next_state.set(GameState::Playing);
        }
    }

    // Обработка кнопки "Назад"
    for interaction in back_to_menu_query.iter() {
        if *interaction == Interaction::Pressed {
            // Удаляем UI магазина
            for entity in shop_ui_query.iter() {
                commands.entity(entity).despawn();
            }
            // Возвращаемся в главное меню
            next_state.set(GameState::MainMenu);
        }
    }
}

/// Обновление текста баланса золота
pub fn update_gold_balance(
    meta: Res<MetaProgression>,
    mut text_query: Query<&mut Text, With<GoldBalanceText>>,
) {
    if meta.is_changed() {
        for mut text in text_query.iter_mut() {
            text.0 = format!("Золото: {}", meta.save_data.total_gold);
        }
    }
}
