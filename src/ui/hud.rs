use crate::components::{
    AttackRange, Boss, Enemy, EnemyHpIndicator, EnemyHpSegment, EnemyHpText, Experience, Gold,
    Health, Hitbox, LocalPlayer, MovementSpeed, Pet, PickupRadius, Player, Team,
};
use crate::constants::{
    ui_colors, ui_text, ENEMY_HP_INDICATOR_MIN_RADIUS, ENEMY_HP_INDICATOR_MIN_SEGMENT_RADIUS,
    ENEMY_HP_INDICATOR_OFFSET, ENEMY_HP_INDICATOR_RADIUS_MULT, ENEMY_HP_INDICATOR_SEGMENT_SCALE,
    ENEMY_HP_SEGMENTS, UI_FONT_SCALE,
};
use crate::network::NetworkServer;
use crate::resources::{PlayerDamageFlash, UiFonts, UpgradeState, WaveConfig};
use bevy::math::primitives::{Circle, Rectangle};
use bevy::prelude::*;
use bevy::ui::prelude::BorderColor;
use std::collections::HashSet;
use std::f32::consts::{FRAC_PI_2, TAU};
use sysinfo::{ProcessRefreshKind, System};

/// Общий маркер для всех элементов HUD
#[derive(Component)]
pub struct HudUI;

/// Маркер для мигания урона игрока
#[derive(Component)]
pub struct PlayerDamageFlashOverlay;

/// Маркер для HP текста
#[derive(Component)]
pub struct HpText;

/// Маркер для XP текста
#[derive(Component)]
pub struct XpText;

/// Маркер для таймера
#[derive(Component)]
pub struct TimerText;

/// Маркер для счетчика золота
#[derive(Component)]
pub struct GoldText;

/// Маркер для текста кода подключения
#[derive(Component)]
pub struct NetworkJoinCodeText;

/// Маркер для полоски здоровья босса
#[derive(Component)]
pub struct BossHealthBar;

/// Маркер для текста имени босса
#[derive(Component)]
pub struct BossNameText;

/// Маркер для панели статов
#[derive(Component)]
pub struct StatsPanel;

#[derive(Component)]
pub struct DevPanel;

/// Маркеры для текста в панели статов
#[derive(Component)]
pub struct StatsHealthText;

#[derive(Component)]
pub struct StatsLevelText;

#[derive(Component)]
pub struct StatsExperienceText;

#[derive(Component)]
pub struct StatsGoldText;

#[derive(Component)]
pub struct StatsSpeedText;

#[derive(Component)]
pub struct StatsUpgradesText;

#[derive(Component)]
pub struct StatsFpsText;

#[derive(Component)]
pub struct StatsCpuText;

#[derive(Component)]
pub struct StatsCpuCoresText;

#[derive(Component)]
pub struct StatsMemoryText;

/// Ресурс для управления видимостью панели статов
#[derive(Resource, Default)]
pub struct StatsPanelVisible(pub bool);

#[derive(Resource, Default)]
pub struct DevPanelVisible(pub bool);

#[derive(Resource, Default)]
pub struct HitboxVisualsVisible(pub bool);

#[derive(Resource)]
pub struct PerformanceStats {
    pub fps: f32,
    pub memory_mb: f32,
    pub cpu_percent: f32,
    pub cpu_cores_used: f32,
    pub cpu_cores_total: usize,
    frame_timer: Timer,
    frame_count: u32,
    system: System,
    pid: Option<sysinfo::Pid>,
}

impl Default for PerformanceStats {
    fn default() -> Self {
        let mut system = System::new();
        let pid = sysinfo::get_current_pid().ok();
        if let Some(pid) = pid {
            system.refresh_pids(&[pid]);
        } else {
            system.refresh_processes();
        }
        Self {
            fps: 0.0,
            memory_mb: 0.0,
            cpu_percent: 0.0,
            cpu_cores_used: 0.0,
            cpu_cores_total: 0,
            frame_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            frame_count: 0,
            system,
            pid,
        }
    }
}

#[derive(Component)]
pub struct AttackRangeVisual {
    pub owner: Entity,
}

#[derive(Component)]
pub struct HitboxVisual {
    pub owner: Entity,
}

#[derive(Component)]
pub struct AreaDamageVisual;

#[derive(Resource)]
pub struct AttackRangeVisualAssets {
    pub mesh: Handle<Mesh>,
    pub friendly_material: Handle<ColorMaterial>,
    pub enemy_material: Handle<ColorMaterial>,
    pub neutral_material: Handle<ColorMaterial>,
}

