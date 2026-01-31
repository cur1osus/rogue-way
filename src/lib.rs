pub mod components;
pub mod constants;
pub mod network;
pub mod resources;
pub mod systems;
pub mod ui;

use bevy::prelude::*;
use bevy::window::{MonitorSelection, PrimaryWindow, WindowMode};
use bevy::winit::WinitWindows;
use winit::window::Icon;

use network::*;
use resources::*;
use systems::*;
use ui::*;

pub fn build_base_app() -> App {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Roggy".to_string(),
                    resolution: (1280, 720).into(),
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
    )
    .add_plugins(NetworkPlugin)
    // Состояния
    .init_state::<GameState>()
    // События
    .add_message::<DamageEvent>()
    .add_message::<GainXpEvent>()
    .add_message::<LevelUpEvent>()
    .add_message::<NetFxEvent>()
    // Ресурсы
    .init_resource::<WaveConfig>()
    .init_resource::<UpgradeState>()
    .init_resource::<ScreenShake>()
    .init_resource::<PlayerDamageFlash>()
    .init_resource::<PhysicsAccumulator>()
    .init_resource::<UiFonts>()
    .init_resource::<JoinCodeState>()
    .init_resource::<EnemySpriteSheet>()
    .init_resource::<MetaProgression>()
    .init_resource::<PetSpriteSheet>()
    .init_resource::<ParticleEffects>()
    .init_resource::<GoldSprites>()
    .init_resource::<XpGemSprites>()
    .init_resource::<StatsPanelVisible>()
    .init_resource::<DevPanelVisible>()
    .init_resource::<PerformanceStats>()
    .init_resource::<AttackRangeVisualAssets>()
    .init_resource::<HitboxVisualAssets>()
    .init_resource::<AreaDamageVisualAssets>()
    .init_resource::<EnemyHpIndicatorAssets>()
    .init_resource::<HitboxVisualsVisible>()
    .init_resource::<TerrainSprites>()
    .init_resource::<TerrainConfig>()
    .init_resource::<TerrainChunks>()
    .init_resource::<PlayerAttackSprites>()
    // Системы запуска (Startup)
    .add_systems(Startup, setup_camera)
    // Системы главного меню
    .add_systems(
        OnEnter(GameState::MainMenu),
        (reset_camera_for_main_menu, setup_main_menu),
    )
    .add_systems(
        Update,
        (
            handle_main_menu_buttons,
            join_menu_input_system,
            join_menu_visibility_system,
            menu_floating_animation_system,
            menu_button_hover_system,
            menu_resize_system,
        )
            .run_if(in_state(GameState::MainMenu)),
    )
    .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
    // Системы паузы
    .add_systems(OnEnter(GameState::Paused), setup_pause_menu)
    .add_systems(
        Update,
        (
            handle_pause_menu_buttons,
            pause_menu_button_hover_system,
            resume_on_escape_system,
        )
            .run_if(in_state(GameState::Paused)),
    )
    .add_systems(OnExit(GameState::Paused), cleanup_pause_menu)
    // Системы для магазина
    .add_systems(OnEnter(GameState::Shop), setup_shop_ui)
    .add_systems(
        Update,
        (
            setup_shop_ui, // Пересоздаёт UI если он был удалён после покупки
            handle_shop_buttons,
            update_gold_balance,
            scroll_system,
            update_scroll_bounds,
        )
            .run_if(in_state(GameState::Shop)),
    )
    // Системы при входе в игру
    .add_systems(
        OnEnter(GameState::Playing),
        (setup_player, setup_pets).run_if(is_authoritative),
    )
    .add_systems(
        OnEnter(GameState::Playing),
        (setup_hud, reset_terrain_chunks),
    )
    // Системы очистки при переходе в меню/магазин (НЕ при LevelUpChoice!)
    .add_systems(OnEnter(GameState::Shop), cleanup_game_entities)
    .add_systems(OnEnter(GameState::MainMenu), cleanup_game_entities)
    .add_systems(OnExit(GameState::Shop), cleanup_game_entities)
    // Системы обновления (только во время игры)
    .add_systems(
        Update,
        (
            apply_player_input_system,
            pushback_last_direction_system,
            pushback_input_system,
            pushback_ready_glow_setup_system.before(pushback_ready_glow_system),
            pushback_ready_glow_system.after(pushback_input_system),
            enemy_ai_system,
            physics_update_system,
            knockback_effect_system,
            pet_ai_system,
            pet_attack_timer_system,
        )
            .run_if(in_state(GameState::Playing))
            .run_if(is_authoritative),
    )
    .add_systems(
        Update,
        (
            input_system,
            enemy_facing_system,
            interpolation_system,
            player_animation_system,
            pushback_ready_glow_sync_system.after(player_animation_system),
            pushback_animation_system,
            pushback_overlay_system,
            pushback_cone_visual_system,
            pet_animation_system,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        pause_on_escape_system.run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        (
            spawn_system,
            attack_timer_tick_system,
            pending_attack_system,
            collision_system,
            pet_projectile_attack_system,
            projectile_movement_system,
            projectile_collision_system,
            enemy_attack_system,
            damage_system,
            slow_effect_system,
        )
            .run_if(in_state(GameState::Playing))
            .run_if(is_authoritative),
    )
    .add_systems(
        Update,
        (enemy_animation_system, enemy_death_animation_system)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        terrain_chunk_system.run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        (pickup_system, gain_xp_system, cleanup_on_death_system)
            .run_if(in_state(GameState::Playing))
            .run_if(is_authoritative),
    )
    .add_systems(
        Update,
        (
            gold_highlight_system,
            gold_animation_system,
            hit_flash_system,
            player_damage_flash_system,
            floating_text_system,
            particle_system,
            effect_animation_system,
            timed_despawn_system,
            screen_shake_system,
            camera_follow_system,
        )
            .run_if(in_state(GameState::Playing)),
    )
    // Системы, которые работают всегда
    .add_systems(
        Update,
        (
            ui_update_system,
            boss_health_bar_system,
            update_boss_health_bar_system,
            set_window_icon,
        ),
    )
    .add_systems(
        Update,
        (level_up_system, auto_save_system).run_if(is_authoritative),
    )
    // Системы панели статов (работают только в Playing)
    .add_systems(
        Update,
        (
            toggle_hitbox_visuals_system,
            toggle_stats_panel_system,
            toggle_dev_panel_system,
            stats_panel_system,
            dev_panel_system,
            performance_stats_system,
            update_stats_panel_system,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        dev_panel_actions_system
            .run_if(in_state(GameState::Playing))
            .run_if(is_authoritative),
    )
    .add_systems(
        Update,
        (
            spawn_attack_range_visuals_system,
            update_attack_range_visuals_system,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        (spawn_hitbox_visuals_system, update_hitbox_visuals_system)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(
        Update,
        (
            spawn_enemy_hp_indicator_system,
            update_enemy_hp_indicator_system,
        )
            .run_if(in_state(GameState::Playing)),
    )
    // Системы выбора апгрейда
    .add_systems(
        Update,
        handle_upgrade_button.run_if(in_state(GameState::LevelUpChoice)),
    );

    app
}

fn set_window_icon(
    windows: Option<NonSend<WinitWindows>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut has_set: Local<bool>,
) {
    if *has_set {
        return;
    }

    let windows = match windows {
        Some(windows) => windows,
        None => return,
    };

    let window_entity = match primary_window.iter().next() {
        Some(entity) => entity,
        None => return,
    };

    let window = match windows.get_window(window_entity) {
        Some(window) => window,
        None => return,
    };

    let image = match image::open("assets/app_image/roggy.png") {
        Ok(image) => image.into_rgba8(),
        Err(_) => return,
    };

    let (width, height) = (image.width(), image.height());
    let rgba = image.into_raw();

    let icon = match Icon::from_rgba(rgba, width, height) {
        Ok(icon) => icon,
        Err(_) => return,
    };

    window.set_window_icon(Some(icon));
    *has_set = true;
}
