use bevy::asset::RenderAssetUsages;
use bevy::ecs::hierarchy::ChildOf;
use bevy::math::primitives::CircularSector;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::components::{
    Enemy, KnockbackEffect, LocalPlayer, PhysicsPosition, Player, PlayerInputState,
    PlayerPushbackOverlay, PushbackAttack, PushbackAttackCooldown, PushbackConeVisual,
    PushbackReadyGlow, PushbackReadyGlowPending, Team, Velocity,
};
use crate::constants::{
    PUSHBACK_OVERLAY_OFFSET, PUSHBACK_OVERLAY_SCALE, PUSHBACK_READY_GLOW_INNER_ALPHA,
    PUSHBACK_READY_GLOW_INNER_RADIUS, PUSHBACK_READY_GLOW_OUTER_ALPHA,
    PUSHBACK_READY_GLOW_OUTER_RADIUS, PUSHBACK_READY_GLOW_ROUNDING_ALPHA,
    PUSHBACK_READY_GLOW_ROUNDING_OFFSET, PUSHBACK_READY_GLOW_SCANLINE_ALPHA,
    PUSHBACK_READY_GLOW_SCANLINE_STEP, PUSHBACK_READY_GLOW_TINT,
};
use crate::resources::PlayerAttackSprites;
use crate::systems::geometry::is_within_cone;
use crate::ui::{AreaDamageVisualAssets, HitboxVisualsVisible};

const KNOCKBACK_DECAY_PER_SEC: f32 = 5.0;
const ATTACK_FRAME_DURATION: f32 = 0.08;
const ATTACK_FRAME_COUNT: usize = 4;

struct GlowLayer {
    radius: i32,
    alpha: f32,
}

/// Обновление last_direction из Velocity (влево/вправо/вверх/вниз).
pub fn pushback_last_direction_system(
    mut query: Query<(&Velocity, &mut PushbackAttack), With<Player>>,
) {
    for (velocity, mut pushback) in query.iter_mut() {
        if velocity.0.length_squared() > 0.01 {
            pushback.last_direction = velocity.0.normalize_or_zero();
        }
    }
}

