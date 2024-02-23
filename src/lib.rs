//! A Bevy plugin the provides a simple single-line text input widget.

use bevy::{
    asset::load_internal_binary_asset,
    ecs::system::SystemParam,
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    text::BreakLineOn,
};

/// A `Plugin` providing the systems and assets required to make a [`TextInputBundle`] work.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        // This is a special font with a zero-width `|` glyph.
        load_internal_binary_asset!(
            app,
            CURSOR_HANDLE,
            "../assets/Cursor.ttf",
            |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
        );

        app.add_event::<TextInputSubmitEvent>()
            .add_systems(
                Update,
                (
                    create,
                    keyboard,
                    update_value.after(keyboard),
                    blink_cursor,
                    show_hide_cursor,
                    update_style,
                ),
            )
            .register_type::<TextInputTextStyle>()
            .register_type::<TextInputInactive>()
            .register_type::<TextInputCursorTimer>()
            .register_type::<TextInputInner>()
            .register_type::<TextInput>();
    }
}

const CURSOR_HANDLE: Handle<Font> = Handle::weak_from_u128(10482756907980398621);

/// A bundle providing the additional components required for a text input.
///
/// Add this to a [`NodeBundle`].
///
/// Examples:
/// ```rust
/// # use bevy::prelude::*;
/// use bevy_simple_text_input::TextInputBundle;
/// fn setup(mut commands: Commands) {
///     commands.spawn((NodeBundle::default(), TextInputBundle::default()));
/// }
/// ```
#[derive(Bundle, Default, Reflect)]
pub struct TextInputBundle {
    text_style: TextInputTextStyle,
    inactive: TextInputInactive,
    cursor_timer: TextInputCursorTimer,
    cursor_pos: TextInputCursorPos,
    text_input: TextInput,
    interaction: Interaction,
}

impl TextInputBundle {
    /// Creates a new [`TextInputBundle`] with the specified [`TextStyle`].
    pub fn new(text_style: TextStyle) -> Self {
        Self {
            text_style: TextInputTextStyle(text_style),
            ..default()
        }
    }

    /// Creates a new [`TextInputBundle`] with the specified [`TextStyle`] and starting text.
    pub fn with_starting_text(text_style: TextStyle, starting_text: String) -> Self {
        Self {
            text_style: TextInputTextStyle(text_style),
            text_input: TextInput(starting_text),
            ..default()
        }
    }
}

/// The [`TextStyle`] that will be used when creating the text input's inner [`TextBundle`].
#[derive(Component, Default, Reflect)]
pub struct TextInputTextStyle(pub TextStyle);

/// If true, the text input does not respond to keyboard events.
#[derive(Component, Default, Reflect)]
pub struct TextInputInactive(pub bool);

/// A component that manages the cursor's blinking.
#[derive(Component, Reflect)]
pub struct TextInputCursorTimer {
    /// The timer that blinks the cursor on and off, and resets when the user types.
    pub timer: Timer,
    should_reset: bool,
}

impl Default for TextInputCursorTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.5, TimerMode::Repeating),
            should_reset: false,
        }
    }
}

/// A component containing the current value of the text input.
#[derive(Component, Default, Reflect)]
pub struct TextInput(pub String);

/// A component containing the current text cursor position.
#[derive(Component, Default, Reflect)]
pub struct TextInputCursorPos(pub usize);

#[derive(Component, Reflect)]
struct TextInputInner;

/// An event that is fired when the user presses the enter key.
#[derive(Event)]
pub struct TextInputSubmitEvent {
    /// The text input that triggered the event.
    pub entity: Entity,
    /// The string contained in the text input at the time of the event.
    pub value: String,
}

/// A convenience parameter for dealing with a [`TextInput`]'s inner [`Text`] entity.
#[derive(SystemParam)]
struct InnerText<'w, 's> {
    text_query: Query<'w, 's, &'static mut Text, With<TextInputInner>>,
    children_query: Query<'w, 's, &'static Children>,
}
impl<'w, 's> InnerText<'w, 's> {
    fn get_mut(&mut self, entity: Entity) -> Option<Mut<'_, Text>> {
        self.children_query
            .iter_descendants(entity)
            .find(|descendant_entity| self.text_query.get(*descendant_entity).is_ok())
            .and_then(|text_entity| self.text_query.get_mut(text_entity).ok())
    }
}