#[derive(Resource)]
pub struct HitboxVisualAssets {
    pub box_mesh: Handle<Mesh>,
    pub circle_mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

#[derive(Resource)]
pub struct AreaDamageVisualAssets {
    pub material: Handle<ColorMaterial>,
}

#[derive(Resource)]
pub struct EnemyHpIndicatorAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

impl FromWorld for AttackRangeVisualAssets {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

            let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
            let friendly_material = materials.add(ui_colors::ZONE_FRIENDLY);
            let enemy_material = materials.add(ui_colors::ZONE_ENEMY);
            let neutral_material = materials.add(ui_colors::ZONE_NEUTRAL);

            Self {
                mesh,
                friendly_material,
                enemy_material,
                neutral_material,
            }
        })
    }
}

impl FromWorld for HitboxVisualAssets {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

            let box_mesh = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
            let circle_mesh = meshes.add(Mesh::from(Circle::new(1.0)));
            let material = materials.add(ui_colors::HITBOX_COLOR);

            Self {
                box_mesh,
                circle_mesh,
                material,
            }
        })
    }
}

impl FromWorld for AreaDamageVisualAssets {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|_world, mut materials: Mut<Assets<ColorMaterial>>| {
            let material = materials.add(ui_colors::ZONE_SPLASH);

            Self { material }
        })
    }
}

impl FromWorld for EnemyHpIndicatorAssets {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
            let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
            let material = materials.add(ui_colors::HP_BAR);

            Self { mesh, material }
        })
    }
}

/// Настройка HUD (запускается один раз при старте)
pub fn setup_hud(
    mut commands: Commands,
    ui_fonts: Res<UiFonts>,
    mut player_damage_flash: ResMut<PlayerDamageFlash>,
    existing_hud: Query<Entity, With<HudUI>>,
    network_server: Option<Res<NetworkServer>>,
) {
    // Если HUD уже существует, не создаем новый (возврат из LevelUpChoice)
    if !existing_hud.is_empty() {
        return;
    }

    *player_damage_flash = PlayerDamageFlash::default();

    let font = ui_fonts.main.clone();
    // HP текст (сверху слева)
    commands.spawn((
        HudUI,
        Text::new(ui_text::format_health(100.0, 100.0)),
        TextFont {
            font: font.clone(),
            font_size: 24.0 * UI_FONT_SCALE,
            ..default()
        },
        TextColor(ui_colors::TEXT_RED),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        HpText,
    ));

    // Таймер (сверху по центру)
    commands.spawn((
        HudUI,
        Text::new(ui_text::format_time(0, 0)),
        TextFont {
            font: font.clone(),
            font_size: 28.0 * UI_FONT_SCALE,
            ..default()
        },
        TextColor(ui_colors::TEXT_WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(45.0),
            top: Val::Px(10.0),
            ..default()
        },
        TimerText,
    ));

    // XP текст (внизу по центру)
    commands.spawn((
        HudUI,
        Text::new(ui_text::format_level_xp(1, 0, 100)),
        TextFont {
            font: font.clone(),
            font_size: 20.0 * UI_FONT_SCALE,
            ..default()
        },
        TextColor(ui_colors::TEXT_CYAN),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(40.0),
            bottom: Val::Px(10.0),
            ..default()
        },
        XpText,
    ));

    // Золото (сверху справа)
    commands.spawn((
        HudUI,
        Text::new(ui_text::format_gold(0)),
        TextFont {
            font: font.clone(),
            font_size: 24.0 * UI_FONT_SCALE,
            ..default()
        },
        TextColor(ui_colors::TEXT_GOLD),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        GoldText,
    ));

    // Контур мигания при получении урона
    commands.spawn((
        HudUI,
        PlayerDamageFlashOverlay,
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            border: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BorderColor::all(Color::NONE),
        BackgroundColor(Color::NONE),
    ));

    if let Some(server) = network_server {
        commands.spawn((
            HudUI,
            Text::new(format!("Код: {}", server.join_code)),
            TextFont {
                font: font.clone(),
                font_size: 16.0 * UI_FONT_SCALE,
                ..default()
            },
            TextColor(ui_colors::TEXT_GRAY_LIGHT),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                ..default()
            },
            NetworkJoinCodeText,
        ));
    }
}

