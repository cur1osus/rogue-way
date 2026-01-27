use bevy::prelude::*;

mod components;
mod constants;
mod resources;
mod systems;
mod ui;

use resources::*;
use systems::*;
use ui::*;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Merchant's Menagerie".to_string(),
                        resolution: (1280, 720).into(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        // Состояния
        .init_state::<GameState>()
        // События
        .add_message::<DamageEvent>()
        .add_message::<GainXpEvent>()
        .add_message::<LevelUpEvent>()
        // Ресурсы
        .init_resource::<WaveConfig>()
        .init_resource::<UpgradeState>()
        .init_resource::<ScreenShake>()
        .init_resource::<UiFonts>()
        .init_resource::<EnemySpriteSheet>()
        .init_resource::<MetaProgression>()
        .init_resource::<PetSpriteSheet>()
        .init_resource::<ParticleEffects>()
        .init_resource::<GoldSprites>()
        .init_resource::<XpGemSprites>()
        .init_resource::<StatsPanelVisible>()
        .init_resource::<PerformanceStats>()
        .init_resource::<AttackRangeVisualAssets>()
        .init_resource::<HitboxVisualAssets>()
        .init_resource::<HitboxVisualsVisible>()
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
                menu_floating_animation_system,
                menu_button_hover_system,
            )
                .run_if(in_state(GameState::MainMenu)),
        )
        .add_systems(OnExit(GameState::MainMenu), cleanup_main_menu)
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
            (setup_player, setup_pets, setup_hud),
        )
        // Системы очистки при переходе в меню/магазин (НЕ при LevelUpChoice!)
        .add_systems(OnEnter(GameState::Shop), cleanup_game_entities)
        .add_systems(OnEnter(GameState::MainMenu), cleanup_game_entities)
        .add_systems(OnExit(GameState::Shop), cleanup_game_entities)
        // Системы обновления (только во время игры)
        .add_systems(
            Update,
            (
                input_system,
                enemy_ai_system,
                enemy_facing_system,
                movement_system,
                entity_collision_system,
                player_animation_system,
                pet_ai_system,
                pet_animation_system,
                pet_attack_timer_system,
                spawn_system,
                attack_timer_tick_system,
                enemy_animation_system,
                pending_attack_system,
                collision_system,
                pet_projectile_attack_system,
                projectile_movement_system,
                projectile_collision_system,
                enemy_attack_system,
                damage_system,
                slow_effect_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (
                pickup_system,
                gain_xp_system,
                gold_highlight_system,
                gold_animation_system,
                hit_flash_system,
                floating_text_system,
                particle_system,
                effect_animation_system,
                timed_despawn_system,
                screen_shake_system,
                camera_follow_system,
                cleanup_on_death_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        // Системы, которые работают всегда
        .add_systems(
            Update,
            (
                level_up_system,
                ui_update_system,
                boss_health_bar_system,
                update_boss_health_bar_system,
                auto_save_system,
            ),
        )
        // Системы панели статов (работают только в Playing)
        .add_systems(
            Update,
            (
                toggle_hitbox_visuals_system,
                toggle_stats_panel_system,
                stats_panel_system,
                performance_stats_system,
                update_stats_panel_system,
            )
                .run_if(in_state(GameState::Playing)),
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
        // Системы выбора апгрейда
        .add_systems(
            Update,
            handle_upgrade_button.run_if(in_state(GameState::LevelUpChoice)),
        )
        .run();
}
