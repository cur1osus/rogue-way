use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct UiFonts {
    pub main: Handle<Font>,
}

impl FromWorld for UiFonts {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let main = asset_server.load("fonts/VMVSegaGenesis-Regular.otf");
        Self { main }
    }
}