/// Система обновления HUD
pub fn ui_update_system(
    player_query: Query<(&Health, &Experience, &Gold), (With<Player>, With<LocalPlayer>)>,
    wave_config: Res<WaveConfig>,
    mut hp_text_query: Query<
        &mut Text,
        (
            With<HpText>,
            Without<XpText>,
            Without<TimerText>,
            Without<GoldText>,
        ),
    >,
    mut xp_text_query: Query<
        &mut Text,
        (
            With<XpText>,
            Without<HpText>,
            Without<TimerText>,
            Without<GoldText>,
        ),
    >,
    mut timer_text_query: Query<
        &mut Text,
        (
            With<TimerText>,
            Without<HpText>,
            Without<XpText>,
            Without<GoldText>,
        ),
    >,
    mut gold_text_query: Query<
        &mut Text,
        (
            With<GoldText>,
            Without<HpText>,
            Without<XpText>,
            Without<TimerText>,
        ),
    >,
) {
    // Обновляем HP, XP и золото
    if let Ok((health, experience, gold)) = player_query.single() {
        if let Ok(mut text) = hp_text_query.single_mut() {
            text.0 = ui_text::format_hud_health(health.current, health.max);
        }

        // Обновляем XP
        if let Ok(mut text) = xp_text_query.single_mut() {
            text.0 = ui_text::format_hud_level_xp(
                experience.level,
                experience.current,
                experience.to_next_level,
            );
        }

        // Обновляем золото
        if let Ok(mut text) = gold_text_query.single_mut() {
            text.0 = ui_text::format_hud_gold(gold.amount);
        }
    }

    // Обновляем таймер
    if let Ok(mut text) = timer_text_query.single_mut() {
        let total_seconds = wave_config.game_time as u32;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        text.0 = ui_text::format_hud_time(minutes, seconds);
    }
}

pub fn player_damage_flash_system(
    time: Res<Time>,
    mut flash: ResMut<PlayerDamageFlash>,
    mut overlay_query: Query<&mut BorderColor, With<PlayerDamageFlashOverlay>>,
) {
    if flash.timer.duration().as_secs_f32() <= 0.0 || flash.intensity <= 0.0 {
        if let Ok(mut border) = overlay_query.single_mut() {
            *border = BorderColor::all(Color::NONE);
        }
        return;
    }

    flash.timer.tick(time.delta());

    let Ok(mut border) = overlay_query.single_mut() else {
        if flash.timer.is_finished() {
            flash.intensity = 0.0;
        }
        return;
    };

    if flash.timer.is_finished() {
        flash.intensity = 0.0;
        *border = BorderColor::all(Color::NONE);
        return;
    }

    let progress = (flash.timer.elapsed_secs() / flash.timer.duration().as_secs_f32()).min(1.0);
    let pulse = (progress * std::f32::consts::TAU * 2.0).sin().abs();
    let fade = 1.0 - progress;
    let alpha = (0.15 + 0.85 * pulse) * fade * flash.intensity;

    *border = BorderColor::all(Color::srgba(1.0, 0.2, 0.2, alpha));
}

/// Система отображения полоски здоровья босса
pub fn boss_health_bar_system(
    mut commands: Commands,
    boss_query: Query<(&Health, &Boss), Added<Boss>>,
    existing_bar_query: Query<Entity, With<BossHealthBar>>,
    ui_fonts: Res<UiFonts>,
) {
    let font = ui_fonts.main.clone();
    // Удаляем старую полоску здоровья
    for entity in existing_bar_query.iter() {
        commands.entity(entity).despawn();
    }

    // Создаем новую полоску здоровья, если есть босс
    if let Ok((_health, boss)) = boss_query.single() {
        // Контейнер для полоски здоровья босса
        commands
            .spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(60.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    position_type: PositionType::Absolute,
                    top: Val::Px(50.0),
                    ..default()
                },
                BossHealthBar,
            ))
            .with_children(|parent| {
                // Имя босса
                parent.spawn((
                    Text::new(boss.boss_type.get_name()),
                    TextFont {
                        font: font.clone(),
                        font_size: 24.0 * UI_FONT_SCALE,
                        ..default()
                    },
                    TextColor(ui_colors::TEXT_RED_DARK),
                    BossNameText,
                ));

                // Полоска здоровья
                parent
                    .spawn(Node {
                        width: Val::Px(600.0),
                        height: Val::Px(20.0),
                        margin: UiRect::top(Val::Px(5.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    })
                    .with_children(|health_bar_parent| {
                        // Заполнение здоровья
                        health_bar_parent.spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(ui_colors::HP_BAR),
                        ));
                    });
            });
    }
}

