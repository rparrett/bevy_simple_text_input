//! An example showing a very basic implementation.

use bevy::prelude::*;
use bevy_simple_text_input::{
    TextInputBundle, TextInputPlugin, TextInputSettings, TextInputTextStyle, TextInputValue,
};

const BORDER_COLOR_ACTIVE: Color = Color::rgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::rgb(0.25, 0.25, 0.25);
const BORDER_COLOR_HOVER: Color = Color::rgb(0.9, 0.9, 0.9);
const TEXT_COLOR: Color = BORDER_COLOR_HOVER;
const BACKGROUND_COLOR: Color = Color::rgb(0.15, 0.15, 0.15);

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
        color: TEXT_COLOR,
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
                    border_color: BorderColor(BORDER_COLOR_ACTIVE),
                    background_color: BACKGROUND_COLOR.into(),
                    ..default()
                },
                TextInputBundle {
                    settings: TextInputSettings {
                        retain_on_submit: true,
                    },
                    value: TextInputValue("1".to_string()),
                    text_style: TextInputTextStyle(text_style.clone()),
                    ..default()
                },
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
                        border_color: BorderColor(BORDER_COLOR_INACTIVE),
                        background_color: BACKGROUND_COLOR.into(),
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
    mut text_input_query: Query<&mut TextInputValue>,
) {
    for interaction in &interaction_query {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }

        let mut text_input = text_input_query.single_mut();

        let current_value = text_input.0.parse::<i32>().unwrap_or(0);

        text_input.0 = format!("{}", current_value + 1);
    }
}

fn button_style_system(
    mut interaction_query: Query<
        (&Interaction, &mut BorderColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut border_color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                border_color.0 = BORDER_COLOR_ACTIVE;
            }
            Interaction::Hovered => {
                border_color.0 = BORDER_COLOR_HOVER;
            }
            Interaction::None => {
                border_color.0 = BORDER_COLOR_INACTIVE;
            }
        }
    }
}
