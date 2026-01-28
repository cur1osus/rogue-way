use bevy::prelude::*;
use rand::Rng;

use crate::components::{FloatingText, HitFlash, Particle, TimedDespawn};
use crate::resources::ScreenShake;
use crate::ui::{AreaDamageVisual, AreaDamageVisualAssets};
use bevy::math::primitives::CircularSector;

pub fn spawn_hit_particles(commands: &mut Commands, position: Vec2, color: Color, count: u32) {
    let mut rng = rand::thread_rng();

    for _ in 0..count {
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let speed = rng.gen_range(60.0..140.0);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
        let lifetime = rng.gen_range(0.25..0.5);
        let size = rng.gen_range(2.0..4.0);

        commands.spawn((
            Particle {
                velocity,
                timer: Timer::from_seconds(lifetime, TimerMode::Once),
                start_color: color,
            },
            Sprite::from_color(color, Vec2::splat(size)),
            Transform::from_xyz(position.x, position.y, 0.6),
        ));
    }
}

pub fn spawn_area_damage_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    visuals: &AreaDamageVisualAssets,
    position: Vec2,
    radius: f32,
    direction: Vec2,
    cone_angle: f32,
) {
    if radius <= 0.0 {
        return;
    }

    let mesh = meshes.add(Mesh::from(CircularSector::from_radians(1.0, cone_angle)));
    let rotation = if direction.length_squared() > f32::EPSILON {
        Quat::from_rotation_z(direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2)
    } else {
        Quat::IDENTITY
    };

    commands.spawn((
        AreaDamageVisual,
        TimedDespawn {
            timer: Timer::from_seconds(0.3, TimerMode::Once),
        },
        Mesh2d(mesh),
        MeshMaterial2d(visuals.material.clone()),
        Transform::from_xyz(position.x, position.y, 0.22)
            .with_rotation(rotation)
            .with_scale(Vec3::splat(radius)),
        GlobalTransform::default(),
        Visibility::Visible,
        InheritedVisibility::default(),
        ViewVisibility::default(),
    ));
}

pub fn particle_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut Sprite, &mut Particle)>,
) {
    for (entity, mut transform, mut sprite, mut particle) in query.iter_mut() {
        particle.timer.tick(time.delta());
        transform.translation += (particle.velocity * time.delta_secs()).extend(0.0);

        let t = (particle.timer.elapsed_secs() / particle.timer.duration().as_secs_f32()).min(1.0);
        let srgba = particle.start_color.to_srgba();
        sprite.color = Color::srgba(srgba.red, srgba.green, srgba.blue, srgba.alpha * (1.0 - t));

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn floating_text_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut TextColor, &mut FloatingText)>,
) {
    for (entity, mut transform, mut text_color, mut floating) in query.iter_mut() {
        floating.timer.tick(time.delta());
        transform.translation += (floating.velocity * time.delta_secs()).extend(0.0);

        let t = (floating.timer.elapsed_secs() / floating.timer.duration().as_secs_f32()).min(1.0);
        let srgba = text_color.0.to_srgba();
        text_color.0 = Color::srgba(srgba.red, srgba.green, srgba.blue, srgba.alpha * (1.0 - t));

        if floating.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn hit_flash_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut HitFlash)>,
) {
    for (entity, mut sprite, mut flash) in query.iter_mut() {
        flash.timer.tick(time.delta());

        if flash.timer.is_finished() {
            sprite.color = flash.original_color;
            commands.entity(entity).queue_silenced(
                |mut entity: bevy::ecs::world::EntityWorldMut| {
                    entity.remove::<HitFlash>();
                },
            );
        }
    }
}

pub fn timed_despawn_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut TimedDespawn)>,
) {
    for (entity, mut timed) in query.iter_mut() {
        timed.timer.tick(time.delta());
        if timed.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn screen_shake_system(time: Res<Time>, mut shake: ResMut<ScreenShake>) {
    if shake.intensity <= 0.0 {
        shake.offset = Vec2::ZERO;
        return;
    }

    shake.timer.tick(time.delta());

    if shake.timer.is_finished() {
        shake.intensity = 0.0;
        shake.offset = Vec2::ZERO;
        return;
    }

    let progress =
        1.0 - (shake.timer.elapsed_secs() / shake.timer.duration().as_secs_f32()).min(1.0);
    let current_intensity = shake.intensity * progress;

    let mut rng = rand::thread_rng();
    let angle = rng.gen_range(0.0..std::f32::consts::TAU);
    let radius = rng.gen_range(0.0..current_intensity);
    shake.offset = Vec2::new(angle.cos(), angle.sin()) * radius;
}
