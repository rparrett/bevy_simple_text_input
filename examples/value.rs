//! An example showing a very basic implementation.

use bevy::prelude::*;
use bevy_simple_text_input::{TextInput, TextInputBundle, TextInputPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, (button_system, button_style_system))
        .run();
}

#[derive(Component)]
struct IncValueButton;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font_size: 40.,
        color: Color::rgb(0.9, 0.9, 0.9),
        ..default()
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: Val::Px(10.),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: BorderColor(Color::BLACK),
                    background_color: Color::RED.into(),
                    ..default()
                },
                TextInputBundle::with_starting_text(text_style.clone(), "1".to_string()),
            ));

            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(50.),
                            border: UiRect::all(Val::Px(5.0)),
                            padding: UiRect::all(Val::Px(5.0)),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        background_color: Color::RED.into(),
                        ..default()
                    },
                    IncValueButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("+", text_style.clone()));
                });
        });
}

fn button_system(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<IncValueButton>)>,
    mut text_input_query: Query<&mut TextInput>,
) {
    for interaction in &interaction_query {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }

        let mut text_input = text_input_query.single_mut();

        let current_value = text_input.get_value().parse::<i32>().unwrap_or(0);

        text_input.set_value(format!("{}", current_value + 1));
    }
}

fn button_style_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                //*color = PRESSED_BUTTON.into();
                border_color.0 = Color::RED;
            }
            Interaction::Hovered => {
                //*color = HOVERED_BUTTON.into();
                border_color.0 = Color::WHITE;
            }
            Interaction::None => {
                //*color = NORMAL_BUTTON.into();
                border_color.0 = Color::BLACK;
            }
        }
    }
}
