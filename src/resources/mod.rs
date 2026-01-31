pub mod effects;
pub mod enemy_sprites;
pub mod gold_sprites;
pub mod meta_progression;
pub mod pet_sprites;
pub mod player_attack_sprites;
pub mod terrain;
pub mod ui_fonts;
pub mod upgrade_state;
pub mod wave_config;
pub mod xp_sprites;

pub use effects::*;
pub use enemy_sprites::*;
pub use gold_sprites::*;
pub use meta_progression::*;
pub use pet_sprites::*;
pub use player_attack_sprites::*;
pub use terrain::*;
pub use ui_fonts::*;
pub use upgrade_state::*;
pub use wave_config::*;
pub use xp_sprites::*;

use bevy::prelude::*;

/// Накопитель времени для fixed timestep физики
/// Обновляется в Update для интерполяции между fixed timesteps
#[derive(Resource, Default)]
pub struct PhysicsAccumulator {
    pub accumulator: f32,
}

/// Фиксированный временной шаг для физики (30 Hz - соответствует FixedUpdate)
pub const FIXED_TIMESTEP: f32 = 1.0 / 30.0;
