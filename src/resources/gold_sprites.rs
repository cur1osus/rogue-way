use bevy::prelude::*;

/// Ресурс для хранения спрайтов золота
#[derive(Resource, Clone)]
pub struct GoldSprites {
    // Обычная монета
    pub normal_texture: Handle<Image>,
    pub normal_layout: Handle<TextureAtlasLayout>,
    pub normal_index: usize,

    // Сверкающая монета (анимация)
    pub highlight_texture: Handle<Image>,
    pub highlight_layout: Handle<TextureAtlasLayout>,
    pub highlight_first: usize,
    pub highlight_last: usize,

    pub frame_size: Vec2,
}

impl FromWorld for GoldSprites {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, asset_server: Mut<AssetServer>| {
            let mut texture_atlas_layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();

            // Обычная монета (один кадр 128x128)
            let normal_texture = asset_server
                .load("sprites/Gold/Gold_Resource.png".to_string());
            let normal_layout =
                TextureAtlasLayout::from_grid(UVec2::new(128, 128), 1, 1, None, None);
            let normal_layout_handle = texture_atlas_layouts.add(normal_layout);

            // Сверкающая монета (6 кадров анимации 128x128 каждый)
            let highlight_texture = asset_server.load(
                "sprites/Gold/Gold_Resource_Highlight.png"
                    .to_string(),
            );
            let highlight_layout =
                TextureAtlasLayout::from_grid(UVec2::new(128, 128), 6, 1, None, None);
            let highlight_layout_handle = texture_atlas_layouts.add(highlight_layout);

            Self {
                normal_texture,
                normal_layout: normal_layout_handle,
                normal_index: 0,
                highlight_texture,
                highlight_layout: highlight_layout_handle,
                highlight_first: 0,
                highlight_last: 5,
                frame_size: Vec2::new(128.0, 128.0),
            }
        })
    }
}