/// Обработка ввода пробела: запуск атаки отталкивания, применение импульса врагам в конусе 120°.
pub fn pushback_input_system(
    time: Res<Time>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    area_visuals: Res<AreaDamageVisualAssets>,
    mut player_query: Query<
        (
            Entity,
            &Transform,
            &mut PlayerInputState,
            &mut PushbackAttack,
            &mut PushbackAttackCooldown,
        ),
        With<Player>,
    >,
    enemy_query: Query<(Entity, &Transform, &Team), With<Enemy>>,
    attack_sprites: Res<PlayerAttackSprites>,
) {
    for (player_entity, player_transform, mut input_state, mut pushback, mut cooldown) in
        player_query.iter_mut()
    {
        cooldown.timer.tick(time.delta());

        if pushback.is_attacking {
            continue;
        }
        if !input_state.pushback {
            continue;
        }
        input_state.pushback = false;
        if !cooldown.timer.is_finished() {
            continue;
        }

        let dir = pushback.last_direction;
        if dir.length_squared() < 0.01 {
            continue;
        }
        let dir_norm = dir.normalize_or_zero();

        pushback.is_attacking = true;
        pushback.animation_timer = Timer::from_seconds(
            ATTACK_FRAME_COUNT as f32 * ATTACK_FRAME_DURATION,
            TimerMode::Once,
        );
        pushback.animation_timer.reset();
        pushback.current_frame = 0;
        cooldown.timer.reset();

        let pos = player_transform.translation.truncate();

        for (enemy_entity, enemy_transform, team) in enemy_query.iter() {
            if team.0 == Team::PLAYER {
                continue;
            }
            let offset = enemy_transform.translation.truncate() - pos;
            let dist = offset.length();
            if dist > pushback.radius || dist < 0.01 {
                continue;
            }
            if !is_within_cone(dir_norm, offset, pushback.cone_angle) {
                continue;
            }
            let push_dir = offset.normalize_or_zero();
            commands.entity(enemy_entity).insert(KnockbackEffect {
                remaining: push_dir * pushback.pushback_force,
                decay_per_second: KNOCKBACK_DECAY_PER_SEC,
            });
        }

        let offset_vec = dir_norm * PUSHBACK_OVERLAY_OFFSET;
        let size = attack_sprites.frame_size * PUSHBACK_OVERLAY_SCALE;
        let angle = dir.y.atan2(dir.x);
        let rot = Quat::from_rotation_z(angle);

        let overlay = commands
            .spawn((
                Sprite {
                    image: attack_sprites.frames[0].clone(),
                    custom_size: Some(size),
                    flip_x: false,
                    ..default()
                },
                Transform::from_xyz(offset_vec.x, offset_vec.y, 1.5).with_rotation(rot),
                PlayerPushbackOverlay,
            ))
            .id();
        commands.entity(player_entity).add_child(overlay);

        let cone_mesh = meshes.add(Mesh::from(CircularSector::from_radians(
            1.0,
            pushback.cone_angle,
        )));
        let cone_rot = Quat::from_rotation_z(angle - std::f32::consts::FRAC_PI_2);
        let cone = commands
            .spawn((
                PushbackConeVisual,
                Mesh2d(cone_mesh),
                MeshMaterial2d(area_visuals.material.clone()),
                Transform::from_xyz(0.0, 0.0, 0.2)
                    .with_rotation(cone_rot)
                    .with_scale(Vec3::splat(pushback.radius)),
                GlobalTransform::default(),
                Visibility::Hidden,
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();
        commands.entity(player_entity).add_child(cone);
    }
}

fn add_glow_from_point(
    accum_rgb: &mut [f32],
    accum_alpha: &mut [f32],
    image_width: i32,
    frame_min: IVec2,
    frame_max: IVec2,
    point: Vec2,
    color: Vec3,
    base_alpha: f32,
    layers: &[GlowLayer],
) {
    for layer in layers {
        if layer.radius <= 0 || layer.alpha <= 0.0 {
            continue;
        }
        let radius = layer.radius as f32;
        let min_x = ((point.x - radius).floor() as i32).max(frame_min.x);
        let max_x = ((point.x + radius).ceil() as i32).min(frame_max.x - 1);
        let min_y = ((point.y - radius).floor() as i32).max(frame_min.y);
        let max_y = ((point.y + radius).ceil() as i32).min(frame_max.y - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = (x as f32 + 0.5) - point.x;
                let dy = (y as f32 + 0.5) - point.y;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > radius {
                    continue;
                }
                let weight = (1.0 - dist / radius) * layer.alpha * base_alpha;
                if weight <= 0.0 {
                    continue;
                }

                let idx = (y as usize * image_width as usize + x as usize) as usize;
                accum_rgb[idx * 3] += color.x * weight;
                accum_rgb[idx * 3 + 1] += color.y * weight;
                accum_rgb[idx * 3 + 2] += color.z * weight;
                accum_alpha[idx] += weight;
            }
        }
    }
}

fn generate_pushback_glow_image(
    source: &Image,
    layout: &TextureAtlasLayout,
) -> Option<(Image, TextureAtlasLayout)> {
    let size = source.texture_descriptor.size;
    let source_width = size.width as i32;
    let source_height = size.height as i32;
    if source_width <= 0 || source_height <= 0 {
        return None;
    }

    let format = source.texture_descriptor.format;
    if format != TextureFormat::Rgba8UnormSrgb && format != TextureFormat::Rgba8Unorm {
        return None;
    }

    let data = source.data.as_ref()?;
    if data.len() != (source_width as usize * source_height as usize * 4) {
        return None;
    }

    let padding = PUSHBACK_READY_GLOW_OUTER_RADIUS
        .max(PUSHBACK_READY_GLOW_INNER_RADIUS)
        .max(1);

    let mut total_width = 0;
    let mut max_height = 0;
    for rect in &layout.textures {
        let frame_w = rect.max.x as i32 - rect.min.x as i32;
        let frame_h = rect.max.y as i32 - rect.min.y as i32;
        if frame_w <= 0 || frame_h <= 0 {
            return None;
        }
        total_width += frame_w + padding * 2;
        max_height = max_height.max(frame_h + padding * 2);
    }
    if total_width <= 0 || max_height <= 0 {
        return None;
    }

    let mut glow_layout =
        TextureAtlasLayout::new_empty(UVec2::new(total_width as u32, max_height as u32));

    struct FrameInfo {
        src_min: IVec2,
        dst_min: IVec2,
        frame_w: i32,
        frame_h: i32,
    }

    let mut frames = Vec::with_capacity(layout.textures.len());
    let mut cursor_x = 0;
    for rect in &layout.textures {
        let frame_w = rect.max.x as i32 - rect.min.x as i32;
        let frame_h = rect.max.y as i32 - rect.min.y as i32;
        let dst_min = IVec2::new(cursor_x, 0);
        let dst_max = IVec2::new(cursor_x + frame_w + padding * 2, frame_h + padding * 2);
        glow_layout.add_texture(URect {
            min: UVec2::new(dst_min.x as u32, dst_min.y as u32),
            max: UVec2::new(dst_max.x as u32, dst_max.y as u32),
        });
        frames.push(FrameInfo {
            src_min: IVec2::new(rect.min.x as i32, rect.min.y as i32),
            dst_min,
            frame_w,
            frame_h,
        });
        cursor_x = dst_max.x;
    }

    let out_width = total_width;
    let out_height = max_height;
    let mut accum_rgb = vec![0.0f32; out_width as usize * out_height as usize * 3];
    let mut accum_alpha = vec![0.0f32; out_width as usize * out_height as usize];

    let layers = [
        GlowLayer {
            radius: PUSHBACK_READY_GLOW_INNER_RADIUS,
            alpha: PUSHBACK_READY_GLOW_INNER_ALPHA,
        },
        GlowLayer {
            radius: PUSHBACK_READY_GLOW_OUTER_RADIUS,
            alpha: PUSHBACK_READY_GLOW_OUTER_ALPHA,
        },
    ];

    for frame in &frames {
        let frame_min = frame.src_min;
        let frame_w = frame.frame_w;
        let frame_h = frame.frame_h;

        let mut mask = vec![false; (frame_w * frame_h) as usize];
        for y in 0..frame_h {
            for x in 0..frame_w {
                let gx = frame_min.x + x;
                let gy = frame_min.y + y;
                let idx = (gy as usize * source_width as usize + gx as usize) * 4;
                let alpha = data[idx + 3];
                if alpha > 0 {
                    let m_idx = (y * frame_w + x) as usize;
                    mask[m_idx] = true;
                }
            }
        }

        for y in 0..frame_h {
            for x in 0..frame_w {
                let m_idx = (y * frame_w + x) as usize;
                if !mask[m_idx] {
                    continue;
                }

                let gx = frame_min.x + x;
                let gy = frame_min.y + y;
                let idx = (gy as usize * source_width as usize + gx as usize) * 4;
                let r = data[idx] as f32 / 255.0;
                let g = data[idx + 1] as f32 / 255.0;
                let b = data[idx + 2] as f32 / 255.0;
                let a = data[idx + 3] as f32 / 255.0;
                if a <= 0.0 {
                    continue;
                }

                let dst_x = frame.dst_min.x + padding + x;
                let dst_y = frame.dst_min.y + padding + y;
                let point = Vec2::new(dst_x as f32 + 0.5, dst_y as f32 + 0.5);
                let color = Vec3::new(r, g, b);
                let dst_frame_min = frame.dst_min;
                let dst_frame_max = IVec2::new(
                    frame.dst_min.x + frame_w + padding * 2,
                    frame.dst_min.y + frame_h + padding * 2,
                );
                add_glow_from_point(
                    &mut accum_rgb,
                    &mut accum_alpha,
                    out_width,
                    dst_frame_min,
                    dst_frame_max,
                    point,
                    color,
                    a,
                    &layers,
                );

                let left_empty = x == 0 || !mask[(y * frame_w + (x - 1)) as usize];
                let right_empty = x + 1 >= frame_w || !mask[(y * frame_w + (x + 1)) as usize];
                let rounding_alpha = a * PUSHBACK_READY_GLOW_ROUNDING_ALPHA;

                if left_empty {
                    let offset = PUSHBACK_READY_GLOW_ROUNDING_OFFSET;
                    add_glow_from_point(
                        &mut accum_rgb,
                        &mut accum_alpha,
                        out_width,
                        dst_frame_min,
                        dst_frame_max,
                        Vec2::new(point.x - offset, point.y - offset),
                        color,
                        rounding_alpha,
                        &layers,
                    );
                    add_glow_from_point(
                        &mut accum_rgb,
                        &mut accum_alpha,
                        out_width,
                        dst_frame_min,
                        dst_frame_max,
                        Vec2::new(point.x - offset, point.y + offset),
                        color,
                        rounding_alpha,
                        &layers,
                    );
                }

                if right_empty {
                    let offset = PUSHBACK_READY_GLOW_ROUNDING_OFFSET;
                    add_glow_from_point(
                        &mut accum_rgb,
                        &mut accum_alpha,
                        out_width,
                        dst_frame_min,
                        dst_frame_max,
                        Vec2::new(point.x + offset, point.y - offset),
                        color,
                        rounding_alpha,
                        &layers,
                    );
                    add_glow_from_point(
                        &mut accum_rgb,
                        &mut accum_alpha,
                        out_width,
                        dst_frame_min,
                        dst_frame_max,
                        Vec2::new(point.x + offset, point.y + offset),
                        color,
                        rounding_alpha,
                        &layers,
                    );
                }
            }
        }
    }

    if PUSHBACK_READY_GLOW_SCANLINE_ALPHA > 0.0 && PUSHBACK_READY_GLOW_SCANLINE_STEP > 0 {
        let factor = (1.0 - PUSHBACK_READY_GLOW_SCANLINE_ALPHA).clamp(0.0, 1.0);
        for y in 0..out_height {
            if ((y as u32 / PUSHBACK_READY_GLOW_SCANLINE_STEP) % 2) == 1 {
                for x in 0..out_width {
                    let idx = (y as usize * out_width as usize + x as usize) as usize;
                    accum_rgb[idx * 3] *= factor;
                    accum_rgb[idx * 3 + 1] *= factor;
                    accum_rgb[idx * 3 + 2] *= factor;
                    accum_alpha[idx] *= factor;
                }
            }
        }
    }

    let tint = PUSHBACK_READY_GLOW_TINT.to_srgba();
    let mut out = vec![0u8; out_width as usize * out_height as usize * 4];
    for y in 0..out_height {
        for x in 0..out_width {
            let idx = (y as usize * out_width as usize + x as usize) as usize;
            let alpha = accum_alpha[idx].min(1.0);
            if alpha <= 0.0 {
                continue;
            }

            let r = (accum_rgb[idx * 3] / alpha).clamp(0.0, 1.0) * tint.red;
            let g = (accum_rgb[idx * 3 + 1] / alpha).clamp(0.0, 1.0) * tint.green;
            let b = (accum_rgb[idx * 3 + 2] / alpha).clamp(0.0, 1.0) * tint.blue;

            let out_idx = idx * 4;
            out[out_idx] = (r.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[out_idx + 1] = (g.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[out_idx + 2] = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
            out[out_idx + 3] = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }

    let image = Image::new(
        Extent3d {
            width: out_width as u32,
            height: out_height as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        out,
        format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    Some((image, glow_layout))
}

/// Генерация текстуры свечения и привязка к дочернему спрайту.
pub fn pushback_ready_glow_setup_system(
    mut commands: Commands,
    player_query: Query<
        (Entity, &Sprite),
        (
            With<Player>,
            With<LocalPlayer>,
            Without<PushbackReadyGlowPending>,
        ),
    >,
    mut glow_query: Query<
        (Entity, &ChildOf, &mut Sprite),
        (With<PushbackReadyGlowPending>, Without<Player>),
    >,
    mut images: ResMut<Assets<Image>>,
    mut atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    if glow_query.is_empty() {
        return;
    }

    let Ok((player_entity, player_sprite)) = player_query.single() else {
        return;
    };
    let Some(player_atlas) = player_sprite.texture_atlas.as_ref() else {
        return;
    };
    let Some(layout) = atlas_layouts.get(&player_atlas.layout) else {
        return;
    };
    let Some(source_image) = images.get(&player_sprite.image).map(|image| image.clone()) else {
        return;
    };

    let Some((glow_image, glow_layout)) = generate_pushback_glow_image(&source_image, layout)
    else {
        return;
    };
    let glow_handle = images.add(glow_image);
    let glow_layout_handle = atlas_layouts.add(glow_layout);

    for (glow_entity, child_of, mut glow_sprite) in glow_query.iter_mut() {
        if child_of.0 != player_entity {
            continue;
        }
        glow_sprite.image = glow_handle.clone();
        glow_sprite.texture_atlas = Some(TextureAtlas {
            layout: glow_layout_handle.clone(),
            index: player_atlas.index,
        });
        commands
            .entity(glow_entity)
            .remove::<PushbackReadyGlowPending>();
    }
}

/// Синхронизация кадра свечения с текущим кадром игрока.
pub fn pushback_ready_glow_sync_system(
    player_query: Query<
        (Entity, &Sprite),
        (With<Player>, With<LocalPlayer>, Without<PushbackReadyGlow>),
    >,
    mut glow_query: Query<(&ChildOf, &mut Sprite), (With<PushbackReadyGlow>, Without<Player>)>,
) {
    let Ok((player_entity, player_sprite)) = player_query.single() else {
        return;
    };
    let Some(player_atlas) = player_sprite.texture_atlas.as_ref() else {
        return;
    };

    for (child_of, mut glow_sprite) in glow_query.iter_mut() {
        if child_of.0 != player_entity {
            continue;
        }
        if let Some(glow_atlas) = glow_sprite.texture_atlas.as_mut() {
            glow_atlas.index = player_atlas.index;
        }
        glow_sprite.flip_x = player_sprite.flip_x;
        glow_sprite.flip_y = player_sprite.flip_y;
    }
}

/// Управление синим свечением: видно, когда атака отталкивания готова.
pub fn pushback_ready_glow_system(
    player_query: Query<
        (Entity, &PushbackAttack, &PushbackAttackCooldown),
        (With<Player>, With<LocalPlayer>),
    >,
    mut glow_query: Query<(&ChildOf, &mut Sprite), With<PushbackReadyGlow>>,
) {
    for (player_entity, pushback, cooldown) in player_query.iter() {
        let ready = cooldown.timer.is_finished() && !pushback.is_attacking;
        let tint = PUSHBACK_READY_GLOW_TINT.to_srgba();
        let target_color = if ready {
            Color::srgba(tint.red, tint.green, tint.blue, 1.0)
        } else {
            Color::srgba(tint.red, tint.green, tint.blue, 0.0)
        };

        for (child_of, mut sprite) in glow_query.iter_mut() {
            if child_of.0 != player_entity {
                continue;
            }
            if sprite.color != target_color {
                sprite.color = target_color;
            }
        }
    }
}

/// Тик анимации атаки, снятие флага и деспавн оверлея и конуса по окончании.
pub fn pushback_animation_system(
    time: Res<Time>,
    mut commands: Commands,
    mut player_query: Query<(Entity, &mut PushbackAttack), With<Player>>,
    overlay_query: Query<(Entity, &ChildOf), With<PlayerPushbackOverlay>>,
    cone_query: Query<(Entity, &ChildOf), With<PushbackConeVisual>>,
) {
    let mut to_despawn = Vec::new();
    for (player_entity, mut pushback) in player_query.iter_mut() {
        if !pushback.is_attacking {
            continue;
        }
        pushback.animation_timer.tick(time.delta());
        let elapsed = pushback.animation_timer.elapsed_secs();
        pushback.current_frame = (elapsed / ATTACK_FRAME_DURATION).min(3.0) as usize;

        if pushback.animation_timer.is_finished() {
            pushback.is_attacking = false;
            for (e, child_of) in overlay_query.iter() {
                if child_of.0 == player_entity {
                    to_despawn.push(e);
                }
            }
            for (e, child_of) in cone_query.iter() {
                if child_of.0 == player_entity {
                    to_despawn.push(e);
                }
            }
        }
    }
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }
}

/// Обновление оверлея атаки: кадр анимации, позиция и поворот по last_direction.
pub fn pushback_overlay_system(
    player_query: Query<(Entity, &PushbackAttack), With<Player>>,
    attack_sprites: Res<PlayerAttackSprites>,
    mut overlay_query: Query<
        (Entity, &ChildOf, &mut Sprite, &mut Transform),
        With<PlayerPushbackOverlay>,
    >,
) {
    for (player_entity, pushback) in player_query.iter() {
        if !pushback.is_attacking {
            continue;
        }
        let dir = pushback.last_direction.normalize_or_zero();
        let frame = pushback.current_frame.min(3);
        let offset_vec = dir * PUSHBACK_OVERLAY_OFFSET;
        let size = attack_sprites.frame_size * PUSHBACK_OVERLAY_SCALE;
        let angle = dir.y.atan2(dir.x);
        let rot = Quat::from_rotation_z(angle);

        for (_, child_of, mut sprite, mut transform) in overlay_query.iter_mut() {
            if child_of.0 != player_entity {
                continue;
            }
            sprite.image = attack_sprites.frames[frame].clone();
            sprite.custom_size = Some(size);
            transform.translation.x = offset_vec.x;
            transform.translation.y = offset_vec.y;
            transform.translation.z = 1.5;
            transform.rotation = rot;
        }
    }
}

/// Обновление визуала конуса атаки: позиция/поворот по last_direction,
/// видимость — только при включённых хитбоксах (H).
pub fn pushback_cone_visual_system(
    hitbox_visible: Res<HitboxVisualsVisible>,
    player_query: Query<(Entity, &PushbackAttack), With<Player>>,
    mut cone_query: Query<
        (Entity, &ChildOf, &mut Transform, &mut Visibility),
        With<PushbackConeVisual>,
    >,
) {
    let visible = hitbox_visible.0;
    for (player_entity, pushback) in player_query.iter() {
        if !pushback.is_attacking {
            continue;
        }
        let dir = pushback.last_direction.normalize_or_zero();
        let angle = dir.y.atan2(dir.x) - std::f32::consts::FRAC_PI_2;

        for (_, child_of, mut transform, mut vis) in cone_query.iter_mut() {
            if child_of.0 != player_entity {
                continue;
            }
            transform.rotation = Quat::from_rotation_z(angle);
            transform.scale = Vec3::splat(pushback.radius);
            *vis = if visible {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

/// Применение отталкивания к сущностям с KnockbackEffect и затухание.
pub fn knockback_effect_system(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut PhysicsPosition, &mut KnockbackEffect)>,
) {
    let dt = time.delta_secs();
    let mut to_remove = Vec::new();
    for (entity, mut physics_pos, mut knockback) in query.iter_mut() {
        let delta = knockback.remaining * dt;

        // Применяем knockback к физической позиции
        physics_pos.0 += delta;

        let len = knockback.remaining.length();
        let decay = (knockback.decay_per_second * dt).min(1.0);
        if len > 0.01 {
            knockback.remaining *= 1.0 - decay;
            if knockback.remaining.length() < 1.0 {
                to_remove.push(entity);
            }
        } else {
            to_remove.push(entity);
        }
    }
    for entity in to_remove {
        commands.entity(entity).remove::<KnockbackEffect>();
    }
}
