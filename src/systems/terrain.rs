use bevy::prelude::*;
use std::collections::HashSet;

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::{
    Color, Commands, Entity, IVec2, Query, Res, ResMut, Sprite, TextureAtlas, Transform, Vec2,
    Vec3, With,
};

use crate::components::{
    CollisionLayer, Hitbox, PhysicsPosition, Player, PreviousPhysicsPosition, TerrainChunk,
    TerrainDecoration, TerrainTile, Velocity,
};
use crate::constants::ROCK_HITBOX_SCALE;
use crate::resources::{TerrainChunks, TerrainConfig, TerrainSprites, TerrainTileset};

const Z_GROUND: f32 = 0.0;
const Z_WATER: f32 = 0.01;
const Z_FOAM: f32 = 0.02;
#[allow(dead_code)]
const Z_SHADOW: f32 = 0.03;
const Z_DECOR: f32 = 0.04;

const GROUND_SOLID_INDICES: &[usize] = &[10];
const WATER_FOAM_INDICES: &[usize] = &[
    37, 40, 49, 52, 55, 58, 61, 64, 67, 70, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85,
    86, 87, 88, 89, 91, 94, 97, 100, 103, 106, 109, 112, 115, 118, 121, 124, 127, 130, 133, 136,
    139, 142,
];

pub fn reset_terrain_chunks(
    mut terrain_chunks: ResMut<TerrainChunks>,
    chunk_query: Query<Entity, With<TerrainChunk>>,
) {
    if chunk_query.is_empty() {
        terrain_chunks.chunks.clear();
    }
}

pub fn terrain_chunk_system(
    mut commands: Commands,
    player_query: Query<&Transform, With<Player>>,
    mut terrain_chunks: ResMut<TerrainChunks>,
    terrain_sprites: Res<TerrainSprites>,
    config: Res<TerrainConfig>,
) {
    let Ok(player_transform) = player_query.single() else {
        return;
    };

    let center_chunk = world_to_chunk(player_transform.translation.truncate(), &config);
    let mut desired_chunks = HashSet::new();

    for dx in -config.view_distance..=config.view_distance {
        for dy in -config.view_distance..=config.view_distance {
            desired_chunks.insert(IVec2::new(center_chunk.x + dx, center_chunk.y + dy));
        }
    }

    for coords in desired_chunks.iter() {
        if terrain_chunks.chunks.contains_key(coords) {
            continue;
        }
        let chunk_entity = spawn_chunk(&mut commands, *coords, &terrain_sprites, &config);
        terrain_chunks.chunks.insert(*coords, chunk_entity);
    }

    let mut to_remove = Vec::new();
    for (coords, entity) in terrain_chunks.chunks.iter() {
        if !desired_chunks.contains(coords) {
            commands.entity(*entity).despawn();
            to_remove.push(*coords);
        }
    }
    for coords in to_remove {
        terrain_chunks.chunks.remove(&coords);
    }
}

pub fn spawn_menu_terrain(
    parent: &mut ChildSpawnerCommands,
    window_size: Vec2,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
) {
    let tile_size = config.tile_size;
    let padding = tile_size * 2.0;
    let half_width = window_size.x / 2.0 + padding;
    let half_height = window_size.y / 2.0 + padding;

    let min_tile_x = ((-half_width) / tile_size).floor() as i32;
    let max_tile_x = (half_width / tile_size).ceil() as i32;
    let min_tile_y = ((-half_height) / tile_size).floor() as i32;
    let max_tile_y = (half_height / tile_size).ceil() as i32;

    let chunk_origin = Vec3::ZERO;

    for tile_x in min_tile_x..=max_tile_x {
        for tile_y in min_tile_y..=max_tile_y {
            let position = Vec3::new(
                (tile_x as f32 + 0.5) * tile_size,
                (tile_y as f32 + 0.5) * tile_size,
                0.0,
            );

            spawn_ground_tile(parent, position, terrain_sprites, config, tile_x, tile_y);
            spawn_land_decorations(
                parent,
                position,
                terrain_sprites,
                config,
                tile_x,
                tile_y,
                chunk_origin,
            );
        }
    }
}

