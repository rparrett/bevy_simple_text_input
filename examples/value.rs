//! An example showing a text input that is updated by a button.

use bevy::prelude::*;
use bevy_simple_text_input::{
    TextInput, TextInputPlugin, TextInputSettings, TextInputTextColor, TextInputTextFont,
    TextInputValue,
};

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);
const BORDER_COLOR_HOVER: Color = Color::srgb(0.9, 0.9, 0.9);
const TEXT_COLOR: Color = BORDER_COLOR_HOVER;
const BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);

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
    commands.spawn(Camera2d);

    let text_font = TextFont {
        font_size: 40.,
        ..default()
    };
    let text_color = TextColor(TEXT_COLOR);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(10.),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: Val::Px(200.0),
                    border: UiRect::all(Val::Px(5.0)),
                    padding: UiRect::all(Val::Px(5.0)),
                    ..default()
                },
                BorderColor(BORDER_COLOR_ACTIVE),
                BackgroundColor(BACKGROUND_COLOR),
                TextInput,
                TextInputTextFont(text_font.clone()),
                TextInputTextColor(text_color),
                TextInputValue("1".to_string()),
                TextInputSettings {
                    retain_on_submit: true,
                    ..default()
                },
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(50.),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BorderColor(BORDER_COLOR_INACTIVE),
                    BackgroundColor(BACKGROUND_COLOR),
                    IncValueButton,
                ))
                .with_children(|parent| {
                    parent.spawn((Text::new("+"), text_font, text_color));
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

        let Ok(mut text_input) = text_input_query.single_mut() else {
            continue;
        };

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
