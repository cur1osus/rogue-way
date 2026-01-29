use crate::constants::{ui_colors, ui_text, UI_FONT_SCALE};
use crate::network::{start_client, start_host, NetworkMode, DEFAULT_PORT};
use crate::resources::{TerrainConfig, TerrainSprites, UiFonts};
use crate::systems::spawn_menu_terrain;
use crate::ui::GameState;
use bevy::app::AppExit;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResized};
use rand::Rng;
use std::net::SocketAddr;

/// Маркер для UI главного меню
#[derive(Component)]
pub struct MainMenuUI;

/// Маркер для спрайтов меню (для очистки)
#[derive(Component)]
pub struct MenuSprite;

/// Маркер для фона меню
#[derive(Component)]
pub struct MenuBackground;

/// Маркер для корня террейна меню
#[derive(Component)]
pub struct MenuTerrainRoot;

/// Маркер для кнопки "Играть"
#[derive(Component)]
pub struct PlayButton;

/// Маркер для кнопки "Выход"
#[derive(Component)]
pub struct QuitButton;

/// Маркер для кнопки "Host"
#[derive(Component)]
pub struct HostButton;

/// Маркер для кнопки "Join"
#[derive(Component)]
pub struct JoinButton;

/// Маркер для окна подключения
#[derive(Component)]
pub struct JoinMenuUI;

/// Текст ввода кода подключения
#[derive(Component)]
pub struct JoinInputText;

/// Текст ошибки подключения
#[derive(Component)]
pub struct JoinErrorText;

#[derive(Resource, Default)]
pub struct JoinCodeState {
    pub active: bool,
    pub text: String,
    pub error: Option<String>,
}

#[derive(Component, Copy, Clone)]
pub struct ButtonBaseColor(pub Color);

/// Компонент для анимации покачивания
#[derive(Component)]
pub struct FloatingAnimation {
    pub offset: f32,
    pub speed: f32,
    pub amplitude: f32,
}

const MENU_BACKDROP_Z: f32 = -1.0;
const MENU_TEXT_Z: f32 = 0.2;
const MENU_TEXT_SPACING: i32 = 1;
const ROCK_BASE_SIZE: f32 = 64.0;

const LETTER_R: [&str; 7] = [
    "####.", "#...#", "#...#", "####.", "#.#..", "#..#.", "#...#",
];
const LETTER_O: [&str; 7] = [
    ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.",
];
const LETTER_G: [&str; 7] = [
    ".###.", "#...#", "#....", "#.###", "#...#", "#...#", ".###.",
];
const LETTER_Y: [&str; 7] = [
    "#...#", "#...#", ".#.#.", "..#..", "..#..", "..#..", "..#..",
];
const ROGGY_LETTERS: [&[&str; 7]; 5] = [&LETTER_R, &LETTER_O, &LETTER_G, &LETTER_G, &LETTER_Y];

