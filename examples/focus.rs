//! An example showing a more advanced implementation with focus.

use bevy::prelude::*;
use bevy_simple_text_input::{TextInput, TextInputBundle, TextInputInactive, TextInputPlugin};

const BORDER_COLOR_ACTIVE: Color = Color::VIOLET;
const BORDER_COLOR_INACTIVE: Color = Color::BLACK;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, focus)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            // Make this container node bundle to be Interactive so that clicking on it removes
            // focus from the text input.
            Interaction::None,
        ))
        .with_children(|parent| {
            parent.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: BORDER_COLOR_ACTIVE.into(),
                    background_color: Color::INDIGO.into(),
                    ..default()
                },
                TextInputBundle::with_starting_text(
                    TextStyle {
                        font_size: 40.,
                        color: Color::rgb(0.9, 0.9, 0.9),
                        ..default()
                    },
                    "text".to_owned(),
                ),
            ));
        });
}

fn focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
    text_query: Query<&TextInput, Changed<TextInput>>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut inactive, mut border_color) in &mut text_input_query {
                if entity == interaction_entity {
                    inactive.0 = false;
                    *border_color = BORDER_COLOR_ACTIVE.into();
                } else {
                    inactive.0 = true;
                    *border_color = BORDER_COLOR_INACTIVE.into();
                }
            }
        }
    }
    for text in &text_query {
        info!("Input updated: {}", **text);
    }
}
