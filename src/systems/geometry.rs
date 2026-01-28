use bevy::prelude::Vec2;

pub fn is_within_cone(direction: Vec2, offset: Vec2, cone_angle: f32) -> bool {
    if cone_angle >= std::f32::consts::TAU {
        return true;
    }

    let direction_len_sq = direction.length_squared();
    let offset_len_sq = offset.length_squared();

    if direction_len_sq <= f32::EPSILON || offset_len_sq <= f32::EPSILON {
        return true;
    }

    let direction_norm = direction / direction_len_sq.sqrt();
    let offset_norm = offset / offset_len_sq.sqrt();
    let cos_threshold = (cone_angle * 0.5).cos();

    direction_norm.dot(offset_norm) >= cos_threshold
}