/// Настройка UI главного меню
pub fn setup_main_menu(
    mut commands: Commands,
    ui_fonts: Res<UiFonts>,
    asset_server: Res<AssetServer>,
    windows: Query<&Window, With<PrimaryWindow>>,
    terrain_sprites: Res<TerrainSprites>,
    terrain_config: Res<TerrainConfig>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let font = ui_fonts.main.clone();
    let window_size = match windows.single() {
        Ok(window) => window.size(),
        Err(_) => Vec2::new(1280.0, 720.0),
    };

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

    // Базовый фон
    commands.spawn((
        Sprite::from_color(ui_colors::MENU_BACKGROUND, window_size),
        Transform::from_xyz(0.0, 0.0, MENU_BACKDROP_Z),
        MenuBackground,
        MenuSprite,
    ));

    spawn_menu_terrain_root(
        &mut commands,
        window_size,
        &terrain_sprites,
        &terrain_config,
    );

    // Фоновый замок
    commands.spawn((
        Sprite {
            image: asset_server.load("sprites/Buildings/Purple Buildings/Castle.png"),
            color: ui_colors::MENU_ACCENT,
            ..default()
        },
        Transform::from_xyz(0.0, -150.0, 0.12).with_scale(Vec3::splat(1.5)),
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
                            BackgroundColor(ui_colors::PANEL_PURPLE),
                        ))
                        .with_children(|banner_parent| {
                            // Название игры поверх баннера
                            banner_parent.spawn((
                                Text::new(ui_text::GAME_TITLE),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 48.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_GOLD),
                            ));
                        });

                    // Подзаголовок
                    top_parent.spawn((
                        Text::new(ui_text::GAME_SUBTITLE),
                        TextFont {
                            font: font.clone(),
                            font_size: 26.0 * UI_FONT_SCALE,
                            ..default()
                        },
                        TextColor(ui_colors::TEXT_PURPLE_LIGHT),
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
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    bottom: Val::Px(180.0),
                    left: Val::Px(0.0),
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
                            BackgroundColor(ui_colors::BUTTON_BLUE),
                            PlayButton,
                            ButtonBaseColor(ui_colors::BUTTON_BLUE),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new(ui_text::BTN_PLAY),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 38.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_WHITE),
                            ));
                        });

                    // Кнопка "Host"
                    button_container
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(320.0),
                                height: Val::Px(70.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(ui_colors::BUTTON_GREEN),
                            HostButton,
                            ButtonBaseColor(ui_colors::BUTTON_GREEN),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new(ui_text::BTN_HOST),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 28.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_WHITE),
                            ));
                        });

                    // Кнопка "Join"
                    button_container
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(320.0),
                                height: Val::Px(70.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(ui_colors::BUTTON_BLUE),
                            JoinButton,
                            ButtonBaseColor(ui_colors::BUTTON_BLUE),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new(ui_text::BTN_JOIN),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 26.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_WHITE),
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
                            BackgroundColor(ui_colors::BUTTON_RED),
                            QuitButton,
                            ButtonBaseColor(ui_colors::BUTTON_RED),
                        ))
                        .with_children(|button_parent| {
                            button_parent.spawn((
                                Text::new(ui_text::BTN_EXIT),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 34.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_WHITE),
                            ));
                        });
                });

            // Окно подключения (скрыто по умолчанию)
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        display: Display::None,
                        ..default()
                    },
                    BackgroundColor(ui_colors::OVERLAY_DARK),
                    JoinMenuUI,
                ))
                .with_children(|overlay| {
                    overlay
                        .spawn((
                            Node {
                                width: Val::Px(520.0),
                                height: Val::Px(260.0),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                row_gap: Val::Px(12.0),
                                padding: UiRect::all(Val::Px(20.0)),
                                ..default()
                            },
                            BackgroundColor(ui_colors::PANEL_DARK),
                        ))
                        .with_children(|panel| {
                            panel.spawn((
                                Text::new(ui_text::JOIN_TITLE),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 26.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_YELLOW_LIGHT),
                            ));
                            panel.spawn((
                                Text::new(ui_text::JOIN_HINT),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 16.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_GRAY_LIGHT),
                            ));
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 20.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_WHITE),
                                JoinInputText,
                            ));
                            panel.spawn((
                                Text::new(""),
                                TextFont {
                                    font: font.clone(),
                                    font_size: 14.0 * UI_FONT_SCALE,
                                    ..default()
                                },
                                TextColor(ui_colors::TEXT_RED_BRIGHT),
                                JoinErrorText,
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
                        Text::new(ui_text::GAME_DESCRIPTION),
                        TextFont {
                            font: font.clone(),
                            font_size: 18.0 * UI_FONT_SCALE,
                            ..default()
                        },
                        TextColor(ui_colors::TEXT_GRAY),
                    ));
                });
        });
}

fn spawn_menu_terrain_root(
    commands: &mut Commands,
    window_size: Vec2,
    terrain_sprites: &TerrainSprites,
    terrain_config: &TerrainConfig,
) -> Entity {
    let terrain_root = commands
        .spawn((
            Transform::default(),
            GlobalTransform::default(),
            MenuTerrainRoot,
            MenuSprite,
        ))
        .id();
    let mut menu_terrain_config = terrain_config.clone();
    menu_terrain_config.seed = rand::thread_rng().gen();
    commands.entity(terrain_root).with_children(|parent| {
        spawn_menu_terrain(parent, window_size, terrain_sprites, &menu_terrain_config);
        spawn_roggy_rock_text(parent, Vec2::ZERO, window_size, terrain_sprites);
    });
    terrain_root
}