fn spawn_chunk(
    commands: &mut Commands,
    chunk_coords: IVec2,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
) -> Entity {
    let chunk_origin = Vec3::new(
        chunk_coords.x as f32 * config.chunk_size as f32 * config.tile_size,
        chunk_coords.y as f32 * config.chunk_size as f32 * config.tile_size,
        0.0,
    );
    let chunk_entity = commands
        .spawn((
            Transform::from_translation(chunk_origin),
            TerrainChunk {
                coords: chunk_coords,
            },
        ))
        .id();

    let chunk_origin_tile_x = chunk_coords.x * config.chunk_size;
    let chunk_origin_tile_y = chunk_coords.y * config.chunk_size;

    commands.entity(chunk_entity).with_children(|parent| {
        for local_x in 0..config.chunk_size {
            for local_y in 0..config.chunk_size {
                let tile_x = chunk_origin_tile_x + local_x;
                let tile_y = chunk_origin_tile_y + local_y;
                let position = Vec3::new(
                    (local_x as f32 + 0.5) * config.tile_size,
                    (local_y as f32 + 0.5) * config.tile_size,
                    0.0,
                );

                let water_value = water_value(tile_x, tile_y, config);
                let is_water = water_value < config.water_level;

                if is_water {
                    spawn_water_tile(parent, position, terrain_sprites);
                    if is_shoreline(tile_x, tile_y, config)
                        && hash_f32(tile_x, tile_y, config.seed.wrapping_add(9103))
                            < config.shoreline_chance
                    {
                        spawn_foam(parent, position, terrain_sprites, config, tile_x, tile_y);
                    }
                    spawn_water_decorations(
                        parent,
                        position,
                        terrain_sprites,
                        config,
                        tile_x,
                        tile_y,
                        chunk_origin,
                    );
                } else {
                    spawn_ground_tile(parent, position, terrain_sprites, config, tile_x, tile_y);
                    spawn_land_decorations(
                        parent,
                        position,
                        terrain_sprites,
                        config,
                        tile_x,
                        tile_y,
                        chunk_origin,
                    );
                }
            }
        }
    });

    chunk_entity
}

fn spawn_ground_tile(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
    tile_x: i32,
    tile_y: i32,
) {
    let biome = biome_value(tile_x, tile_y, config);
    let tileset = pick_tileset(
        &terrain_sprites.ground_tilesets,
        tile_x,
        tile_y,
        config.seed,
        biome,
    );
    let tile_index = pick_from_indices(
        GROUND_SOLID_INDICES,
        tile_x,
        tile_y,
        config.seed.wrapping_add(1337),
    );
    let scale = config.tile_size / tileset.tile_size.x;

    parent.spawn((
        Sprite {
            image: tileset.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: tileset.layout.clone(),
                index: tile_index,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, Z_GROUND).with_scale(Vec3::splat(scale)),
        TerrainTile,
    ));
}

fn spawn_water_tile(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
) {
    parent.spawn((
        Sprite {
            image: terrain_sprites.water_background.clone(),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, Z_WATER),
        TerrainTile,
    ));
}

fn spawn_foam(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
    tile_x: i32,
    tile_y: i32,
) {
    let tileset = &terrain_sprites.water_foam;
    let tile_index = pick_from_indices(
        WATER_FOAM_INDICES,
        tile_x,
        tile_y,
        config.seed.wrapping_add(7719),
    );
    let scale = config.tile_size / tileset.tile_size.x;

    parent.spawn((
        Sprite {
            image: tileset.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: tileset.layout.clone(),
                index: tile_index,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, Z_FOAM).with_scale(Vec3::splat(scale)),
        TerrainDecoration,
    ));
}

fn spawn_land_decorations(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
    tile_x: i32,
    tile_y: i32,
    chunk_origin: Vec3,
) {
    let roll = hash_f32(tile_x, tile_y, config.seed.wrapping_add(4242));

    if roll < config.bush_chance {
        let tileset = pick_simple_tileset(
            &terrain_sprites.bushes,
            tile_x,
            tile_y,
            config.seed.wrapping_add(101),
        );
        let tile_index = pick_tile_index(tileset, tile_x, tile_y, config.seed.wrapping_add(102));
        spawn_tileset_decoration(parent, position, tileset, config, tile_index, 2.0);
    } else if roll < config.bush_chance + config.rock_chance {
        let rock_index = pick_index_from_len(
            terrain_sprites.rocks.len(),
            tile_x,
            tile_y,
            config.seed.wrapping_add(103),
        );
        let hitbox_size = Vec2::splat(config.tile_size * ROCK_HITBOX_SCALE);
        // Вычисляем мировые координаты (chunk_origin + local position)
        let world_position = Vec2::new(chunk_origin.x + position.x, chunk_origin.y + position.y);
        parent.spawn((
            Sprite {
                image: terrain_sprites.rocks[rock_index].clone(),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, Z_DECOR),
            TerrainDecoration,
            Hitbox::from_full_size(hitbox_size),
            CollisionLayer::obstacle(),
            PhysicsPosition(world_position),
            PreviousPhysicsPosition(world_position),
            Velocity::default(),
        ));
    }
}

