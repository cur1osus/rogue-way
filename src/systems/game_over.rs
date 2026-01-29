use crate::components::{
    EffectSprite, Enemy, FloatingText, GoldPickup, HitFlash, Particle, Pet, Player, Projectile,
    PushbackReadyGlow, TerrainChunk, TimedDespawn, XpGem,
};
use crate::ui::{AttackRangeVisual, GameState, HitboxVisual, HudUI};
use bevy::prelude::*;

/// Система очистки игры при смерти игрока
pub fn cleanup_on_death_system(
    player_query: Query<(&crate::components::Health, Entity), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if player_query.is_empty() {
        return;
    }

    let mut any_alive = false;
    for (health, _) in player_query.iter() {
        if health.current > 0.0 {
            any_alive = true;
            break;
        }
    }

    if !any_alive {
        next_state.set(GameState::Shop);
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
    glow_query: Query<Entity, With<PushbackReadyGlow>>,
    mut upgrade_state: ResMut<crate::resources::UpgradeState>,
    mut wave_config: ResMut<crate::resources::WaveConfig>,
    mut terrain_chunks: ResMut<crate::resources::TerrainChunks>,
    mut network_map: Option<ResMut<crate::network::NetworkEntityMap>>,
) {
    for entity in cleanup_query.iter() {
        commands.entity(entity).despawn();
    }
    for entity in glow_query.iter() {
        commands.entity(entity).despawn();
    }
    terrain_chunks.chunks.clear();

    // Сбрасываем состояние апгрейдов для новой игры
    *upgrade_state = crate::resources::UpgradeState::default();

    // Сбрасываем конфигурацию волн
    *wave_config = crate::resources::WaveConfig::default();

    if let Some(mut map) = network_map {
        map.entities.clear();
    }
}
