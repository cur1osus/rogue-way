// УДАЛЕНО: Система разделения сущностей по хитбоксам
// Функциональность перенесена в resolve_collisions() внутри physics_update_system
// (src/systems/physics.rs), который работает с PhysicsPosition вместо Transform
// и использует fixed timestep для более стабильной физики

// pub fn entity_collision_system(
//     collider_query: Query<(Entity, &GlobalTransform, &Hitbox, &CollisionLayer)>,
//     mut transform_query: Query<&mut Transform>,
// ) {
//     ... (код удален)
// }
