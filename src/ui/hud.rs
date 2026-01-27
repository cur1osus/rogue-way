use crate::components::{
    AttackRange, Boss, Experience, Gold, Health, Hitbox, MovementSpeed, Pet, Player, Team,
};
use crate::resources::{UiFonts, UpgradeState, WaveConfig};
use bevy::math::primitives::{Circle, Rectangle};
use bevy::prelude::*;
use std::collections::HashSet;
use sysinfo::System;

/// Общий маркер для всех элементов HUD
#[derive(Component)]
pub struct HudUI;

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

/// Маркер для полоски здоровья босса
#[derive(Component)]
pub struct BossHealthBar;

/// Маркер для текста имени босса
#[derive(Component)]
pub struct BossNameText;

/// Маркер для панели статов
#[derive(Component)]
pub struct StatsPanel;

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
pub struct StatsMemoryText;

/// Ресурс для управления видимостью панели статов
#[derive(Resource, Default)]
pub struct StatsPanelVisible(pub bool);

#[derive(Resource, Default)]
pub struct HitboxVisualsVisible(pub bool);

#[derive(Resource)]
pub struct PerformanceStats {
    pub fps: f32,
    pub memory_mb: f32,
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

#[derive(Resource)]
pub struct AttackRangeVisualAssets {
    pub mesh: Handle<Mesh>,
    pub friendly_material: Handle<ColorMaterial>,
    pub enemy_material: Handle<ColorMaterial>,
    pub neutral_material: Handle<ColorMaterial>,
}

#[derive(Resource)]
pub struct HitboxVisualAssets {
    pub mesh: Handle<Mesh>,
    pub material: Handle<ColorMaterial>,
}

impl FromWorld for AttackRangeVisualAssets {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
            let mut materials = world.resource_mut::<Assets<ColorMaterial>>();

            let mesh = meshes.add(Mesh::from(Circle::new(1.0)));
            let friendly_material = materials.add(Color::srgba(0.2, 0.8, 1.0, 0.18));
            let enemy_material = materials.add(Color::srgba(1.0, 0.3, 0.3, 0.15));
            let neutral_material = materials.add(Color::srgba(0.8, 0.8, 0.8, 0.12));

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

            let mesh = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
            let material = materials.add(Color::srgba(0.2, 1.0, 0.6, 0.18));

            Self { mesh, material }
        })
    }
}

/// Настройка HUD (запускается один раз при старте)
pub fn setup_hud(
    mut commands: Commands,
    ui_fonts: Res<UiFonts>,
    existing_hud: Query<Entity, With<HudUI>>,
) {
    // Если HUD уже существует, не создаем новый (возврат из LevelUpChoice)
    if !existing_hud.is_empty() {
        return;
    }

    let font = ui_fonts.main.clone();
    // HP текст (сверху слева)
    commands.spawn((
        HudUI,
        Text::new("ОЗ: 100/100"),
        TextFont {
            font: font.clone(),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
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
        Text::new("Время: 0:00"),
        TextFont {
            font: font.clone(),
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 1.0)),
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
        Text::new("Уровень 1 | Опыт: 0/100"),
        TextFont {
            font: font.clone(),
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.3, 0.8, 1.0)),
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
        Text::new("Золото: 0"),
        TextFont {
            font: font.clone(),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.84, 0.0)), // Золотой цвет
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
        GoldText,
    ));
}

/// Система обновления HUD
pub fn ui_update_system(
    player_query: Query<(&Health, &Experience, &Gold), With<Player>>,
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
            text.0 = format!("ОЗ: {:.0}/{:.0}", health.current, health.max);
        }

        // Обновляем XP
        if let Ok(mut text) = xp_text_query.single_mut() {
            text.0 = format!(
                "Уровень {} | Опыт: {}/{}",
                experience.level, experience.current, experience.to_next_level
            );
        }

        // Обновляем золото
        if let Ok(mut text) = gold_text_query.single_mut() {
            text.0 = format!("Золото: {}", gold.amount);
        }
    }

    // Обновляем таймер
    if let Ok(mut text) = timer_text_query.single_mut() {
        let total_seconds = wave_config.game_time as u32;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        text.0 = format!("Время: {}:{:02}", minutes, seconds);
    }
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
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::srgb(1.0, 0.2, 0.2)),
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
                            BackgroundColor(Color::srgb(0.8, 0.1, 0.1)),
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
            perf.system.refresh_pids(&[pid]);
            if let Some(process) = perf.system.process(pid) {
                perf.memory_mb = process.memory() as f32 / (1024.0 * 1024.0);
            }
        } else {
            perf.system.refresh_processes();
        }
    }
}

