use crate::resources::UiFonts;
use crate::ui::GameState;
use bevy::app::AppExit;
use bevy::prelude::*;

/// Маркер для UI главного меню
#[derive(Component)]
pub struct MainMenuUI;

/// Маркер для спрайтов меню (для очистки)
#[derive(Component)]
pub struct MenuSprite;

/// Маркер для кнопки "Играть"
#[derive(Component)]
pub struct PlayButton;

/// Маркер для кнопки "Выход"
#[derive(Component)]
pub struct QuitButton;

#[derive(Component, Copy, Clone)]
pub struct ButtonBaseColor(pub Color);

/// Компонент для анимации покачивания
#[derive(Component)]
pub struct FloatingAnimation {
    pub offset: f32,
    pub speed: f32,
    pub amplitude: f32,
}

/// Настройка UI главного меню
pub fn setup_main_menu(
    mut commands: Commands,
    ui_fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let font = ui_fonts.main.clone();

    // Создаем TextureAtlasLayout для спрайтов персонажей
    // Warrior (Guard Dog) - 8 кадров 192x192
    let warrior_texture =
        asset_server.load("sprites/Units/Blue Units/Warrior/Warrior_Idle.png".to_string());
    let warrior_layout = TextureAtlasLayout::from_grid(UVec2::new(192, 192), 8, 1, None, None);
    let warrior_layout_handle = texture_atlas_layouts.add(warrior_layout);

    // Archer (Fire Sprite) - 6 кадров 192x192
    let archer_texture =
        asset_server.load("sprites/Units/Red Units/Archer/Archer_Idle.png".to_string());
    let archer_layout = TextureAtlasLayout::from_grid(UVec2::new(192, 192), 6, 1, None, None);
    let archer_layout_handle = texture_atlas_layouts.add(archer_layout);

    // Monk (Slime) - 6 кадров 192x192
    let monk_texture = asset_server.load("sprites/Units/Yellow Units/Monk/Idle.png".to_string());
    let monk_layout = TextureAtlasLayout::from_grid(UVec2::new(192, 192), 6, 1, None, None);
    let monk_layout_handle = texture_atlas_layouts.add(monk_layout);

    // Lancer (Crow) - 12 кадров 320x320
    let lancer_texture =
        asset_server.load("sprites/Units/Purple Units/Lancer/Lancer_Idle.png".to_string());
    let lancer_layout = TextureAtlasLayout::from_grid(UVec2::new(320, 320), 12, 1, None, None);
    let lancer_layout_handle = texture_atlas_layouts.add(lancer_layout);

    // Фоновый градиент
    commands.spawn((
        Sprite::from_color(Color::srgb(0.15, 0.12, 0.18), Vec2::new(1920.0, 1080.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        MenuSprite,
    ));

    // Фоновый замок
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/Buildings/Purple Buildings/Castle.png"),
            color: Color::srgba(0.6, 0.5, 0.7, 0.4),
            ..default()
        },
        Transform::from_xyz(0.0, -150.0, -5.0).with_scale(Vec3::splat(1.5)),
        MenuSprite,
    ));

    // Питомцы вокруг игрока
    // Guard Dog (синий воин) - слева
    commands.spawn((
        Sprite {
            image: warrior_texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: warrior_layout_handle.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(-180.0, -120.0, 0.5).with_scale(Vec3::splat(1.0)),
        MenuSprite,
        FloatingAnimation {
            offset: 1.0,
            speed: 1.2,
            amplitude: 6.0,
        },
    ));

    // Fire Sprite (красный лучник) - справа
    commands.spawn((
        Sprite {
            image: archer_texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: archer_layout_handle.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(180.0, -120.0, 0.5).with_scale(Vec3::splat(1.0)),
        MenuSprite,
        FloatingAnimation {
            offset: 2.0,
            speed: 1.4,
            amplitude: 7.0,
        },
    ));

    // Slime Companion (желтый монах) - слева сзади
    commands.spawn((
        Sprite {
            image: monk_texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: monk_layout_handle.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(-90.0, -180.0, 0.3).with_scale(Vec3::splat(0.9)),
        MenuSprite,
        FloatingAnimation {
            offset: 0.5,
            speed: 1.0,
            amplitude: 5.0,
        },
    ));

    // Crow Scout (фиолетовый копейщик) - справа сзади
    commands.spawn((
        Sprite {
            image: lancer_texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: lancer_layout_handle.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(90.0, -180.0, 0.3).with_scale(Vec3::splat(0.7)),
        MenuSprite,
        FloatingAnimation {
            offset: 1.5,
            speed: 1.6,
            amplitude: 8.0,
        },
    ));

    // UI слой поверх всего
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(50.0)),
                ..default()
            },
            MainMenuUI,
        ))
        .with_children(|parent| {
            // Верхняя часть - название
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|top_parent| {
                    // Баннер с названием игры
                    top_parent
                        .spawn((
                            Node {
                                width: Val::Px(700.0),
                                height: Val::Px(150.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                margin: UiRect::bottom(Val::Px(20.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.1, 0.18, 0.9)),
                        ))
                        .with_children(|banner_parent| {
                            // Название игры поверх баннера
                            banner_parent.spawn((
                                Text::new("MERCHANT'S MENAGERIE"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 48.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                            ));
                        });

                    // Подзаголовок
                    top_parent.spawn((
                        Text::new("Bullet Heaven Roguelike"),
                        TextFont {
                            font: font.clone(),
                            font_size: 26.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 1.0)),
                        Node {
                            margin: UiRect::bottom(Val::Px(10.0)),
                            ..default()
                        },
                    ));
                });

            // Центральная часть - кнопки
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(20.0),
                    ..default()
                })
                .with_children(|button_container| {
                    // Кнопка "Играть"
                    button_container
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(350.0),
                                height: Val::Px(90.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.18, 0.36, 0.7)),
                            PlayButton,
                            ButtonBaseColor(Color::srgb(0.18, 0.36, 0.7)),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new("⚔ ИГРАТЬ ⚔"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 38.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            ));
                        });

                    // Кнопка "Выход"
                    button_container
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(350.0),
                                height: Val::Px(90.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.7, 0.22, 0.22)),
                            QuitButton,
                            ButtonBaseColor(Color::srgb(0.7, 0.22, 0.22)),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new("ВЫХОД"),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 34.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(1.0, 1.0, 1.0)),
                            ));
                        });
                });

            // Нижняя часть - информация
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|bottom_parent| {
                    bottom_parent.spawn((
                        Text::new("Собирай питомцев • Уничтожай врагов • Развивай торговца"),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.7, 0.7, 0.8)),
                    ));
                });
        });
}

