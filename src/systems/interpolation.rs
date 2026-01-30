use bevy::prelude::*;

use crate::components::{LocalPlayer, PhysicsPosition, PreviousPhysicsPosition, Velocity};
use crate::network::{NetworkInterpolation, NetworkMode};
use crate::resources::{PhysicsAccumulator, FIXED_TIMESTEP};

/// Интерполирует визуальную позицию (Transform) между физическими кадрами
pub fn interpolation_system(
    mode: Option<Res<NetworkMode>>,
    accumulator: Res<PhysicsAccumulator>,
    net_interp: Option<Res<NetworkInterpolation>>,
    mut query: Query<
        (
            &PhysicsPosition,
            &PreviousPhysicsPosition,
            Option<&Velocity>,
            &mut Transform,
            Option<&LocalPlayer>,
        ),
        Without<ChildOf>,
    >,
) {
    let is_client = matches!(mode.map(|m| *m), Some(NetworkMode::Client));
    let (alpha, extrapolation_time) = if is_client {
        let interp = net_interp.map(|interp| *interp).unwrap_or_default();
        let alpha = (interp.accumulator / interp.interval).clamp(0.0, 1.0);
        let extra = (interp.accumulator - interp.interval).clamp(0.0, interp.max_extrapolation);
        (alpha, extra)
    } else {
        let alpha = (accumulator.accumulator / FIXED_TIMESTEP).clamp(0.0, 1.0);
        (alpha, 0.0)
    };

    for (physics_pos, prev_pos, velocity, mut transform, local_player) in query.iter_mut() {
        if is_client && local_player.is_some() {
            continue;
        }
        // Линейная интерполяция между предыдущей и текущей физической позицией
        let mut interpolated = prev_pos.0.lerp(physics_pos.0, alpha);
        if is_client {
            if let Some(velocity) = velocity {
                interpolated += velocity.0 * extrapolation_time;
            }
        }

        // Обновляем визуальную позицию
        transform.translation.x = interpolated.x;
        transform.translation.y = interpolated.y;
    }
}