/// Система создания/обновления панели статов
pub fn stats_panel_system(
    mut commands: Commands,
    stats_visible: Res<StatsPanelVisible>,
    mut panel_query: Query<&mut Node, With<StatsPanel>>,
    player_query: Query<(&Health, &MovementSpeed, &Experience, &Gold), With<Player>>,
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
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            StatsPanel,
            HudUI,
        ))
        .with_children(|parent| {
            // Заголовок
            parent.spawn((
                Text::new("СТАТИСТИКА"),
                TextFont {
                    font: font.clone(),
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 1.0, 0.3)),
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
                BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            // Статы игрока
            parent.spawn((
                Text::new(format!("ОЗ: {:.0}/{:.0}", health.current, health.max)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.4, 0.4)),
                StatsHealthText,
            ));

            parent.spawn((
                Text::new(format!("Уровень: {}", experience.level)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.8, 1.0)),
                StatsLevelText,
            ));

            parent.spawn((
                Text::new(format!(
                    "Опыт: {}/{}",
                    experience.current, experience.to_next_level
                )),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.3, 0.8, 1.0)),
                StatsExperienceText,
            ));

            parent.spawn((
                Text::new(format!("Золото: {}", gold.amount)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.84, 0.0)),
                StatsGoldText,
            ));

            parent.spawn((
                Text::new(format!("Скорость: {:.0}", speed.0)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 1.0, 0.5)),
                StatsSpeedText,
            ));

            parent.spawn((
                Text::new(format!("FPS: {:.0}", perf_stats.fps)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.9, 1.0)),
                StatsFpsText,
            ));

            parent.spawn((
                Text::new(format!("Память: {:.1} МБ", perf_stats.memory_mb)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.9, 1.0)),
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
                BackgroundColor(Color::srgb(0.5, 0.5, 0.5)),
            ));

            // Статы улучшений
            parent.spawn((
                Text::new("УЛУЧШЕНИЯ"),
                TextFont {
                    font: font.clone(),
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.3)),
                Node {
                    margin: UiRect::bottom(Val::Px(5.0)),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new(build_upgrades_text(pet_count, &upgrade_state)),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.8, 0.8)),
                StatsUpgradesText,
            ));

            // Подсказка
            parent.spawn((
                Text::new("\nНажми T чтобы скрыть"),
                TextFont {
                    font: font.clone(),
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
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
    player_query: Query<(&Health, &MovementSpeed, &Experience, &Gold), With<Player>>,
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

    for (mut text, is_health, is_level, is_xp, is_gold, is_speed, is_upgrades, is_fps, is_memory) in
        stats_text_query.iter_mut()
    {
        if is_health.is_some() {
            text.0 = format!("ОЗ: {:.0}/{:.0}", health.current, health.max);
        } else if is_level.is_some() {
            text.0 = format!("Уровень: {}", experience.level);
        } else if is_xp.is_some() {
            text.0 = format!("Опыт: {}/{}", experience.current, experience.to_next_level);
        } else if is_gold.is_some() {
            text.0 = format!("Золото: {}", gold.amount);
        } else if is_speed.is_some() {
            text.0 = format!("Скорость: {:.0}", speed.0);
        } else if is_upgrades.is_some() {
            text.0 = upgrades_text.clone();
        } else if is_fps.is_some() {
            text.0 = format!("FPS: {:.0}", perf_stats.fps);
        } else if is_memory.is_some() {
            text.0 = format!("Память: {:.1} МБ", perf_stats.memory_mb);
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
    hitbox_query: Query<(Entity, &Transform, &Hitbox), Without<HitboxVisual>>,
    visual_query: Query<&HitboxVisual>,
) {
    if !hitbox_visible.0 {
        return;
    }

    let mut existing = HashSet::new();
    for visual in visual_query.iter() {
        existing.insert(visual.owner);
    }

    for (entity, transform, hitbox) in hitbox_query.iter() {
        if existing.contains(&entity) {
            continue;
        }

        let size = hitbox.half_size * 2.0;

        commands.spawn((
            HitboxVisual { owner: entity },
            Mesh2d(visuals.mesh.clone()),
            MeshMaterial2d(visuals.material.clone()),
            Transform::from_xyz(transform.translation.x, transform.translation.y, 0.15)
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
    hitbox_query: Query<(&Transform, &Hitbox), Without<HitboxVisual>>,
    mut visual_query: Query<(
        Entity,
        &HitboxVisual,
        &mut Transform,
        &mut MeshMaterial2d<ColorMaterial>,
    )>,
) {
    for (visual_entity, visual, mut transform, mut material) in visual_query.iter_mut() {
        if !hitbox_visible.0 {
            commands.entity(visual_entity).despawn();
            continue;
        }

        let Ok((owner_transform, hitbox)) = hitbox_query.get(visual.owner) else {
            commands.entity(visual_entity).despawn();
            continue;
        };

        let size = hitbox.half_size * 2.0;
        transform.translation.x = owner_transform.translation.x;
        transform.translation.y = owner_transform.translation.y;
        transform.translation.z = 0.15;
        transform.scale = Vec3::new(size.x, size.y, 1.0);

        if material.0 != visuals.material {
            material.0 = visuals.material.clone();
        }
    }
}

fn build_upgrades_text(pet_count: usize, upgrade_state: &UpgradeState) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Питомцев: {}", pet_count));
    lines.push(format!(
        "Урон питомцев: x{:.2}",
        upgrade_state.pet_damage_mult
    ));
    lines.push(format!(
        "Скорость атаки: x{:.2}",
        upgrade_state.pet_attack_speed_mult
    ));
    lines.push(format!("Дальность: +{:.0}", upgrade_state.pet_range_bonus));

    if upgrade_state.projectile_extra_shots > 0 {
        lines.push(format!(
            "Доп. снаряды: +{}",
            upgrade_state.projectile_extra_shots
        ));
    }

    if upgrade_state.projectile_pierce_bonus > 0 {
        lines.push(format!(
            "Пробивание: +{}",
            upgrade_state.projectile_pierce_bonus
        ));
    }

    if upgrade_state.area_damage_radius > 0.0 {
        lines.push(format!(
            "Радиус урона: {:.0}",
            upgrade_state.area_damage_radius
        ));
    }

    lines.push(format!(
        "Множ. золота: x{:.2}",
        upgrade_state.gold_drop_mult
    ));

    lines.join("\n")
}