fn keyboard(
    mut events: EventReader<KeyboardInput>,
    mut text_input_query: Query<(
        Entity,
        &TextInputInactive,
        &mut TextInput,
        &mut TextInputCursorPos,
        &mut TextInputCursorTimer,
    )>,
    mut submit_writer: EventWriter<TextInputSubmitEvent>,
) {
    if events.is_empty() {
        return;
    }

    for (input_entity, inactive, mut text_input, mut cursor_pos, mut cursor_timer) in
        &mut text_input_query
    {
        if inactive.0 {
            continue;
        }

        let mut submitted_value = None;

        for event in events.read() {
            if !event.state.is_pressed() {
                continue;
            };

            let pos = cursor_pos.bypass_change_detection().0;

            match event.key_code {
                KeyCode::ArrowLeft => {
                    if pos > 0 {
                        cursor_pos.0 -= 1;

                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::ArrowRight => {
                    if pos < text_input.0.len() {
                        cursor_pos.0 += 1;

                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::Backspace => {
                    if pos > 0 {
                        cursor_pos.0 -= 1;
                        text_input.0.remove(cursor_pos.0);

                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::Delete => {
                    if pos < text_input.0.len() {
                        text_input.0.remove(pos);
                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::Enter => {
                    submitted_value = Some(std::mem::take(&mut text_input.0));
                    cursor_pos.0 = 0;

                    continue;
                }
                KeyCode::Space => {
                    text_input.0.insert(pos, ' ');
                    cursor_pos.0 += 1;

                    cursor_timer.should_reset = true;
                    continue;
                }
                _ => {}
            }

            if let Key::Character(ref s) = event.logical_key {
                text_input.0.insert_str(cursor_pos.0, s);
                cursor_pos.0 += 1;

                cursor_timer.should_reset = true;
            }
        }

        if let Some(value) = submitted_value {
            submit_writer.send(TextInputSubmitEvent {
                entity: input_entity,
                value,
            });
        }
    }
}

fn update_value(
    mut input_query: Query<
        (Entity, Ref<TextInput>, &mut TextInputCursorPos),
        Or<(Changed<TextInput>, Changed<TextInputCursorPos>)>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, text_input, mut cursor_pos) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        if text_input.is_changed() && !cursor_pos.is_changed() {
            cursor_pos.0 = text_input.0.len();
        }

        let (before, after) = text_input.0.split_at(cursor_pos.0);
        text.sections[0].value = before.to_string();
        text.sections[2].value = after.to_string();

        // If the cursor is between two characters, use the zero-width cursor.
        if cursor_pos.0 >= text_input.0.len() {
            text.sections[1].value = "}".to_string();
        } else {
            text.sections[1].value = "|".to_string();
        }
    }
}

fn create(
    mut commands: Commands,
    query: Query<(Entity, &TextInputTextStyle, &TextInput), Added<TextInput>>,
) {
    for (entity, style, text_input) in &query {
        let text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: BreakLineOn::NoWrap,
                        sections: vec![
                            // Pre-cursor
                            TextSection {
                                value: text_input.0.clone(),
                                style: style.0.clone(),
                            },
                            // cursor
                            TextSection {
                                value: "}".to_string(),
                                style: TextStyle {
                                    font: CURSOR_HANDLE,
                                    ..style.0.clone()
                                },
                            },
                            // Post-cursor
                            TextSection {
                                value: "".to_string(),
                                style: style.0.clone(),
                            },
                        ],
                        ..default()
                    },
                    ..default()
                },
                TextInputInner,
            ))
            .id();

        let overflow_container = commands
            .spawn(NodeBundle {
                style: Style {
                    overflow: Overflow::clip(),
                    justify_content: JustifyContent::FlexEnd,
                    max_width: Val::Percent(100.),
                    ..default()
                },
                ..default()
            })
            .id();

        commands.entity(overflow_container).add_child(text);
        commands.entity(entity).add_child(overflow_container);
    }
}

// Shows or hides the cursor based on the text input's `inactive` property.
fn show_hide_cursor(
    mut input_query: Query<(
        Entity,
        &TextInputTextStyle,
        &mut TextInputCursorTimer,
        Ref<TextInputInactive>,
    )>,
    mut inner_text: InnerText,
) {
    for (entity, style, mut cursor_timer, inactive) in &mut input_query {
        if !inactive.is_changed() {
            continue;
        }

        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        text.sections[1].style.color = if inactive.0 {
            Color::NONE
        } else {
            style.0.color
        };

        cursor_timer.timer.reset();
    }
}

// Blinks the cursor on a timer.
fn blink_cursor(
    mut input_query: Query<(
        Entity,
        &TextInputTextStyle,
        &mut TextInputCursorTimer,
        Ref<TextInputInactive>,
    )>,
    mut inner_text: InnerText,
    time: Res<Time>,
) {
    for (entity, style, mut cursor_timer, inactive) in &mut input_query {
        if inactive.0 {
            continue;
        }

        if cursor_timer.is_changed() && cursor_timer.should_reset {
            cursor_timer.timer.reset();
            cursor_timer.should_reset = false;
            if let Some(mut text) = inner_text.get_mut(entity) {
                text.sections[1].style.color = style.0.color;
            }
            continue;
        }

        if !cursor_timer.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        if text.sections[1].style.color != Color::NONE {
            text.sections[1].style.color = Color::NONE;
        } else {
            text.sections[1].style.color = style.0.color;
        }
    }
}

fn update_style(
    mut input_query: Query<(Entity, &TextInputTextStyle), Changed<TextInputTextStyle>>,
    mut inner_text: InnerText,
) {
    for (entity, style) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        text.sections[0].style = style.0.clone();
        text.sections[1].style = TextStyle {
            font: CURSOR_HANDLE,
            ..style.0.clone()
        };
        text.sections[2].style = style.0.clone();
    }
}
