use bevy::{prelude::*, text::DEFAULT_FONT_HANDLE};
use bevy_simple_text_input::{TextInput, TextInputPlugin, TextInputSubmitEvent};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TextInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, listener)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
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

fn listener(mut events: EventReader<TextInputSubmitEvent>) {
    for event in events.iter() {
        info!("{:?} submitted: {}", event.entity, event.value);
    }
}