/// Сброс камеры для корректного центрирования меню
pub fn reset_camera_for_main_menu(mut camera_query: Query<&mut Transform, With<Camera2d>>) {
    if let Ok(mut transform) = camera_query.single_mut() {
        transform.translation = Vec3::new(0.0, 0.0, 1000.0);
        transform.rotation = Quat::IDENTITY;
        transform.scale = Vec3::ONE;
    }
}

/// Система анимации покачивания спрайтов в меню
pub fn menu_floating_animation_system(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut FloatingAnimation)>,
) {
    for (mut transform, mut anim) in query.iter_mut() {
        anim.offset += time.delta_secs() * anim.speed;
        let y_offset = (anim.offset).sin() * anim.amplitude;
        transform.translation.y +=
            y_offset - ((anim.offset - time.delta_secs() * anim.speed).sin() * anim.amplitude);
    }
}

/// Система hover-эффекта для кнопок
pub fn menu_button_hover_system(
    mut button_query: Query<
        (&Interaction, &ButtonBaseColor, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(With<PlayButton>, With<QuitButton>)>,
        ),
    >,
) {
    for (interaction, base_color, mut background) in button_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                // Увеличиваем яркость при наведении
                let srgba = base_color.0.to_srgba();
                *background = BackgroundColor(Color::srgba(
                    (srgba.red * 1.2).min(1.0),
                    (srgba.green * 1.2).min(1.0),
                    (srgba.blue * 1.2).min(1.0),
                    srgba.alpha,
                ));
            }
            Interaction::Pressed => {
                // Затемняем при нажатии
                let srgba = base_color.0.to_srgba();
                *background = BackgroundColor(Color::srgba(
                    srgba.red * 0.8,
                    srgba.green * 0.8,
                    srgba.blue * 0.8,
                    srgba.alpha,
                ));
            }
            Interaction::None => {
                *background = BackgroundColor(base_color.0);
            }
        }
    }
}

/// Обработка нажатий кнопок главного меню
pub fn handle_main_menu_buttons(
    mut commands: Commands,
    play_query: Query<&Interaction, (Changed<Interaction>, With<PlayButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>, Without<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    menu_ui_query: Query<Entity, With<MainMenuUI>>,
    menu_sprite_query: Query<Entity, With<MenuSprite>>,
    mut exit: MessageWriter<AppExit>,
) {
    // Обработка кнопки "Играть"
    for interaction in play_query.iter() {
        if *interaction == Interaction::Pressed {
            // Удаляем UI главного меню
            for entity in menu_ui_query.iter() {
                commands.entity(entity).despawn();
            }
            // Удаляем спрайты меню
            for entity in menu_sprite_query.iter() {
                commands.entity(entity).despawn();
            }
            // Переходим в магазин
            next_state.set(GameState::Shop);
        }
    }

    // Обработка кнопки "Выход"
    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            exit.write(AppExit::Success);
        }
    }
}

/// Очистка UI главного меню
pub fn cleanup_main_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MainMenuUI>>,
    menu_sprite_query: Query<Entity, With<MenuSprite>>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in menu_sprite_query.iter() {
        commands.entity(entity).despawn();
    }
}
