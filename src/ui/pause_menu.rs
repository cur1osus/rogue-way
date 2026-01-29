use bevy::prelude::*;

use crate::constants::{ui_colors, ui_text, UI_FONT_SCALE};
use crate::resources::UiFonts;
use crate::ui::GameState;

#[derive(Component)]
pub struct PauseMenuUI;

#[derive(Component)]
pub struct ResumeButton;

#[derive(Component)]
pub struct ExitToMenuButton;

#[derive(Component, Copy, Clone)]
pub struct PauseButtonBaseColor(pub Color);

pub fn setup_pause_menu(mut commands: Commands, ui_fonts: Res<UiFonts>) {
    let font = ui_fonts.main.clone();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(ui_colors::OVERLAY_DARK),
            PauseMenuUI,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(520.0),
                        height: Val::Px(320.0),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: Val::Px(20.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(ui_colors::PANEL_DARK),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(ui_text::PAUSE_TITLE),
                        TextFont {
                            font: font.clone(),
                            font_size: 36.0 * UI_FONT_SCALE,
                            ..default()
                        },
                        TextColor(ui_colors::TEXT_YELLOW_LIGHT),
                    ));

                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(12.0),
                            ..default()
                        })
                        .with_children(|buttons| {
                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(280.0),
                                        height: Val::Px(60.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(ui_colors::BUTTON_GREEN),
                                    PauseButtonBaseColor(ui_colors::BUTTON_GREEN),
                                    ResumeButton,
                                ))
                                .with_children(|button_parent| {
                                    button_parent.spawn((
                                        Text::new(ui_text::BTN_RESUME),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 28.0 * UI_FONT_SCALE,
                                            ..default()
                                        },
                                        TextColor(ui_colors::TEXT_WHITE),
                                    ));
                                });

                            buttons
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(280.0),
                                        height: Val::Px(60.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(ui_colors::BUTTON_RED),
                                    PauseButtonBaseColor(ui_colors::BUTTON_RED),
                                    ExitToMenuButton,
                                ))
                                .with_children(|button_parent| {
                                    button_parent.spawn((
                                        Text::new(ui_text::BTN_EXIT_TO_MENU),
                                        TextFont {
                                            font: font.clone(),
                                            font_size: 22.0 * UI_FONT_SCALE,
                                            ..default()
                                        },
                                        TextColor(ui_colors::TEXT_WHITE),
                                    ));
                                });
                        });
                });
        });
}

pub fn handle_pause_menu_buttons(
    mut commands: Commands,
    resume_query: Query<&Interaction, (Changed<Interaction>, With<ResumeButton>)>,
    exit_query: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<ExitToMenuButton>,
            Without<ResumeButton>,
        ),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    pause_ui_query: Query<Entity, With<PauseMenuUI>>,
) {
    for interaction in resume_query.iter() {
        if *interaction == Interaction::Pressed {
            for entity in pause_ui_query.iter() {
                commands.entity(entity).despawn();
            }
            next_state.set(GameState::Playing);
        }
    }

    for interaction in exit_query.iter() {
        if *interaction == Interaction::Pressed {
            for entity in pause_ui_query.iter() {
                commands.entity(entity).despawn();
            }
            next_state.set(GameState::MainMenu);
        }
    }
}

pub fn pause_menu_button_hover_system(
    mut button_query: Query<
        (&Interaction, &PauseButtonBaseColor, &mut BackgroundColor),
        (
            Changed<Interaction>,
            Or<(With<ResumeButton>, With<ExitToMenuButton>)>,
        ),
    >,
) {
    for (interaction, base_color, mut background) in button_query.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                let srgba = base_color.0.to_srgba();
                *background = BackgroundColor(Color::srgba(
                    (srgba.red * 1.2).min(1.0),
                    (srgba.green * 1.2).min(1.0),
                    (srgba.blue * 1.2).min(1.0),
                    srgba.alpha,
                ));
            }
            Interaction::Pressed => {
                let srgba = base_color.0.to_srgba();
                *background = BackgroundColor(Color::srgba(
                    srgba.red * 0.8,
                    srgba.green * 0.8,
                    srgba.blue * 0.8,
                    srgba.alpha,
                ));
            }
            Interaction::None => {
                *background = BackgroundColor(base_color.0);
            }
        }
    }
}

pub fn pause_on_escape_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Paused);
    }
}

pub fn resume_on_escape_system(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Playing);
    }
}

pub fn cleanup_pause_menu(
    mut commands: Commands,
    pause_ui_query: Query<Entity, With<PauseMenuUI>>,
) {
    for entity in pause_ui_query.iter() {
        commands.entity(entity).despawn();
    }
}
