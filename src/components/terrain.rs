use bevy::prelude::*;

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TerrainChunk {
    pub coords: IVec2,
}

#[derive(Component)]
pub struct TerrainTile;

#[derive(Component)]
pub struct TerrainDecoration;

#[allow(dead_code)]
#[derive(Component)]
pub struct TerrainCloud;