/// Система обновления полоски здоровья босса
pub fn update_boss_health_bar_system(
    boss_query: Query<&Health, With<Boss>>,
    mut health_bar_query: Query<&mut Node, With<BossHealthBar>>,
) {
    if let Ok(health) = boss_query.single() {
        for mut node in health_bar_query.iter_mut() {
            let health_percent = (health.current / health.max).max(0.0).min(1.0);
            node.width = Val::Percent(health_percent * 100.0);
        }
    }
}

/// Система обработки нажатия клавиши T для показа/скрытия статов
pub fn toggle_stats_panel_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut stats_visible: ResMut<StatsPanelVisible>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        stats_visible.0 = !stats_visible.0;
    }
}

/// Система обработки нажатия клавиши P для показа/скрытия панели разработчика
pub fn toggle_dev_panel_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut dev_visible: ResMut<DevPanelVisible>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        dev_visible.0 = !dev_visible.0;
    }
}

pub fn toggle_hitbox_visuals_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut hitbox_visible: ResMut<HitboxVisualsVisible>,
) {
    if keyboard_input.just_pressed(KeyCode::KeyH) {
        hitbox_visible.0 = !hitbox_visible.0;
    }
}

pub fn performance_stats_system(time: Res<Time>, mut perf: ResMut<PerformanceStats>) {
    perf.frame_count += 1;
    perf.frame_timer.tick(time.delta());

    if perf.frame_timer.just_finished() {
        let seconds = perf.frame_timer.duration().as_secs_f32().max(0.0001);
        perf.fps = perf.frame_count as f32 / seconds;
        perf.frame_count = 0;

        if let Some(pid) = perf.pid {
            perf.system
                .refresh_pids_specifics(&[pid], ProcessRefreshKind::new().with_cpu().with_memory());
            let cores_total = perf.system.cpus().len();
            let (memory_mb, cpu_percent, cpu_cores_used) =
                if let Some(process) = perf.system.process(pid) {
                    let memory_mb = process.memory() as f32 / (1024.0 * 1024.0);
                    let cpu_percent_total = process.cpu_usage();
                    let cores_used = (cpu_percent_total / 100.0).max(0.0);
                    let cores_total_f = cores_total as f32;
                    let cpu_percent = if cores_total_f > 0.0 {
                        cpu_percent_total / cores_total_f
                    } else {
                        0.0
                    };
                    let cpu_cores_used = if cores_total_f > 0.0 {
                        cores_used.min(cores_total_f)
                    } else {
                        cores_used
                    };
                    (memory_mb, cpu_percent, cpu_cores_used)
                } else {
                    (0.0, 0.0, 0.0)
                };
            perf.memory_mb = memory_mb;
            perf.cpu_percent = cpu_percent;
            perf.cpu_cores_used = cpu_cores_used;
            perf.cpu_cores_total = cores_total;
        } else {
            perf.system.refresh_processes();
            perf.cpu_percent = 0.0;
            perf.cpu_cores_used = 0.0;
            perf.cpu_cores_total = 0;
        }
    }
}

