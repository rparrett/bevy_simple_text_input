use bevy::{prelude::*, text::DEFAULT_FONT_HANDLE};
use bevy_simple_text_input::{TextInput, TextInputPlugin};

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
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
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
                TextInput {
                    text_style: TextStyle {
                        font_size: 40.,
                        font: DEFAULT_FONT_HANDLE.typed(),
                        color: Color::rgb(0.9, 0.9, 0.9),
                    },
                    ..default()
                },
            ));
        });
}

fn focus(
    query: Query<(Entity, &Interaction), Changed<Interaction>>,
    mut text_input_query: Query<(Entity, &mut TextInput, &mut BorderColor)>,
) {
    for (interaction_entity, interaction) in &query {
        if *interaction == Interaction::Pressed {
            for (entity, mut text_input, mut border_color) in &mut text_input_query {
                if entity == interaction_entity {
                    text_input.inactive = false;
                    *border_color = BORDER_COLOR_ACTIVE.into();
                } else {
                    text_input.inactive = true;
                    *border_color = BORDER_COLOR_INACTIVE.into();
                }
            }
        }
    }
}
