use crate::components::{
    EffectSprite, Enemy, FloatingText, GoldPickup, HitFlash, Particle, Pet, Player, Projectile,
    TimedDespawn, XpGem,
};
use crate::ui::{AttackRangeVisual, GameState, HitboxVisual, HudUI};
use bevy::prelude::*;

/// Система очистки игры при смерти игрока
pub fn cleanup_on_death_system(
    player_query: Query<(&crate::components::Health, Entity), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // Проверяем жив ли игрок
    if let Ok((health, _)) = player_query.single() {
        if health.current <= 0.0 {
            // Игрок мёртв, возвращаемся в магазин через небольшую задержку
            // Пока просто сразу переключаемся
            next_state.set(GameState::Shop);
        }
    }
}

/// Система очистки всех игровых сущностей при выходе из игры
pub fn cleanup_game_entities(
    mut commands: Commands,
    player_query: Query<Entity, With<Player>>,
    enemy_query: Query<Entity, With<Enemy>>,
    pet_query: Query<Entity, With<Pet>>,
    projectile_query: Query<Entity, With<Projectile>>,
    xp_query: Query<Entity, With<XpGem>>,
    gold_query: Query<Entity, With<GoldPickup>>,
    hud_query: Query<Entity, With<HudUI>>,
    visual_query: Query<Entity, Or<(With<AttackRangeVisual>, With<HitboxVisual>)>>,
    effect_query: Query<Entity, With<EffectSprite>>,
    particle_query: Query<Entity, With<Particle>>,
    floating_text_query: Query<Entity, With<FloatingText>>,
    timed_despawn_query: Query<Entity, With<TimedDespawn>>,
    hit_flash_query: Query<Entity, With<HitFlash>>,
    mut upgrade_state: ResMut<crate::resources::UpgradeState>,
    mut wave_config: ResMut<crate::resources::WaveConfig>,
) {
    let mut total_deleted = 0;

    // Удаляем игрока
    let player_count = player_query.iter().count();
    for entity in player_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += player_count;

    // Удаляем врагов
    let enemy_count = enemy_query.iter().count();
    for entity in enemy_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += enemy_count;

    // Удаляем питомцев
    let pet_count = pet_query.iter().count();
    for entity in pet_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += pet_count;

    // Удаляем снаряды
    let projectile_count = projectile_query.iter().count();
    for entity in projectile_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += projectile_count;

    // Удаляем XP гемы
    let xp_count = xp_query.iter().count();
    for entity in xp_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += xp_count;

    // Удаляем золото
    let gold_count = gold_query.iter().count();
    for entity in gold_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += gold_count;

    // Удаляем HUD
    let hud_count = hud_query.iter().count();
    for entity in hud_query.iter() {
        commands.entity(entity).despawn();
    }
    let visual_count = visual_query.iter().count();
    for entity in visual_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in effect_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in particle_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in floating_text_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in timed_despawn_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in hit_flash_query.iter() {
        commands.entity(entity).despawn();
    }
    total_deleted += hud_count + visual_count;

    // Сбрасываем состояние апгрейдов для новой игры
    *upgrade_state = crate::resources::UpgradeState::default();

    // Сбрасываем конфигурацию волн
    *wave_config = crate::resources::WaveConfig::default();
}