/// Система создания/обновления панели статов
pub fn stats_panel_system(
    mut commands: Commands,
    stats_visible: Res<StatsPanelVisible>,
    mut panel_query: Query<&mut Node, With<StatsPanel>>,
    player_query: Query<
        (&Health, &MovementSpeed, &Experience, &Gold),
        (With<Player>, With<LocalPlayer>),
    >,
    pet_query: Query<&Pet>,
    upgrade_state: Res<UpgradeState>,
    perf_stats: Res<PerformanceStats>,
    ui_fonts: Res<UiFonts>,
) {
    let mut has_panel = false;
    for mut node in panel_query.iter_mut() {
        has_panel = true;
        node.display = if stats_visible.0 {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !stats_visible.0 {
        return;
    }

    // Если панель уже существует, не создаём новую
    if has_panel {
        return;
    }

    // Получаем статы игрока
    let Ok((health, speed, experience, gold)) = player_query.single() else {
        return;
    };

    let pet_count = pet_query.iter().count();
    let font = ui_fonts.main.clone();

    // Создаём панель статов
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(15.0)),
                row_gap: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(ui_colors::OVERLAY_DARKER),
            StatsPanel,
            HudUI,
        ))
        .with_children(|parent| {
            // Заголовок
            parent.spawn((
                Text::new(ui_text::STATS_TITLE),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_YELLOW_LIGHT),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            // Разделитель
            parent.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(2.0),
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(ui_colors::SEPARATOR),
            ));

            // Статы игрока
            parent.spawn((
                Text::new(ui_text::format_health(health.current, health.max)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_RED_BRIGHT),
                StatsHealthText,
            ));

            parent.spawn((
                Text::new(ui_text::format_level(experience.level)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN),
                StatsLevelText,
            ));

            parent.spawn((
                Text::new(ui_text::format_experience(
                    experience.current,
                    experience.to_next_level,
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN),
                StatsExperienceText,
            ));

            parent.spawn((
                Text::new(ui_text::format_gold(gold.amount)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GOLD),
                StatsGoldText,
            ));

            parent.spawn((
                Text::new(ui_text::format_speed(speed.0)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GREEN),
                StatsSpeedText,
            ));

            parent.spawn((
                Text::new(ui_text::format_fps(perf_stats.fps)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN_LIGHT),
                StatsFpsText,
            ));

            parent.spawn((
                Text::new(ui_text::format_cpu_usage(perf_stats.cpu_percent)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN_LIGHT),
                StatsCpuText,
            ));

            parent.spawn((
                Text::new(ui_text::format_cpu_cores_used(
                    perf_stats.cpu_cores_used,
                    perf_stats.cpu_cores_total,
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN_LIGHT),
                StatsCpuCoresText,
            ));

            parent.spawn((
                Text::new(ui_text::format_memory(perf_stats.memory_mb)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_CYAN_LIGHT),
                StatsMemoryText,
            ));

            // Разделитель
            parent.spawn((
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(2.0),
                    margin: UiRect::vertical(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(ui_colors::SEPARATOR),
            ));

            // Статы улучшений
            parent.spawn((
                Text::new(ui_text::UPGRADES_TITLE),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_ORANGE),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(build_upgrades_text(pet_count, &upgrade_state)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GRAY_LIGHT),
                StatsUpgradesText,
            ));

            // Подсказка
            parent.spawn((
                Text::new(ui_text::STATS_HIDE_HINT),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GRAY_DARK),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
            ));
        });
}

/// Система создания/обновления панели разработчика
pub fn dev_panel_system(
    mut commands: Commands,
    dev_visible: Res<DevPanelVisible>,
    mut panel_query: Query<&mut Node, With<DevPanel>>,
    ui_fonts: Res<UiFonts>,
) {
    let mut has_panel = false;
    for mut node in panel_query.iter_mut() {
        has_panel = true;
        node.display = if dev_visible.0 {
            Display::Flex
        } else {
            Display::None
        };
    }

    if !dev_visible.0 {
        return;
    }

    if has_panel {
        return;
    }

    let font = ui_fonts.main.clone();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(60.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(15.0)),
                row_gap: Val::Px(5.0),
                ..default()
            },
            BackgroundColor(ui_colors::OVERLAY_DARKER),
            DevPanel,
            HudUI,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(ui_text::DEV_PANEL_TITLE),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_PURPLE_LIGHT),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(
                    "1: XP гем +25\n2: XP гем +100\n3: Монета золота +50\n4: +500 золота\n5: Полное лечение\n6: +100 XP (сразу)\n7: Призвать пса\n8: Призвать огненную фею\n9: Призвать слизня\n0: Призвать ворона",
                ),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GRAY_LIGHT),
            ));

            parent.spawn((
                Text::new(ui_text::DEV_PANEL_HIDE_HINT),
                TextFont {
                    font,
                    font_size: 14.0 * UI_FONT_SCALE,
                    ..default()
                },
                TextColor(ui_colors::TEXT_GRAY),
                Node {
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
            ));
        });
}

