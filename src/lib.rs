//! A Bevy plugin the provides a simple single-line text input widget.
//!
//! # Examples
//!
//! See the [examples](https://github.com/rparrett/bevy_simple_text_input/tree/latest/examples) folder.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_simple_text_input::{TextInputBundle, TextInputPlugin};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(TextInputPlugin)
//!         .add_systems(Startup, setup)
//!         .run();
//! }
//!
//! fn setup(mut commands: Commands) {
//!     commands.spawn(Camera2dBundle::default());
//!     commands.spawn((NodeBundle::default(), TextInputBundle::default()));
//! }
//! ```

use bevy::{
    asset::load_internal_binary_asset,
    ecs::system::SystemParam,
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    text::BreakLineOn,
};

/// A Bevy `Plugin` providing the systems and assets required to make a [`TextInputBundle`] work.
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
                    show_hide_placeholder.after(create),
                ),
            )
            .register_type::<TextInputSettings>()
            .register_type::<TextInputTextStyle>()
            .register_type::<TextInputInactive>()
            .register_type::<TextInputCursorTimer>()
            .register_type::<TextInputInner>()
            .register_type::<TextInputValue>();
    }
}

const CURSOR_HANDLE: Handle<Font> = Handle::weak_from_u128(10482756907980398621);

/// A bundle providing the additional components required for a text input.
///
/// Add this to a Bevy `NodeBundle`.
///
/// # Example
///
/// ```rust
/// # use bevy::prelude::*;
/// use bevy_simple_text_input::TextInputBundle;
/// fn setup(mut commands: Commands) {
///     commands.spawn((NodeBundle::default(), TextInputBundle::default()));
/// }
/// ```
#[derive(Bundle, Default, Reflect)]
pub struct TextInputBundle {
    /// A component containing the text input's settings.
    pub settings: TextInputSettings,
    /// A component containing the Bevy `TextStyle` that will be used when creating the text input's inner Bevy `TextBundle`.
    pub text_style: TextInputTextStyle,
    /// A component containing a value indicating whether the text input is active or not.
    pub inactive: TextInputInactive,
    /// A component that manages the cursor's blinking.
    pub cursor_timer: TextInputCursorTimer,
    /// A component containing the current text cursor position.
    pub cursor_pos: TextInputCursorPos,
    /// A component containing the current value of the text input.
    pub value: TextInputValue,
    /// A component containing the placeholder text that is displayed when the text input is empty.
    pub placeholder: TextInputPlaceholder,
    /// This component's value is managed by Bevy's UI systems and enables tracking of hovers and presses.
    pub interaction: Interaction,
}

impl TextInputBundle {
    /// Returns this [`TextInputBundle`] with a new [`TextInputValue`] containing the provided `String`.
    ///
    /// This also sets [`TextInputCursorPos`] so that the cursor position is at the end of the provided `String`.
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        let owned = value.into();

        self.cursor_pos = TextInputCursorPos(owned.len());
        self.value = TextInputValue(owned);

        self
    }

    /// Returns this [`TextInputBundle`] with a new [`TextInputPlaceholder`] containing the provided `String`.
    pub fn with_placeholder(
        mut self,
        placeholder: impl Into<String>,
        text_style: Option<TextStyle>,
    ) -> Self {
        self.placeholder = TextInputPlaceholder {
            value: placeholder.into(),
            text_style,
        };
        self
    }

    /// Returns this [`TextInputBundle`] with a new [`TextInputTextStyle`] containing the provided Bevy `TextStyle`.
    pub fn with_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = TextInputTextStyle(text_style);
        self
    }

    /// Returns this [`TextInputBundle`] with a new [`TextInputInactive`] containing the provided `bool`.
    pub fn with_inactive(mut self, inactive: bool) -> Self {
        self.inactive = TextInputInactive(inactive);
        self
    }

    /// Returns this [`TextInputBundle`] with a new [`TextInputSettings`].
    pub fn with_settings(mut self, settings: TextInputSettings) -> Self {
        self.settings = settings;
        self
    }
}

/// The Bevy `TextStyle` that will be used when creating the text input's inner Bevy `TextBundle`.
#[derive(Component, Default, Reflect)]
pub struct TextInputTextStyle(pub TextStyle);

/// If true, the text input does not respond to keyboard events and the cursor is hidden.
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

/// A component containing the text input's settings.
#[derive(Component, Default, Reflect)]
pub struct TextInputSettings {
    /// If true, text is not cleared after pressing enter.
    pub retain_on_submit: bool,
}

/// A component containing the current value of the text input.
#[derive(Component, Default, Reflect)]
pub struct TextInputValue(pub String);

/// A component containing the placeholder text that is displayed when the text input is empty.
#[derive(Component, Default, Reflect)]
pub struct TextInputPlaceholder {
    /// The placeholder text.
    pub value: String,
    /// The style to use when rendering the placeholder text.
    pub text_style: Option<TextStyle>,
}

impl TextInputPlaceholder {
    /// Returns the style to use when rendering the placeholder text.
    /// Uses the own style if it exists, otherwise uses the input style with half opacity.
    pub fn get_style(&self, input_text_style: &TextStyle) -> TextStyle {
        if let Some(style) = &self.text_style {
            style.clone()
        } else {
            let color = input_text_style
                .color
                .with_a(input_text_style.color.a() * 0.25);
            TextStyle {
                color,
                ..input_text_style.clone()
            }
        }
    }
}

