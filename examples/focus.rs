//! An example showing a more advanced implementation with focus.

use bevy::{
    input_focus::{InputDispatchPlugin, InputFocus},
    prelude::*,
};
use bevy_simple_text_input::{
    TextInput, TextInputInactive, TextInputPlaceholder, TextInputPlugin, TextInputSystem,
    TextInputTextColor, TextInputTextFont,
};

const BORDER_COLOR_ACTIVE: Color = Color::srgb(0.75, 0.52, 0.99);
const BORDER_COLOR_INACTIVE: Color = Color::srgb(0.25, 0.25, 0.25);
const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);
const BACKGROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.15);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin))
        .add_plugins(TextInputPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, focus.before(TextInputSystem))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::ColumnReverse,
            row_gap: Val::Px(10.),
            ..default()
        })
        .observe(background_node_click)
        .with_children(|parent| {
            parent.spawn(text_input(true)).observe(text_input_click);
            parent.spawn(text_input(false)).observe(text_input_click);
        });
}

fn text_input(always_visible_when_empty: bool) -> impl Bundle {
    (
        Node {
            width: Val::Px(200.0),
            border: UiRect::all(Val::Px(5.0)),
            padding: UiRect::all(Val::Px(5.0)),
            ..default()
        },
        BorderColor(BORDER_COLOR_INACTIVE),
        BackgroundColor(BACKGROUND_COLOR),
        TextInput,
        TextInputTextFont(TextFont {
            font_size: 34.,
            ..default()
        }),
        TextInputTextColor(TextColor(TEXT_COLOR)),
        TextInputPlaceholder {
            value: "Click Me".to_string(),
            always_visible_when_empty,
            ..default()
        },
        TextInputInactive(true),
    )
}

fn focus(
    focus: Res<InputFocus>,
    mut text_inputs: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
) {
    if !focus.is_changed() {
        return;
    }

    for (entity, mut inactive, mut border_color) in text_inputs.iter_mut() {
        if focus.0 == Some(entity) {
            inactive.0 = false;
            *border_color = BORDER_COLOR_ACTIVE.into();
        } else {
            inactive.0 = true;
            *border_color = BORDER_COLOR_INACTIVE.into();
        }
    }
}

fn background_node_click(mut trigger: Trigger<Pointer<Click>>, mut focus: ResMut<InputFocus>) {
    focus.0 = None;
    trigger.propagate(false);
}

fn text_input_click(mut trigger: Trigger<Pointer<Click>>, mut focus: ResMut<InputFocus>) {
    focus.0 = Some(trigger.target());
    trigger.propagate(false);
}