/// Система обновления текста панели статов в реальном времени
pub fn update_stats_panel_system(
    stats_visible: Res<StatsPanelVisible>,
    player_query: Query<
        (&Health, &MovementSpeed, &Experience, &Gold),
        (With<Player>, With<LocalPlayer>),
    >,
    pet_query: Query<&Pet>,
    upgrade_state: Res<UpgradeState>,
    perf_stats: Res<PerformanceStats>,
    mut stats_text_query: Query<(
        &mut Text,
        Option<&StatsHealthText>,
        Option<&StatsLevelText>,
        Option<&StatsExperienceText>,
        Option<&StatsGoldText>,
        Option<&StatsSpeedText>,
        Option<&StatsUpgradesText>,
        Option<&StatsFpsText>,
        Option<&StatsCpuText>,
        Option<&StatsCpuCoresText>,
        Option<&StatsMemoryText>,
    )>,
) {
    if !stats_visible.0 {
        return;
    }

    let Ok((health, speed, experience, gold)) = player_query.single() else {
        return;
    };

    let pet_count = pet_query.iter().count();
    let upgrades_text = build_upgrades_text(pet_count, &upgrade_state);

    for (
        mut text,
        is_health,
        is_level,
        is_xp,
        is_gold,
        is_speed,
        is_upgrades,
        is_fps,
        is_cpu,
        is_cpu_cores,
        is_memory,
    ) in stats_text_query.iter_mut()
    {
        if is_health.is_some() {
            text.0 = ui_text::format_hud_health(health.current, health.max);
        } else if is_level.is_some() {
            text.0 = ui_text::format_level(experience.level);
        } else if is_xp.is_some() {
            text.0 = ui_text::format_experience(experience.current, experience.to_next_level);
        } else if is_gold.is_some() {
            text.0 = ui_text::format_hud_gold(gold.amount);
        } else if is_speed.is_some() {
            text.0 = ui_text::format_speed(speed.0);
        } else if is_upgrades.is_some() {
            text.0 = upgrades_text.clone();
        } else if is_fps.is_some() {
            text.0 = ui_text::format_fps(perf_stats.fps);
        } else if is_cpu.is_some() {
            text.0 = ui_text::format_cpu_usage(perf_stats.cpu_percent);
        } else if is_cpu_cores.is_some() {
            text.0 = ui_text::format_cpu_cores_used(
                perf_stats.cpu_cores_used,
                perf_stats.cpu_cores_total,
            );
        } else if is_memory.is_some() {
            text.0 = ui_text::format_memory(perf_stats.memory_mb);
        }
    }
}