fn spawn_water_decorations(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
    tile_x: i32,
    tile_y: i32,
    chunk_origin: Vec3,
) {
    let roll = hash_f32(tile_x, tile_y, config.seed.wrapping_add(9001));

    if roll < config.water_rock_chance {
        let tileset = pick_simple_tileset(
            &terrain_sprites.water_rocks,
            tile_x,
            tile_y,
            config.seed.wrapping_add(200),
        );
        let tile_index = pick_tile_index(tileset, tile_x, tile_y, config.seed.wrapping_add(201));
        spawn_tileset_obstacle(
            parent,
            position,
            tileset,
            config,
            tile_index,
            1.0,
            chunk_origin,
        );
    } else if roll < config.water_rock_chance + config.duck_chance {
        let tileset = &terrain_sprites.rubber_duck;
        let tile_index = pick_tile_index(tileset, tile_x, tile_y, config.seed.wrapping_add(202));
        let scale = config.tile_size / tileset.tile_size.x;
        parent.spawn((
            Sprite {
                image: tileset.texture.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: tileset.layout.clone(),
                    index: tile_index,
                }),
                ..default()
            },
            Transform::from_xyz(position.x, position.y, Z_DECOR).with_scale(Vec3::splat(scale)),
            TerrainDecoration,
        ));
    }
}

#[allow(dead_code)]
fn spawn_shadow(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    terrain_sprites: &TerrainSprites,
    config: &TerrainConfig,
    scale_multiplier: f32,
    _tile_x: i32,
    _tile_y: i32,
) {
    let tileset = &terrain_sprites.shadow;
    let tile_index = 4;
    let scale = (config.tile_size / tileset.tile_size.x) * scale_multiplier;
    let shadow_offset = 4.0 * scale_multiplier;
    let shadow_pos = position + Vec3::new(shadow_offset, -shadow_offset, 0.0);

    parent.spawn((
        Sprite {
            image: tileset.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: tileset.layout.clone(),
                index: tile_index,
            }),
            color: Color::srgba(0.0, 0.0, 0.0, 0.35),
            ..default()
        },
        Transform::from_xyz(shadow_pos.x, shadow_pos.y, Z_SHADOW).with_scale(Vec3::splat(scale)),
        TerrainDecoration,
    ));
}

fn spawn_tileset_decoration(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    tileset: &TerrainTileset,
    config: &TerrainConfig,
    tile_index: usize,
    scale_multiplier: f32,
) {
    let scale = (config.tile_size / tileset.tile_size.x) * scale_multiplier;
    parent.spawn((
        Sprite {
            image: tileset.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: tileset.layout.clone(),
                index: tile_index,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, Z_DECOR).with_scale(Vec3::splat(scale)),
        TerrainDecoration,
    ));
}

fn spawn_tileset_obstacle(
    parent: &mut ChildSpawnerCommands,
    position: Vec3,
    tileset: &TerrainTileset,
    config: &TerrainConfig,
    tile_index: usize,
    scale_multiplier: f32,
    chunk_origin: Vec3,
) {
    let scale = (config.tile_size / tileset.tile_size.x) * scale_multiplier;
    let hitbox_size = Vec2::splat(config.tile_size * ROCK_HITBOX_SCALE * scale_multiplier);
    // Вычисляем мировые координаты (chunk_origin + local position)
    let world_position = Vec2::new(chunk_origin.x + position.x, chunk_origin.y + position.y);
    parent.spawn((
        Sprite {
            image: tileset.texture.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: tileset.layout.clone(),
                index: tile_index,
            }),
            ..default()
        },
        Transform::from_xyz(position.x, position.y, Z_DECOR).with_scale(Vec3::splat(scale)),
        TerrainDecoration,
        Hitbox::from_full_size(hitbox_size),
        CollisionLayer::obstacle(),
        PhysicsPosition(world_position),
        PreviousPhysicsPosition(world_position),
        Velocity::default(),
    ));
}

fn world_to_chunk(position: Vec2, config: &TerrainConfig) -> IVec2 {
    let tile_x = (position.x / config.tile_size).floor() as i32;
    let tile_y = (position.y / config.tile_size).floor() as i32;
    IVec2::new(
        div_floor(tile_x, config.chunk_size),
        div_floor(tile_y, config.chunk_size),
    )
}

