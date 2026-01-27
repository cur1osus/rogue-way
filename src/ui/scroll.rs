use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

/// Маркер для скроллируемого контейнера
#[derive(Component)]
pub struct ScrollContainer {
    pub scroll_position: f32,
    pub max_scroll: f32,
}

impl Default for ScrollContainer {
    fn default() -> Self {
        Self {
            scroll_position: 0.0,
            max_scroll: 0.0,
        }
    }
}

/// Маркер для контента внутри скролла
#[derive(Component)]
pub struct ScrollContent;

/// Система обработки скролла колесом мыши
pub fn scroll_system(
    mut scroll_events: MessageReader<MouseWheel>,
    mut scroll_query: Query<(&mut ScrollContainer, &ComputedNode, &Children)>,
    mut content_query: Query<(&mut Node, &ComputedNode), With<ScrollContent>>,
) {
    for event in scroll_events.read() {
        for (mut scroll_container, container_node, children) in scroll_query.iter_mut() {
            // Находим контент
            for child in children.iter() {
                if let Ok((mut content_node, content_layout)) = content_query.get_mut(child) {
                    // Скорость скролла
                    let scroll_speed = 30.0;

                    // Обновляем позицию скролла
                    scroll_container.scroll_position -= event.y * scroll_speed;

                    // Вычисляем максимальный скролл
                    let container_height = container_node.size().y;
                    let content_height = content_layout.size().y;
                    scroll_container.max_scroll = (content_height - container_height).max(0.0);

                    // Ограничиваем скролл
                    scroll_container.scroll_position = scroll_container
                        .scroll_position
                        .max(0.0)
                        .min(scroll_container.max_scroll);

                    // Применяем позицию
                    content_node.top = Val::Px(-scroll_container.scroll_position);
                }
            }
        }
    }
}

/// Обновление максимального скролла при изменении размера
pub fn update_scroll_bounds(
    mut scroll_query: Query<
        (&mut ScrollContainer, &ComputedNode, &Children),
        Changed<ComputedNode>,
    >,
    content_query: Query<&ComputedNode, With<ScrollContent>>,
) {
    for (mut scroll_container, container_node, children) in scroll_query.iter_mut() {
        for child in children.iter() {
            if let Ok(content_node) = content_query.get(child) {
                let container_height = container_node.size().y;
                let content_height = content_node.size().y;
                scroll_container.max_scroll = (content_height - container_height).max(0.0);

                // Ограничиваем текущую позицию
                scroll_container.scroll_position = scroll_container
                    .scroll_position
                    .max(0.0)
                    .min(scroll_container.max_scroll);
            }
        }
    }
}
