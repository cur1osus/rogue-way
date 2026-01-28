use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TerrainTileset {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub columns: u32,
    pub rows: u32,
    pub tile_size: Vec2,
}

impl TerrainTileset {
    pub fn tile_count(&self) -> usize {
        (self.columns * self.rows) as usize
    }
}

#[derive(Resource, Clone)]
pub struct TerrainSprites {
    pub ground_tilesets: Vec<TerrainTileset>,
    pub water_background: Handle<Image>,
    pub water_foam: TerrainTileset,
    #[allow(dead_code)]
    pub shadow: TerrainTileset,
    pub rocks: Vec<Handle<Image>>,
    pub water_rocks: Vec<TerrainTileset>,
    pub bushes: Vec<TerrainTileset>,
    pub rubber_duck: TerrainTileset,
}

impl FromWorld for TerrainSprites {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, asset_server: Mut<AssetServer>| {
            let mut texture_atlas_layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();

            let ground_tilesets = vec![build_tileset(
                &asset_server,
                &mut texture_atlas_layouts,
                "sprites/Terrain/Tileset/Tilemap_color1.png",
                Vec2::splat(64.0),
                9,
                6,
            )];

            let water_background =
                asset_server.load("sprites/Terrain/Tileset/Water Background color.png".to_string());

            let water_foam = build_tileset(
                &asset_server,
                &mut texture_atlas_layouts,
                "sprites/Terrain/Tileset/Water Foam.png",
                Vec2::splat(64.0),
                48,
                3,
            );

            let shadow = build_tileset(
                &asset_server,
                &mut texture_atlas_layouts,
                "sprites/Terrain/Tileset/Shadow.png",
                Vec2::splat(64.0),
                3,
                3,
            );

            let rocks = [
                "sprites/Terrain/Decorations/Rocks/Rock1.png",
                "sprites/Terrain/Decorations/Rocks/Rock2.png",
                "sprites/Terrain/Decorations/Rocks/Rock3.png",
                "sprites/Terrain/Decorations/Rocks/Rock4.png",
            ]
            .into_iter()
            .map(|path| asset_server.load(path.to_string()))
            .collect();

            let water_rocks = [
                "sprites/Terrain/Decorations/Rocks in the Water/Water Rocks_01.png",
                "sprites/Terrain/Decorations/Rocks in the Water/Water Rocks_02.png",
                "sprites/Terrain/Decorations/Rocks in the Water/Water Rocks_03.png",
                "sprites/Terrain/Decorations/Rocks in the Water/Water Rocks_04.png",
            ]
            .into_iter()
            .map(|path| {
                build_tileset(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    path,
                    Vec2::splat(64.0),
                    16,
                    1,
                )
            })
            .collect();

            let bushes = [
                "sprites/Terrain/Decorations/Bushes/Bushe1.png",
                "sprites/Terrain/Decorations/Bushes/Bushe2.png",
                "sprites/Terrain/Decorations/Bushes/Bushe3.png",
                "sprites/Terrain/Decorations/Bushes/Bushe4.png",
            ]
            .into_iter()
            .map(|path| {
                build_tileset(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    path,
                    Vec2::splat(128.0),
                    8,
                    1,
                )
            })
            .collect();

            let rubber_duck = build_tileset(
                &asset_server,
                &mut texture_atlas_layouts,
                "sprites/Terrain/Decorations/Rubber Duck/Rubber duck.png",
                Vec2::splat(32.0),
                3,
                1,
            );

            Self {
                ground_tilesets,
                water_background,
                water_foam,
                shadow,
                rocks,
                water_rocks,
                bushes,
                rubber_duck,
            }
        })
    }
}

#[derive(Resource, Clone)]
pub struct TerrainConfig {
    pub tile_size: f32,
    pub chunk_size: i32,
    pub view_distance: i32,
    pub seed: u32,
    pub water_level: f32,
    pub shoreline_chance: f32,
    pub bush_chance: f32,
    pub rock_chance: f32,
    pub water_rock_chance: f32,
    pub duck_chance: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            tile_size: 64.0,
            chunk_size: 16,
            view_distance: 2,
            seed: 0x7A5F_11C3,
            water_level: 0.38,
            shoreline_chance: 0.7,
            bush_chance: 0.06,
            rock_chance: 0.04,
            water_rock_chance: 0.06,
            duck_chance: 0.01,
        }
    }
}

#[derive(Resource, Default)]
pub struct TerrainChunks {
    pub chunks: HashMap<IVec2, Entity>,
}

fn build_tileset(
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    path: &str,
    tile_size: Vec2,
    columns: u32,
    rows: u32,
) -> TerrainTileset {
    let texture = asset_server.load(path.to_string());
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(tile_size.x as u32, tile_size.y as u32),
        columns,
        rows,
        None,
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);

    TerrainTileset {
        texture,
        layout: layout_handle,
        columns,
        rows,
        tile_size,
    }
}
