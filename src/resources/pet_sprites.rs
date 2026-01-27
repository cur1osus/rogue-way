use crate::components::PetType;
use bevy::prelude::*;

#[derive(Clone)]
pub struct PetAnimationSheet {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub first: usize,
    pub last: usize,
    pub frame_size: Vec2,
}

/// Ресурс для хранения спрайтов питомцев
#[derive(Resource, Clone)]
pub struct PetSpriteSheet {
    // Текстурные атласы для каждого типа питомца
    pub guard_dog_idle: PetAnimationSheet,
    pub guard_dog_run: PetAnimationSheet,
    pub guard_dog_attack: PetAnimationSheet,

    pub fire_sprite_idle: PetAnimationSheet,
    pub fire_sprite_shoot: PetAnimationSheet,
    pub arrow: PetAnimationSheet,

    pub slime_idle: PetAnimationSheet,
    pub slime_heal: PetAnimationSheet,
    pub heal_effect: PetAnimationSheet,

    pub crow_idle: PetAnimationSheet,
    pub crow_run: PetAnimationSheet,
}

impl FromWorld for PetSpriteSheet {
    fn from_world(world: &mut World) -> Self {
        world.resource_scope(|world, asset_server: Mut<AssetServer>| {
            let mut texture_atlas_layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();

            Self {
                // Guard Dog = Blue Warrior
                guard_dog_idle: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Warrior/Warrior_Idle.png",
                    Vec2::new(192.0, 192.0),
                    8,
                ),
                guard_dog_run: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Warrior/Warrior_Run.png",
                    Vec2::new(192.0, 192.0),
                    6,
                ),
                guard_dog_attack: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Warrior/Warrior_Attack1.png",
                    Vec2::new(192.0, 192.0),
                    4,
                ),

                // Fire Sprite = Blue Archer
                fire_sprite_idle: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Archer/Archer_Idle.png",
                    Vec2::new(192.0, 192.0),
                    6,
                ),
                fire_sprite_shoot: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Archer/Archer_Shoot.png",
                    Vec2::new(192.0, 192.0),
                    8,
                ),
                arrow: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Archer/Arrow.png",
                    Vec2::new(64.0, 64.0),
                    1,
                ),

                // Slime = Blue Monk
                slime_idle: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Monk/Idle.png",
                    Vec2::new(192.0, 192.0),
                    6,
                ),
                slime_heal: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Monk/Heal.png",
                    Vec2::new(192.0, 192.0),
                    11,
                ),
                heal_effect: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Monk/Heal_Effect.png",
                    Vec2::new(192.0, 192.0),
                    11,
                ),

                // Crow Scout = Blue Lancer
                crow_idle: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Lancer/Lancer_Idle.png",
                    Vec2::new(320.0, 320.0),
                    12,
                ),
                crow_run: build_pet_sheet(
                    &asset_server,
                    &mut texture_atlas_layouts,
                    "sprites/Blue Units/Lancer/Lancer_Run.png",
                    Vec2::new(320.0, 320.0),
                    6,
                ),
            }
        })
    }
}

fn build_pet_sheet(
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    path: &str,
    frame_size: Vec2,
    columns: u32,
) -> PetAnimationSheet {
    let texture = asset_server.load(path.to_string());
    let layout = TextureAtlasLayout::from_grid(
        UVec2::new(frame_size.x as u32, frame_size.y as u32),
        columns,
        1,
        None,
        None,
    );
    let layout_handle = texture_atlas_layouts.add(layout);

    PetAnimationSheet {
        texture,
        layout: layout_handle,
        first: 0,
        last: (columns - 1) as usize,
        frame_size,
    }
}

impl PetSpriteSheet {
    /// Получить idle спрайт для типа питомца
    pub fn get_idle_sheet(&self, pet_type: &PetType) -> &PetAnimationSheet {
        match pet_type {
            PetType::GuardDog => &self.guard_dog_idle,
            PetType::FireSprite => &self.fire_sprite_idle,
            PetType::SlimeCompanion => &self.slime_idle,
            PetType::CrowScout => &self.crow_idle,
        }
    }

    /// Получить run спрайт для типа питомца
    pub fn get_run_sheet(&self, pet_type: &PetType) -> &PetAnimationSheet {
        match pet_type {
            PetType::GuardDog => &self.guard_dog_run,
            PetType::FireSprite => &self.fire_sprite_shoot,
            PetType::SlimeCompanion => &self.slime_heal,
            PetType::CrowScout => &self.crow_run,
        }
    }
}

/// Ресурс для эффектов частиц
#[derive(Resource)]
pub struct ParticleEffects {
    pub fire_01: Handle<Image>,
    pub fire_02: Handle<Image>,
    pub fire_03: Handle<Image>,
    pub explosion_01: Handle<Image>,
    pub explosion_02: Handle<Image>,
    pub dust_01: Handle<Image>,
    pub dust_02: Handle<Image>,
    pub water_splash: Handle<Image>,
}

impl FromWorld for ParticleEffects {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();

        Self {
            fire_01: asset_server.load("sprites/Particle FX/Fire_01.png"),
            fire_02: asset_server.load("sprites/Particle FX/Fire_02.png"),
            fire_03: asset_server.load("sprites/Particle FX/Fire_03.png"),
            explosion_01: asset_server.load("sprites/Particle FX/Explosion_01.png"),
            explosion_02: asset_server.load("sprites/Particle FX/Explosion_02.png"),
            dust_01: asset_server.load("sprites/Particle FX/Dust_01.png"),
            dust_02: asset_server.load("sprites/Particle FX/Dust_02.png"),
            water_splash: asset_server.load("sprites/Particle FX/Water Splash.png"),
        }
    }
}
