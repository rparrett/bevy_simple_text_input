//! An example showing a very basic implementation.

use bevy::{prelude::*, window::WindowResolution};
use bevy_simple_text_input::{
    TextInputBundle, TextInputPlugin, TextInputSettings, TextInputSubmitEvent, TextInputSystem,
};

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::default().with_scale_factor_override(2.0),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_plugins(TextInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, listener.after(TextInputSystem))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
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
                        width: Val::Px(400.0),
                        height: Val::Px(200.0),
                        border: UiRect::all(Val::Px(5.0)),
                        padding: UiRect::all(Val::Px(5.0)),
                        ..default()
                    },
                    border_color: BORDER_COLOR_ACTIVE.into(),
                    background_color: BACKGROUND_COLOR.into(),
                    ..default()
                },
                TextInputBundle::default().with_text_style(TextStyle {
                    font_size: 40.,
                    color: TEXT_COLOR,
                    ..default()
                }).with_settings(TextInputSettings {
                    multiline: true,
                    ..Default::default()
                }).with_value("one two three\n\nfour\n five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen twenty"),
            ));
        });
}

fn listener(mut events: EventReader<TextInputSubmitEvent>) {
    for event in events.read() {
        info!("{:?} submitted: {}", event.entity, event.value);
    }
}