fn div_floor(value: i32, divisor: i32) -> i32 {
    let mut result = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && ((remainder < 0) ^ (divisor < 0)) {
        result -= 1;
    }
    result
}

fn pick_tileset<'a>(
    tilesets: &'a [TerrainTileset],
    tile_x: i32,
    tile_y: i32,
    seed: u32,
    value: f32,
) -> &'a TerrainTileset {
    if tilesets.len() == 1 {
        return &tilesets[0];
    }

    let mut index = (value * tilesets.len() as f32).floor() as usize;
    let jitter = hash_f32(tile_x, tile_y, seed.wrapping_add(55));
    if jitter > 0.75 && index + 1 < tilesets.len() {
        index += 1;
    }
    if jitter < 0.15 && index > 0 {
        index -= 1;
    }
    &tilesets[index.min(tilesets.len() - 1)]
}

fn pick_simple_tileset<'a>(
    tilesets: &'a [TerrainTileset],
    tile_x: i32,
    tile_y: i32,
    seed: u32,
) -> &'a TerrainTileset {
    let index = pick_index_from_len(tilesets.len(), tile_x, tile_y, seed);
    &tilesets[index]
}

fn pick_tile_index(tileset: &TerrainTileset, tile_x: i32, tile_y: i32, seed: u32) -> usize {
    pick_index_from_len(tileset.tile_count(), tile_x, tile_y, seed)
}

fn pick_index_from_len(len: usize, tile_x: i32, tile_y: i32, seed: u32) -> usize {
    if len == 0 {
        return 0;
    }
    let roll = hash_f32(tile_x, tile_y, seed);
    let index = (roll * len as f32).floor() as usize;
    index.min(len - 1)
}

fn is_shoreline(tile_x: i32, tile_y: i32, config: &TerrainConfig) -> bool {
    let current = water_value(tile_x, tile_y, config);
    if current >= config.water_level {
        return false;
    }

    let neighbors = [
        (1, 0),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ];

    neighbors
        .iter()
        .any(|(dx, dy)| water_value(tile_x + dx, tile_y + dy, config) >= config.water_level)
}

fn water_value(tile_x: i32, tile_y: i32, config: &TerrainConfig) -> f32 {
    let mut amplitude = 0.7;
    let mut frequency = 0.02;
    let mut value = 0.0;
    let mut max = 0.0;

    for i in 0..3_u32 {
        let sample_x = tile_x as f32 * frequency;
        let sample_y = tile_y as f32 * frequency;
        value += amplitude * value_noise(sample_x, sample_y, config.seed.wrapping_add(i * 101));
        max += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if max > 0.0 {
        value / max
    } else {
        0.0
    }
}

fn biome_value(tile_x: i32, tile_y: i32, config: &TerrainConfig) -> f32 {
    let base_frequency = 0.012;
    let mut amplitude = 0.75;
    let mut frequency = base_frequency;
    let mut value = 0.0;
    let mut max = 0.0;

    for i in 0..3_u32 {
        let sample_x = tile_x as f32 * frequency;
        let sample_y = tile_y as f32 * frequency;
        value += amplitude * value_noise(sample_x, sample_y, config.seed.wrapping_add(i * 211));
        max += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if max > 0.0 {
        value / max
    } else {
        0.0
    }
}

fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let sx = fade(x - x0 as f32);
    let sy = fade(y - y0 as f32);

    let n00 = hash_f32(x0, y0, seed);
    let n10 = hash_f32(x1, y0, seed);
    let n01 = hash_f32(x0, y1, seed);
    let n11 = hash_f32(x1, y1, seed);

    let ix0 = lerp(n00, n10, sx);
    let ix1 = lerp(n01, n11, sx);
    lerp(ix0, ix1, sy)
}

fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hash_f32(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = seed;
    h ^= (x as u32).wrapping_mul(0x27d4_eb2d);
    h ^= (y as u32).wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h ^= h >> 16;
    (h as f32) / (u32::MAX as f32)
}

fn pick_from_indices(indices: &[usize], tile_x: i32, tile_y: i32, seed: u32) -> usize {
    if indices.is_empty() {
        return 0;
    }
    let roll = hash_f32(tile_x, tile_y, seed);
    let index = (roll * indices.len() as f32).floor() as usize;
    indices[index.min(indices.len() - 1)]
}