pub fn menu_resize_system(
    mut commands: Commands,
    mut resize_events: MessageReader<WindowResized>,
    windows: Query<&Window, With<PrimaryWindow>>,
    terrain_sprites: Res<TerrainSprites>,
    terrain_config: Res<TerrainConfig>,
    mut background_query: Query<&mut Sprite, With<MenuBackground>>,
    terrain_root_query: Query<Entity, With<MenuTerrainRoot>>,
) {
    if resize_events.is_empty() {
        return;
    }
    resize_events.clear();

    let Ok(window) = windows.single() else {
        return;
    };
    let window_size = window.size();

    for mut sprite in background_query.iter_mut() {
        sprite.custom_size = Some(window_size);
    }

    for entity in terrain_root_query.iter() {
        commands.entity(entity).despawn_children();
        commands.entity(entity).despawn();
    }
    spawn_menu_terrain_root(
        &mut commands,
        window_size,
        &terrain_sprites,
        &terrain_config,
    );
}

fn spawn_roggy_rock_text(
    parent: &mut ChildSpawnerCommands,
    center: Vec2,
    window_size: Vec2,
    terrain_sprites: &TerrainSprites,
) {
    if terrain_sprites.rocks.is_empty() {
        return;
    }

    let mut rng = rand::thread_rng();
    let letter_width = ROGGY_LETTERS[0][0].len() as i32;
    let letter_height = ROGGY_LETTERS[0].len() as i32;
    let letter_count = ROGGY_LETTERS.len() as i32;
    let total_columns = letter_width * letter_count + (letter_count - 1) * MENU_TEXT_SPACING;
    let total_rows = letter_height;
    let max_width = window_size.x * 0.8;
    let max_height = window_size.y * 0.3;
    let cell_size = (max_width / total_columns as f32)
        .min(max_height / total_rows as f32)
        .clamp(28.0, 64.0);
    let total_width = total_columns as f32 * cell_size;
    let total_height = total_rows as f32 * cell_size;
    let rock_scale = (cell_size / ROCK_BASE_SIZE) * 1.15;
    let text_tint = Color::srgb(0.92, 0.95, 1.0);

    for (letter_index, letter) in ROGGY_LETTERS.iter().enumerate() {
        for (row_index, row) in letter.iter().enumerate() {
            for (col_index, glyph) in row.chars().enumerate() {
                if glyph != '#' {
                    continue;
                }

                let grid_x =
                    letter_index as i32 * (letter_width + MENU_TEXT_SPACING) + col_index as i32;
                let grid_y = row_index as i32;
                let x = center.x - total_width / 2.0 + (grid_x as f32 + 0.5) * cell_size;
                let y = center.y + total_height / 2.0 - (grid_y as f32 + 0.5) * cell_size;
                let rock_index = rng.gen_range(0..terrain_sprites.rocks.len());
                parent.spawn((
                    Sprite {
                        image: terrain_sprites.rocks[rock_index].clone(),
                        color: text_tint,
                        ..default()
                    },
                    Transform::from_xyz(x, y, MENU_TEXT_Z).with_scale(Vec3::splat(rock_scale)),
                ));
            }
        }
    }
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
    host_query: Query<&Interaction, (Changed<Interaction>, With<HostButton>)>,
    join_query: Query<&Interaction, (Changed<Interaction>, With<JoinButton>)>,
    quit_query: Query<&Interaction, (Changed<Interaction>, With<QuitButton>, Without<PlayButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    menu_ui_query: Query<Entity, With<MainMenuUI>>,
    menu_sprite_query: Query<Entity, With<MenuSprite>>,
    mut exit: MessageWriter<AppExit>,
    mut join_state: ResMut<JoinCodeState>,
    mut network_mode: ResMut<NetworkMode>,
) {
    // Обработка кнопки "Играть"
    for interaction in play_query.iter() {
        if *interaction == Interaction::Pressed {
            *network_mode = NetworkMode::Offline;
            commands.remove_resource::<crate::network::NetworkServer>();
            commands.remove_resource::<crate::network::NetworkClient>();
            join_state.active = false;
            join_state.error = None;
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

    // Обработка кнопки "Host"
    for interaction in host_query.iter() {
        if *interaction == Interaction::Pressed {
            join_state.active = false;
            join_state.error = None;
            match start_host(&mut commands, DEFAULT_PORT) {
                Ok(join_code) => {
                    println!("Host started. Join code: {join_code}");
                    for entity in menu_ui_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    for entity in menu_sprite_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    next_state.set(GameState::Playing);
                }
                Err(err) => {
                    eprintln!("Host start failed: {err}");
                }
            }
        }
    }

    // Обработка кнопки "Join"
    for interaction in join_query.iter() {
        if *interaction == Interaction::Pressed {
            join_state.active = true;
            join_state.error = None;
        }
    }

    // Обработка кнопки "Выход"
    for interaction in quit_query.iter() {
        if *interaction == Interaction::Pressed {
            exit.write(AppExit::Success);
        }
    }
}

pub fn join_menu_visibility_system(
    join_state: Res<JoinCodeState>,
    mut menu_query: Query<&mut Node, With<JoinMenuUI>>,
    mut input_query: Query<&mut Text, (With<JoinInputText>, Without<JoinErrorText>)>,
    mut error_query: Query<&mut Text, (With<JoinErrorText>, Without<JoinInputText>)>,
) {
    if !join_state.is_changed() {
        return;
    }

    for mut node in menu_query.iter_mut() {
        node.display = if join_state.active {
            Display::Flex
        } else {
            Display::None
        };
    }

    let display_text = if join_state.text.is_empty() {
        "..."
    } else {
        join_state.text.as_str()
    };

    for mut text in input_query.iter_mut() {
        text.0 = display_text.to_string();
    }

    let error_text = join_state.error.as_deref().unwrap_or("");
    for mut text in error_query.iter_mut() {
        text.0 = error_text.to_string();
    }
}

pub fn join_menu_input_system(
    mut commands: Commands,
    mut join_state: ResMut<JoinCodeState>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    menu_ui_query: Query<Entity, With<MainMenuUI>>,
    menu_sprite_query: Query<Entity, With<MenuSprite>>,
) {
    if !join_state.active {
        return;
    }

    let mut new_char: Option<char> = None;
    for (key, ch) in [
        (KeyCode::Digit0, '0'),
        (KeyCode::Digit1, '1'),
        (KeyCode::Digit2, '2'),
        (KeyCode::Digit3, '3'),
        (KeyCode::Digit4, '4'),
        (KeyCode::Digit5, '5'),
        (KeyCode::Digit6, '6'),
        (KeyCode::Digit7, '7'),
        (KeyCode::Digit8, '8'),
        (KeyCode::Digit9, '9'),
        (KeyCode::Numpad0, '0'),
        (KeyCode::Numpad1, '1'),
        (KeyCode::Numpad2, '2'),
        (KeyCode::Numpad3, '3'),
        (KeyCode::Numpad4, '4'),
        (KeyCode::Numpad5, '5'),
        (KeyCode::Numpad6, '6'),
        (KeyCode::Numpad7, '7'),
        (KeyCode::Numpad8, '8'),
        (KeyCode::Numpad9, '9'),
        (KeyCode::Period, '.'),
        (KeyCode::NumpadDecimal, '.'),
        (KeyCode::Semicolon, ':'),
    ] {
        if keyboard_input.just_pressed(key) {
            new_char = Some(ch);
            break;
        }
    }

    if let Some(ch) = new_char {
        if join_state.text.len() < 64 {
            join_state.text.push(ch);
        }
    }

    if keyboard_input.just_pressed(KeyCode::Backspace) {
        join_state.text.pop();
    }

    if keyboard_input.just_pressed(KeyCode::Escape) {
        join_state.active = false;
        join_state.error = None;
        join_state.text.clear();
        return;
    }

    if keyboard_input.just_pressed(KeyCode::Enter) {
        let input = join_state.text.trim();
        if input.is_empty() {
            join_state.error = Some("Код пустой".to_string());
            return;
        }
        match input.parse::<SocketAddr>() {
            Ok(addr) => match start_client(&mut commands, addr) {
                Ok(()) => {
                    for entity in menu_ui_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    for entity in menu_sprite_query.iter() {
                        commands.entity(entity).despawn();
                    }
                    join_state.active = false;
                    join_state.error = None;
                    join_state.text.clear();
                    next_state.set(GameState::Playing);
                }
                Err(err) => {
                    join_state.error = Some(err);
                }
            },
            Err(_) => {
                join_state.error = Some("Неверный формат. Пример: 192.168.0.10:14000".to_string());
            }
        }
    }
}

/// Очистка UI главного меню
pub fn cleanup_main_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MainMenuUI>>,
    menu_sprite_query: Query<Entity, With<MenuSprite>>,
    mut join_state: ResMut<JoinCodeState>,
) {
    for entity in menu_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in menu_sprite_query.iter() {
        commands.entity(entity).despawn();
    }
    join_state.active = false;
    join_state.error = None;
}
