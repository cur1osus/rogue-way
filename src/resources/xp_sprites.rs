use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct XpGemSprites {
    pub texture: Handle<Image>,
}

impl FromWorld for XpGemSprites {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let texture = asset_server.load("sprites/xp.png".to_string());

        Self { texture }
    }
}
