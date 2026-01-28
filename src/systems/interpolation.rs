use bevy::prelude::*;

use crate::components::{PhysicsPosition, PreviousPhysicsPosition};
use crate::resources::{PhysicsAccumulator, FIXED_TIMESTEP};

/// Интерполирует визуальную позицию (Transform) между физическими кадрами
pub fn interpolation_system(
    accumulator: Res<PhysicsAccumulator>,
    mut query: Query<
        (&PhysicsPosition, &PreviousPhysicsPosition, &mut Transform),
        Without<ChildOf>,
    >,
) {
    // Вычисляем коэффициент интерполяции (0.0 = предыдущая позиция, 1.0 = текущая)
    let alpha = (accumulator.accumulator / FIXED_TIMESTEP).clamp(0.0, 1.0);

    for (physics_pos, prev_pos, mut transform) in query.iter_mut() {
        // Линейная интерполяция между предыдущей и текущей физической позицией
        let interpolated = prev_pos.0.lerp(physics_pos.0, alpha);

        // Обновляем визуальную позицию
        transform.translation.x = interpolated.x;
        transform.translation.y = interpolated.y;
    }
}
