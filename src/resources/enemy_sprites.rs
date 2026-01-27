use bevy::prelude::*;

#[derive(Clone)]
pub struct EnemyAnimationSheet {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub first: usize,
    pub last: usize,
    pub frame_size: Vec2,
}

#[derive(Resource, Clone)]
pub struct EnemySpriteSheet {
    pub run: EnemyAnimationSheet,
    pub attack: EnemyAnimationSheet,
}

impl FromWorld for EnemySpriteSheet {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, asset_server: Mut<AssetServer>| {
            let mut texture_atlas_layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
            Self {
                run: build_enemy_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/mushroom-run.png",
                    Vec2::new(80.0, 40.0),
                    8,
                ),
                attack: build_enemy_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/mushroom-attack.png",
                    Vec2::new(80.0, 36.0),
                    10,
                ),
            }
        })
    }
}

fn build_enemy_sheet(
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    path: &str,
    frame_size: Vec2,
    columns: u32,
) -> EnemyAnimationSheet {
    let texture = asset_server.load(path.to_string());
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(frame_size.x as u32, frame_size.y as u32),
        columns,
        1,
        None,
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);

    EnemyAnimationSheet {
        texture,
        layout: layout_handle,
        first: 0,
        last: (columns - 1) as usize,
        frame_size,
    }
}