pub fn spawn_attack_range_visuals_system(
    mut commands: Commands,
    visuals_visible: Res<HitboxVisualsVisible>,
    visuals: Res<AttackRangeVisualAssets>,
    range_query: Query<(Entity, &Transform, &AttackRange, Option<&Team>)>,
    visual_query: Query<&AttackRangeVisual>,
) {
    if !visuals_visible.0 {
        return;
    }

    let mut existing = HashSet::new();
    for visual in visual_query.iter() {
        existing.insert(visual.owner);
    }

    for (entity, transform, attack_range, team) in range_query.iter() {
        if existing.contains(&entity) {
            continue;
        }

        let material = match team.map(|team| team.0) {
            Some(Team::PLAYER) => visuals.friendly_material.clone(),
            Some(Team::ENEMY) => visuals.enemy_material.clone(),
            _ => visuals.neutral_material.clone(),
        };

        commands.spawn((
            AttackRangeVisual { owner: entity },
            Mesh2d(visuals.mesh.clone()),
            MeshMaterial2d(material),
            Transform::from_xyz(transform.translation.x, transform.translation.y, 0.2)
                .with_scale(Vec3::splat(attack_range.0)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
    }
}

pub fn update_attack_range_visuals_system(
    mut commands: Commands,
    visuals_visible: Res<HitboxVisualsVisible>,
    visuals: Res<AttackRangeVisualAssets>,
    range_query: Query<(&Transform, &AttackRange, Option<&Team>), Without<AttackRangeVisual>>,
    mut visual_query: Query<(
        Entity,
        &AttackRangeVisual,
        &mut Transform,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
) {
    for (visual_entity, visual, mut transform, mut material) in visual_query.iter_mut() {
        if !visuals_visible.0 {
            commands.entity(visual_entity).despawn();
            continue;
        }

        let Ok((owner_transform, attack_range, team)) = range_query.get(visual.owner) else {
            commands.entity(visual_entity).despawn();
            continue;
        };

        transform.translation.x = owner_transform.translation.x;
        transform.translation.y = owner_transform.translation.y;
        transform.translation.z = 0.2;
        transform.scale = Vec3::splat(attack_range.0);

        let desired_material = match team.map(|team| team.0) {
            Some(Team::PLAYER) => visuals.friendly_material.clone(),
            Some(Team::ENEMY) => visuals.enemy_material.clone(),
            _ => visuals.neutral_material.clone(),
        };

        if material.0 != desired_material {
            material.0 = desired_material;
        }
    }
}

pub fn spawn_hitbox_visuals_system(
    mut commands: Commands,
    hitbox_visible: Res<HitboxVisualsVisible>,
    visuals: Res<HitboxVisualAssets>,
    hitbox_query: Query<
        (Entity, &GlobalTransform, &Hitbox, Option<&PickupRadius>),
        Without<HitboxVisual>,
    >,
    visual_query: Query<&HitboxVisual>,
) {
    if !hitbox_visible.0 {
        return;
    }

    let mut existing = HashSet::new();
    for visual in visual_query.iter() {
        existing.insert(visual.owner);
    }

    for (entity, transform, hitbox, pickup_radius) in hitbox_query.iter() {
        if existing.contains(&entity) {
            continue;
        }

        let size = if let Some(radius) = pickup_radius {
            Vec2::splat(radius.0 * 2.0)
        } else {
            hitbox.half_size * 2.0
        };
        let mesh = if pickup_radius.is_some() {
            visuals.circle_mesh.clone()
        } else {
            visuals.box_mesh.clone()
        };

        commands.spawn((
            HitboxVisual { owner: entity },
            Mesh2d(mesh),
            MeshMaterial2d(visuals.material.clone()),
            Transform::from_xyz(transform.translation().x, transform.translation().y, 0.15)
                .with_scale(Vec3::new(size.x, size.y, 1.0)),
            GlobalTransform::default(),
            Visibility::Visible,
            InheritedVisibility::default(),
            ViewVisibility::default(),
        ));
    }
}

pub fn update_hitbox_visuals_system(
    mut commands: Commands,
    hitbox_visible: Res<HitboxVisualsVisible>,
    visuals: Res<HitboxVisualAssets>,
    hitbox_query: Query<(&GlobalTransform, &Hitbox, Option<&PickupRadius>), Without<HitboxVisual>>,
    mut visual_query: Query<(
        Entity,
        &HitboxVisual,
        &mut Transform,
        &mut Mesh2d,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
) {
    for (visual_entity, visual, mut transform, mut mesh, mut material) in visual_query.iter_mut() {
        if !hitbox_visible.0 {
            commands.entity(visual_entity).despawn();
            continue;
        }

        let Ok((owner_transform, hitbox, pickup_radius)) = hitbox_query.get(visual.owner) else {
            commands.entity(visual_entity).despawn();
            continue;
        };

        let size = if let Some(radius) = pickup_radius {
            Vec2::splat(radius.0 * 2.0)
        } else {
            hitbox.half_size * 2.0
        };
        transform.translation.x = owner_transform.translation().x;
        transform.translation.y = owner_transform.translation().y;
        transform.translation.z = 0.15;
        transform.scale = Vec3::new(size.x, size.y, 1.0);

        let desired_mesh = if pickup_radius.is_some() {
            visuals.circle_mesh.clone()
        } else {
            visuals.box_mesh.clone()
        };
        if mesh.0 != desired_mesh {
            mesh.0 = desired_mesh;
        }

        if material.0 != visuals.material {
            material.0 = visuals.material.clone();
        }
    }
}

pub fn spawn_enemy_hp_indicator_system(
    mut commands: Commands,
    assets: Res<EnemyHpIndicatorAssets>,
    ui_fonts: Res<UiFonts>,
    enemy_query: Query<(Entity, &Transform, &Hitbox), Added<Enemy>>,
) {
    for (enemy_entity, transform, hitbox) in enemy_query.iter() {
        let parent_scale = transform.scale.x.max(0.0001);
        let inverse_scale = 1.0 / parent_scale;
        let base_radius = hitbox.half_size.x.max(hitbox.half_size.y);
        let world_ring_radius =
            (base_radius * ENEMY_HP_INDICATOR_RADIUS_MULT).max(ENEMY_HP_INDICATOR_MIN_RADIUS);
        let world_segment_radius = (world_ring_radius * ENEMY_HP_INDICATOR_SEGMENT_SCALE)
            .max(ENEMY_HP_INDICATOR_MIN_SEGMENT_RADIUS);
        let ring_radius = world_ring_radius * inverse_scale;
        let segment_radius = world_segment_radius * inverse_scale;
        let offset_y = (hitbox.half_size.y + ENEMY_HP_INDICATOR_OFFSET) * inverse_scale;
        let offset_z = 2.0 * inverse_scale;
        let angle_step = TAU / ENEMY_HP_SEGMENTS as f32;
        let font_size = (world_ring_radius * 0.9).clamp(8.0, 14.0) * UI_FONT_SCALE;

        commands.entity(enemy_entity).with_children(|parent| {
            parent
                .spawn((
                    EnemyHpIndicator,
                    Transform::from_xyz(0.0, offset_y, offset_z),
                    GlobalTransform::default(),
                    Visibility::Visible,
                    InheritedVisibility::default(),
                    ViewVisibility::default(),
                ))
                .with_children(|indicator| {
                    indicator.spawn((
                        EnemyHpText,
                        Text2d::new("0"),
                        TextFont {
                            font: ui_fonts.main.clone(),
                            font_size,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout::new_with_justify(Justify::Center),
                        Transform::from_xyz(0.0, 0.0, 0.1).with_scale(Vec3::splat(inverse_scale)),
                        GlobalTransform::default(),
                        Visibility::Visible,
                        InheritedVisibility::default(),
                        ViewVisibility::default(),
                    ));
                    for index in 0..ENEMY_HP_SEGMENTS {
                        let angle = -FRAC_PI_2 + angle_step * index as f32;
                        let offset = Vec2::new(angle.cos(), angle.sin()) * ring_radius;

                        indicator.spawn((
                            EnemyHpSegment { index: index as u8 },
                            Mesh2d(assets.mesh.clone()),
                            MeshMaterial2d(assets.material.clone()),
                            Transform::from_xyz(offset.x, offset.y, 0.0)
                                .with_scale(Vec3::splat(segment_radius)),
                            GlobalTransform::default(),
                            Visibility::Visible,
                            InheritedVisibility::default(),
                            ViewVisibility::default(),
                        ));
                    }
                });
        });
    }
}

pub fn update_enemy_hp_indicator_system(
    enemy_query: Query<&Health, With<Enemy>>,
    indicator_query: Query<(&ChildOf, &Children), With<EnemyHpIndicator>>,
    mut segment_query: Query<(&EnemyHpSegment, &mut Visibility)>,
    mut text_query: Query<&mut Text2d, With<EnemyHpText>>,
) {
    for (parent, children) in indicator_query.iter() {
        let Ok(health) = enemy_query.get(parent.0) else {
            continue;
        };

        let visible_segments = if health.max > 0.0 && health.current > 0.0 {
            let ratio = (health.current / health.max).clamp(0.0, 1.0);
            let raw = (ratio * ENEMY_HP_SEGMENTS as f32).ceil() as usize;
            raw.max(1).min(ENEMY_HP_SEGMENTS)
        } else {
            0
        };

        for segment_entity in children.iter() {
            if let Ok((segment, mut visibility)) = segment_query.get_mut(segment_entity) {
                let should_show = (segment.index as usize) < visible_segments;
                *visibility = if should_show {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }

        for text_entity in children.iter() {
            if let Ok(mut text) = text_query.get_mut(text_entity) {
                text.0 = format!("{:.0}", health.current.max(0.0));
            }
        }
    }
}

fn build_upgrades_text(pet_count: usize, upgrade_state: &UpgradeState) -> String {
    let mut lines = Vec::new();

    lines.push(ui_text::format_pet_count(pet_count));
    lines.push(ui_text::format_pet_damage_mult(
        upgrade_state.pet_damage_mult,
    ));
    lines.push(ui_text::format_attack_speed_mult(
        upgrade_state.pet_attack_speed_mult,
    ));
    lines.push(ui_text::format_pet_movement_speed_mult(
        upgrade_state.pet_movement_speed_mult,
    ));
    lines.push(ui_text::format_range_bonus(upgrade_state.pet_range_bonus));

    if upgrade_state.projectile_extra_shots > 0 {
        lines.push(ui_text::format_extra_projectiles(
            upgrade_state.projectile_extra_shots,
        ));
    }

    if upgrade_state.projectile_pierce_bonus > 0 {
        lines.push(ui_text::format_pierce_bonus(
            upgrade_state.projectile_pierce_bonus,
        ));
    }

    if upgrade_state.area_damage_cone_angle_deg > 0.0 {
        lines.push(ui_text::format_area_damage(
            upgrade_state.area_damage_cone_angle_deg,
        ));
    }

    lines.push(ui_text::format_gold_mult(upgrade_state.gold_drop_mult));

    lines.join("\n")
}
