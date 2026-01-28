use bevy::prelude::*;

const ATTACK_PATHS: [&str; 4] = [
    "sprites/attack/NPT100.png",
    "sprites/attack/NPT101.png",
    "sprites/attack/NPT102.png",
    "sprites/attack/NPT103.png",
];

/// Размер одного кадра атаки (512x432)
pub const PLAYER_ATTACK_FRAME_SIZE: Vec2 = Vec2::new(512.0, 432.0);

/// Ресурс со спрайтами атаки отталкивания (4 кадра)
#[derive(Resource, Clone)]
pub struct PlayerAttackSprites {
    pub frames: [Handle<Image>; 4],
    pub frame_size: Vec2,
}

impl FromWorld for PlayerAttackSprites {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|_world, asset_server: Mut<AssetServer>| {
            let frames = ATTACK_PATHS.map(|p| asset_server.load(p.to_string()));
            Self {
                frames,
                frame_size: PLAYER_ATTACK_FRAME_SIZE,
            }
        })
    }
}