#[derive(Component, Reflect)]
struct TextInputPlaceholderInner;

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

/// A convenience parameter for dealing with a text input's inner Bevy `Text` entity.
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
        &TextInputSettings,
        &TextInputInactive,
        &mut TextInputValue,
        &mut TextInputCursorPos,
        &mut TextInputCursorTimer,
    )>,
    mut submit_writer: EventWriter<TextInputSubmitEvent>,
) {
    if events.is_empty() {
        return;
    }

    for (input_entity, settings, inactive, mut text_input, mut cursor_pos, mut cursor_timer) in
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
                        text_input.0 = remove_char_at(&text_input.0, cursor_pos.0);

                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::Delete => {
                    if pos < text_input.0.len() {
                        text_input.0 = remove_char_at(&text_input.0, cursor_pos.0);

                        // Ensure that the cursor isn't reset
                        cursor_pos.set_changed();

                        cursor_timer.should_reset = true;
                        continue;
                    }
                }
                KeyCode::Enter => {
                    if settings.retain_on_submit {
                        submitted_value = Some(text_input.0.clone());
                    } else {
                        submitted_value = Some(std::mem::take(&mut text_input.0));
                        cursor_pos.0 = 0;
                    };

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
                let before = text_input.0.chars().take(cursor_pos.0);
                let after = text_input.0.chars().skip(cursor_pos.0);
                text_input.0 = before.chain(s.chars()).chain(after).collect();

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
        (Entity, Ref<TextInputValue>, &mut TextInputCursorPos),
        Or<(Changed<TextInputValue>, Changed<TextInputCursorPos>)>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, text_input, mut cursor_pos) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        // Reset the cursor to the end of the input when the value is changed by
        // a user manipulating the value component.
        if text_input.is_changed() && !cursor_pos.is_changed() {
            cursor_pos.0 = text_input.0.chars().count();
        }

        if cursor_pos.is_changed() {
            cursor_pos.0 = cursor_pos.0.clamp(0, text_input.0.chars().count());
        }

        set_section_values(&text_input.0, cursor_pos.0, &mut text.sections);
    }
}

fn create(
    mut commands: Commands,
    query: Query<
        (
            Entity,
            &TextInputTextStyle,
            &TextInputValue,
            &TextInputCursorPos,
            &TextInputInactive,
            &TextInputPlaceholder,
        ),
        Added<TextInputValue>,
    >,
) {
    for (entity, style, text_input, cursor_pos, inactive, placeholder) in &query {
        let mut sections = vec![
            // Pre-cursor
            TextSection {
                style: style.0.clone(),
                ..default()
            },
            // cursor
            TextSection {
                style: TextStyle {
                    font: CURSOR_HANDLE,
                    color: if inactive.0 {
                        Color::NONE
                    } else {
                        style.0.color
                    },
                    ..style.0.clone()
                },
                ..default()
            },
            // Post-cursor
            TextSection {
                style: style.0.clone(),
                ..default()
            },
        ];

        set_section_values(&text_input.0, cursor_pos.0, &mut sections);

        let text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: BreakLineOn::NoWrap,
                        sections,
                        ..default()
                    },
                    ..default()
                },
                TextInputInner,
            ))
            .id();

        let placeholder_text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: BreakLineOn::NoWrap,
                        sections: vec![TextSection {
                            value: placeholder.value.clone(),
                            style: placeholder.get_style(&style.0),
                        }],
                        ..default()
                    },
                    visibility: Visibility::Hidden,
                    style: Style {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
                TextInputPlaceholderInner,
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
        commands
            .entity(entity)
            .push_children(&[overflow_container, placeholder_text]);
    }
}

// Shows or hides the cursor based on the text input's [`TextInputInactive`] property.
fn show_hide_cursor(
    mut input_query: Query<
        (
            Entity,
            &TextInputTextStyle,
            &mut TextInputCursorTimer,
            &TextInputInactive,
        ),
        Changed<TextInputInactive>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, style, mut cursor_timer, inactive) in &mut input_query {
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

fn show_hide_placeholder(
    input_query: Query<
        (&Children, &TextInputValue, &TextInputInactive),
        Or<(Changed<TextInputValue>, Changed<TextInputInactive>)>,
    >,
    mut vis_query: Query<&mut Visibility, With<TextInputPlaceholderInner>>,
) {
    for (children, text, inactive) in &input_query {
        let mut iter = vis_query.iter_many_mut(children);
        while let Some(mut inner_vis) = iter.fetch_next() {
            inner_vis.set_if_neq(if text.0.is_empty() && inactive.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
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

fn set_section_values(value: &str, cursor_pos: usize, sections: &mut [TextSection]) {
    let before = value.chars().take(cursor_pos).collect();
    let after = value.chars().skip(cursor_pos).collect();

    sections[0].value = before;
    sections[2].value = after;

    // If the cursor is between two characters, use the zero-width cursor.
    if cursor_pos >= value.chars().count() {
        sections[1].value = "}".to_string();
    } else {
        sections[1].value = "|".to_string();
    }
}

fn remove_char_at(input: &str, index: usize) -> String {
    input
        .chars()
        .enumerate()
        .filter_map(|(i, c)| if i != index { Some(c) } else { None })
        .collect()
}
