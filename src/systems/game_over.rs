use crate::components::{
    EffectSprite, Enemy, FloatingText, GoldPickup, HitFlash, Particle, Pet, Player, Projectile,
    TerrainChunk, TimedDespawn, XpGem,
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
    cleanup_query: Query<
        Entity,
        Or<(
            With<Player>,
            With<Enemy>,
            With<Pet>,
            With<Projectile>,
            With<XpGem>,
            With<GoldPickup>,
            With<HudUI>,
            With<AttackRangeVisual>,
            With<HitboxVisual>,
            With<EffectSprite>,
            With<Particle>,
            With<FloatingText>,
            With<TimedDespawn>,
            With<HitFlash>,
            With<TerrainChunk>,
        )>,
    >,
    mut upgrade_state: ResMut<crate::resources::UpgradeState>,
    mut wave_config: ResMut<crate::resources::WaveConfig>,
    mut terrain_chunks: ResMut<crate::resources::TerrainChunks>,
) {
    for entity in cleanup_query.iter() {
        commands.entity(entity).despawn();
    }
    terrain_chunks.chunks.clear();

    // Сбрасываем состояние апгрейдов для новой игры
    *upgrade_state = crate::resources::UpgradeState::default();

    // Сбрасываем конфигурацию волн
    *wave_config = crate::resources::WaveConfig::default();
}
