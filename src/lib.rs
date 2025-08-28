//! A Bevy plugin the provides a simple single-line text input widget.
//!
//! # Examples
//!
//! See the [examples](https://github.com/rparrett/bevy_simple_text_input/tree/latest/examples) folder.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_simple_text_input::{TextInput, TextInputPlugin};
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
//!     commands.spawn(Camera2d);
//!     commands.spawn((
//!         TextInput,
//!         Node {
//!             padding: UiRect::all(Val::Px(5.0)),
//!             border: UiRect::all(Val::Px(2.0)),
//!             ..default()
//!         },
//!         BorderColor(Color::BLACK)
//!     ));
//! }
//! ```

use bevy::{
    asset::{load_internal_binary_asset, uuid_handle},
    ecs::{event::EventCursor, system::SystemParam},
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    text::{LineBreak, TextLayoutInfo},
    ui::FocusPolicy,
};

/// A Bevy `Plugin` providing the systems and assets required to make a [`TextInput`] work.
pub struct TextInputPlugin;

/// Label for systems that update text inputs.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct TextInputSystem;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        // This is a special font with a zero-width `|` glyph.
        load_internal_binary_asset!(
            app,
            CURSOR_HANDLE,
            "../assets/Cursor.ttf",
            |bytes: &[u8], _path: String| { Font::try_from_bytes(bytes.to_vec()).unwrap() }
        );

        app.init_resource::<TextInputNavigationBindings>()
            .add_event::<TextInputSubmitEvent>()
            .add_observer(create)
            .add_systems(
                Update,
                (
                    keyboard,
                    update_value.after(keyboard),
                    (blink_cursor, show_hide_cursor, update_color)
                        .chain()
                        .after(keyboard)
                        .ambiguous_with(update_value)
                        .ambiguous_with(update_style),
                    update_style.ambiguous_with(update_value),
                    show_hide_placeholder.after(keyboard),
                    scroll_with_cursor,
                )
                    .in_set(TextInputSystem),
            )
            .register_type::<TextInputSettings>()
            .register_type::<TextInputTextFont>()
            .register_type::<TextInputTextColor>()
            .register_type::<TextInputInactive>()
            .register_type::<TextInputCursorTimer>()
            .register_type::<TextInputInner>()
            .register_type::<TextInputValue>()
            .register_type::<TextInputPlaceholder>()
            .register_type::<TextInputCursorPos>();
    }
}

const CURSOR_HANDLE: Handle<Font> = uuid_handle!("82b134b2-92c0-461a-891f-c35b968f2b88");

/// The main "driving component" for the Text Input.
///
/// In addition to its [required components](TextInput#impl-Component-for-TextInput), some other
/// components may also be spawned with it: [`TextInputCursorPos`].
///
/// # Example
///
/// ```rust
/// # use bevy::prelude::*;
/// use bevy_simple_text_input::TextInput;
/// fn setup(mut commands: Commands) {
///     commands.spawn(TextInput);
/// }
/// ```
#[derive(Component, Default)]
#[require(
    TextInputSettings,
    TextInputTextFont,
    TextInputTextColor,
    TextInputInactive,
    TextInputCursorTimer,
    TextInputValue,
    TextInputPlaceholder,
    Node,
    Interaction
)]
pub struct TextInput;

/// The Bevy `TextColor` that will be used when creating the text input's inner Bevy `TextBundle`.
#[derive(Component, Default, Reflect)]
pub struct TextInputTextFont(pub TextFont);

/// The Bevy `TextColor` that will be used when creating the text input's inner Bevy `TextBundle`.
#[derive(Component, Default, Reflect)]
pub struct TextInputTextColor(pub TextColor);

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
    /// Mask text with the provided character.
    pub mask_character: Option<char>,
}

/// Text navigation actions that can be bound via `TextInputNavigationBindings`.
#[derive(Debug)]
pub enum TextInputAction {
    /// Moves the cursor one char to the left.
    CharLeft,
    /// Moves the cursor one char to the right.
    CharRight,
    /// Moves the cursor to the start of line.
    LineStart,
    /// Moves the cursor to the end of line.
    LineEnd,
    /// Moves the cursor one word to the left.
    WordLeft,
    /// Moves the cursor one word to the right.
    WordRight,
    /// Removes the char left of the cursor.
    DeletePrev,
    /// Removes the char right of the cursor.
    DeleteNext,
    /// Triggers a `TextInputSubmitEvent`, optionally clearing the text input.
    Submit,
}
/// A resource in which key bindings can be specified. Bindings are given as a tuple of (`TextInputAction`, `TextInputBinding`).
///
/// All modifiers must be held when the primary key is pressed to perform the action.
/// The first matching action in the list will be performed, so a binding that is the same as another with additional
/// modifier keys should be earlier in the vector to be applied.
#[derive(Resource)]
pub struct TextInputNavigationBindings(pub Vec<(TextInputAction, TextInputBinding)>);

/// A combination of a key and required modifier keys that might trigger a `TextInputAction`.
pub struct TextInputBinding {
    /// Primary key
    key: KeyCode,
    /// Required modifier keys
    modifiers: Vec<KeyCode>,
}

impl TextInputBinding {
    /// Creates a new `TextInputBinding` from a key and required modifiers.
    pub fn new(key: KeyCode, modifiers: impl Into<Vec<KeyCode>>) -> Self {
        Self {
            key,
            modifiers: modifiers.into(),
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl Default for TextInputNavigationBindings {
    fn default() -> Self {
        use KeyCode::*;
        use TextInputAction::*;
        Self(vec![
            (LineStart, TextInputBinding::new(Home, [])),
            (LineEnd, TextInputBinding::new(End, [])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [ControlLeft])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [ControlRight])),
            (WordRight, TextInputBinding::new(ArrowRight, [ControlLeft])),
            (WordRight, TextInputBinding::new(ArrowRight, [ControlRight])),
            (CharLeft, TextInputBinding::new(ArrowLeft, [])),
            (CharRight, TextInputBinding::new(ArrowRight, [])),
            (DeletePrev, TextInputBinding::new(Backspace, [])),
            (DeletePrev, TextInputBinding::new(NumpadBackspace, [])),
            (DeleteNext, TextInputBinding::new(Delete, [])),
            (Submit, TextInputBinding::new(Enter, [])),
            (Submit, TextInputBinding::new(NumpadEnter, [])),
        ])
    }
}

#[cfg(target_os = "macos")]
impl Default for TextInputNavigationBindings {
    fn default() -> Self {
        use KeyCode::*;
        use TextInputAction::*;
        Self(vec![
            (LineStart, TextInputBinding::new(ArrowLeft, [SuperLeft])),
            (LineStart, TextInputBinding::new(ArrowLeft, [SuperRight])),
            (LineEnd, TextInputBinding::new(ArrowRight, [SuperLeft])),
            (LineEnd, TextInputBinding::new(ArrowRight, [SuperRight])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [AltLeft])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [AltRight])),
            (WordRight, TextInputBinding::new(ArrowRight, [AltLeft])),
            (WordRight, TextInputBinding::new(ArrowRight, [AltRight])),
            (CharLeft, TextInputBinding::new(ArrowLeft, [])),
            (CharRight, TextInputBinding::new(ArrowRight, [])),
            (DeletePrev, TextInputBinding::new(Backspace, [])),
            (DeletePrev, TextInputBinding::new(NumpadBackspace, [])),
            (DeleteNext, TextInputBinding::new(Delete, [])),
            (Submit, TextInputBinding::new(Enter, [])),
            (Submit, TextInputBinding::new(NumpadEnter, [])),
        ])
    }
}

/// A component containing the current value of the text input.
#[derive(Component, Default, Reflect)]
pub struct TextInputValue(pub String);

/// A component containing the placeholder text that is displayed when the text input is empty and not focused.
#[derive(Component, Default, Reflect)]
pub struct TextInputPlaceholder {
    /// The placeholder text.
    pub value: String,
    /// The `TextFont` to use when rendering the placeholder text.
    ///
    /// If `None`, the text input font will be used.
    pub text_font: Option<TextFont>,
    /// The style to use when rendering the placeholder text.
    ///
    /// If `None`, the text input color will be used with alpha value of `0.25`.
    pub text_color: Option<TextColor>,
}

#[derive(Component, Reflect)]
struct TextInputPlaceholderInner;

/// A component containing the current text cursor position.
#[derive(Component, Default, Reflect)]
pub struct TextInputCursorPos(pub usize);

#[derive(Component, Reflect)]
struct TextInputInner;

/// An event that is fired when the user presses the enter key.
#[derive(BufferedEvent)]
pub struct TextInputSubmitEvent {
    /// The text input that triggered the event.
    pub entity: Entity,
    /// The string contained in the text input at the time of the event.
    pub value: String,
}

/// A convenience parameter for dealing with a text input's inner Bevy `Text` entity.
#[derive(SystemParam)]
struct InnerText<'w, 's> {
    text_query: Query<'w, 's, (), With<TextInputInner>>,
    children_query: Query<'w, 's, &'static Children>,
}
impl InnerText<'_, '_> {
    fn inner_entity(&self, entity: Entity) -> Option<Entity> {
        self.children_query
            .iter_descendants(entity)
            .find(|descendant_entity| self.text_query.get(*descendant_entity).is_ok())
    }
}

fn keyboard(
    key_input: Res<ButtonInput<KeyCode>>,
    input_events: Res<Events<KeyboardInput>>,
    mut input_reader: Local<EventCursor<KeyboardInput>>,
    mut text_input_query: Query<(
        Entity,
        &TextInputSettings,
        &TextInputInactive,
        &mut TextInputValue,
        &mut TextInputCursorPos,
        &mut TextInputCursorTimer,
    )>,
    mut submit_writer: EventWriter<TextInputSubmitEvent>,
    navigation: Res<TextInputNavigationBindings>,
) {
    if input_reader.clone().read(&input_events).next().is_none() {
        return;
    }

    // collect actions that have all required modifiers held
    let valid_actions = navigation
        .0
        .iter()
        .filter(|(_, TextInputBinding { modifiers, .. })| {
            modifiers.iter().all(|m| key_input.pressed(*m))
        })
        .map(|(action, TextInputBinding { key, .. })| (*key, action));

    for (input_entity, settings, inactive, mut text_input, mut cursor_pos, mut cursor_timer) in
        &mut text_input_query
    {
        if inactive.0 {
            continue;
        }

        let mut submitted_value = None;

        for input in input_reader.clone().read(&input_events) {
            if !input.state.is_pressed() {
                continue;
            };

            let pos = cursor_pos.bypass_change_detection().0;

            if let Some((_, action)) = valid_actions
                .clone()
                .find(|(key, _)| *key == input.key_code)
            {
                use TextInputAction::*;
                let mut timer_should_reset = true;
                match action {
                    CharLeft => cursor_pos.0 = cursor_pos.0.saturating_sub(1),
                    CharRight => cursor_pos.0 = (cursor_pos.0 + 1).min(text_input.0.len()),
                    LineStart => cursor_pos.0 = 0,
                    LineEnd => cursor_pos.0 = text_input.0.len(),
                    WordLeft => {
                        cursor_pos.0 = text_input
                            .0
                            .char_indices()
                            .rev()
                            .skip(text_input.0.len() - cursor_pos.0 + 1)
                            .skip_while(|c| c.1.is_ascii_whitespace())
                            .find(|c| c.1.is_ascii_whitespace())
                            .map(|(ix, _)| ix + 1)
                            .unwrap_or(0)
                    }
                    WordRight => {
                        cursor_pos.0 = text_input
                            .0
                            .char_indices()
                            .skip(cursor_pos.0)
                            .skip_while(|c| !c.1.is_ascii_whitespace())
                            .find(|c| !c.1.is_ascii_whitespace())
                            .map(|(ix, _)| ix)
                            .unwrap_or(text_input.0.len())
                    }
                    DeletePrev => {
                        if pos > 0 {
                            cursor_pos.0 -= 1;
                            text_input.0 = remove_char_at(&text_input.0, cursor_pos.0);
                        }
                    }
                    DeleteNext => {
                        if pos < text_input.0.len() {
                            text_input.0 = remove_char_at(&text_input.0, cursor_pos.0);

                            // Ensure that the cursor isn't reset
                            cursor_pos.set_changed();
                        }
                    }
                    Submit => {
                        if settings.retain_on_submit {
                            submitted_value = Some(text_input.0.clone());
                        } else {
                            submitted_value = Some(std::mem::take(&mut text_input.0));
                            cursor_pos.0 = 0;
                        };
                        timer_should_reset = false;
                    }
                }

                cursor_timer.should_reset |= timer_should_reset;
                continue;
            }

            match input.logical_key {
                Key::Space => {
                    let byte_pos = byte_pos(&text_input.0, pos);
                    text_input.0.insert(byte_pos, ' ');
                    cursor_pos.0 += 1;

                    cursor_timer.should_reset = true;
                }
                Key::Character(ref s) => {
                    let byte_pos = byte_pos(&text_input.0, pos);
                    text_input.0.insert_str(byte_pos, s.as_str());

                    cursor_pos.0 += 1;

                    cursor_timer.should_reset = true;
                }
                _ => (),
            }
        }

        if let Some(value) = submitted_value {
            submit_writer.write(TextInputSubmitEvent {
                entity: input_entity,
                value,
            });
        }
    }

    input_reader.clear(&input_events);
}

fn update_value(
    mut input_query: Query<
        (
            Entity,
            Ref<TextInputValue>,
            &TextInputSettings,
            &mut TextInputCursorPos,
        ),
        Or<(Changed<TextInputValue>, Changed<TextInputCursorPos>)>,
    >,
    inner_text: InnerText,
    mut writer: TextUiWriter,
) {
    for (entity, text_input, settings, mut cursor_pos) in &mut input_query {
        let Some(inner) = inner_text.inner_entity(entity) else {
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

        let values = get_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            cursor_pos.0,
        );

        *writer.text(inner, 0) = values.0;
        *writer.text(inner, 1) = values.1;
        *writer.text(inner, 2) = values.2;
    }
}

fn scroll_with_cursor(
    mut inner_text_query: Query<
        (&TextLayoutInfo, &ComputedNode, &ChildOf),
        (With<TextInputInner>, Changed<TextLayoutInfo>),
    >,
    mut style_query: Query<
        (&ComputedNode, &mut Node, &mut ScrollPosition),
        Without<TextInputInner>,
    >,
) {
    for (layout, computed, child_of) in inner_text_query.iter_mut() {
        let Ok((overflow_computed, mut overflow_style, mut overflow_scroll)) =
            style_query.get_mut(child_of.parent())
        else {
            continue;
        };

        match layout.glyphs.last().map(|g| g.span_index) {
            // No text, nothing to do.
            None => continue,
            // If cursor is at the end, we can use FlexEnd so newly typed text does not take a
            // frame to move into view
            Some(1) => {
                overflow_scroll.x = 0.0;
                overflow_style.justify_content = JustifyContent::FlexEnd;
                continue;
            }
            _ => (),
        }

        let inverse_scale_factor = computed.inverse_scale_factor();

        let text_size = computed.size().x * inverse_scale_factor;
        let overflow_size = overflow_computed.size().x * inverse_scale_factor;

        let Some(cursor_pos) = layout
            .glyphs
            .iter()
            .find(|g| g.span_index == 1)
            .map(|p| p.position.x * inverse_scale_factor)
        else {
            continue;
        };

        let relative_pos = cursor_pos - overflow_scroll.x;

        if relative_pos < 0.0 || relative_pos > overflow_size {
            let req_px = overflow_size * 0.5 - cursor_pos;
            let req_px = req_px.clamp(overflow_size - text_size, 0.0);

            // If the cursor is not at the end, we use have to use FlexStart.
            // See https://github.com/bevyengine/bevy/issues/17129.

            overflow_scroll.x = -req_px;
            overflow_style.justify_content = JustifyContent::FlexStart;
        }
    }
}

fn create(
    trigger: On<Add, TextInputValue>,
    mut commands: Commands,
    query: Query<(
        Entity,
        &TextInputTextFont,
        &TextInputTextColor,
        &TextInputValue,
        Option<&TextInputCursorPos>,
        &TextInputInactive,
        &TextInputSettings,
        &TextInputPlaceholder,
    )>,
) {
    if let Ok((
        entity,
        font,
        color,
        text_input,
        maybe_cursor_pos,
        inactive,
        settings,
        placeholder,
    )) = &query.get(trigger.entity())
    {
        let cursor_pos = match maybe_cursor_pos {
            None => {
                let len = text_input.0.len();
                commands.entity(*entity).insert(TextInputCursorPos(len));
                len
            }
            Some(cursor_pos) => cursor_pos.0,
        };

        let values = get_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            cursor_pos,
        );

        let text = commands
            .spawn((
                Text::default(),
                TextLayout::new_with_linebreak(LineBreak::NoWrap),
                Name::new("TextInputInner"),
                TextInputInner,
            ))
            .with_children(|parent| {
                // Pre-cursor
                parent.spawn((TextSpan::new(values.0), font.0.clone()));

                // Cursor
                parent.spawn((
                    TextSpan::new(values.1),
                    TextFont {
                        font: CURSOR_HANDLE,
                        ..font.0.clone()
                    },
                    if inactive.0 {
                        TextColor(Color::NONE)
                    } else {
                        color.0
                    },
                ));

                // Post-cursor
                parent.spawn((TextSpan::new(values.2), font.0.clone()));
            })
            .id();

        let placeholder_font = placeholder
            .text_font
            .clone()
            .unwrap_or_else(|| font.0.clone());

        let placeholder_color = placeholder
            .text_color
            .unwrap_or_else(|| placeholder_color(&color.0));

        let placeholder_visible = inactive.0 && text_input.0.is_empty();

        let placeholder_text = commands
            .spawn((
                Text::new(&placeholder.value),
                TextLayout::new_with_linebreak(LineBreak::NoWrap),
                placeholder_font,
                placeholder_color,
                Name::new("TextInputPlaceholderInner"),
                TextInputPlaceholderInner,
                if placeholder_visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ))
            .id();

        let overflow_container = commands
            .spawn((
                Node {
                    overflow: Overflow::scroll_x(),
                    justify_content: JustifyContent::FlexEnd,
                    max_width: Val::Percent(100.),
                    ..default()
                },
                Name::new("TextInputOverflowContainer"),
            ))
            .id();

        commands.entity(overflow_container).add_child(text);
        commands
            .entity(trigger.entity())
            .add_children(&[overflow_container, placeholder_text]);

        // Prevent clicks from registering on UI elements underneath the text input.
        commands.entity(trigger.entity()).insert(FocusPolicy::Block);
    }
}

// Shows or hides the cursor based on the text input's [`TextInputInactive`] property.
fn show_hide_cursor(
    mut input_query: Query<
        (
            Entity,
            &TextInputTextColor,
            &mut TextInputCursorTimer,
            &TextInputInactive,
        ),
        Changed<TextInputInactive>,
    >,
    inner_text: InnerText,
    mut writer: TextUiWriter,
) {
    for (entity, color, mut cursor_timer, inactive) in &mut input_query {
        let Some(inner) = inner_text.inner_entity(entity) else {
            continue;
        };

        *writer.color(inner, 1) = if inactive.0 {
            TextColor(Color::NONE)
        } else {
            color.0
        };

        cursor_timer.timer.reset();
    }
}

// Blinks the cursor on a timer.
fn blink_cursor(
    mut input_query: Query<(
        Entity,
        &TextInputTextColor,
        &mut TextInputCursorTimer,
        Ref<TextInputInactive>,
    )>,
    inner_text: InnerText,
    mut writer: TextUiWriter,
    time: Res<Time>,
) {
    for (entity, color, mut cursor_timer, inactive) in &mut input_query {
        if inactive.0 {
            continue;
        }

        if cursor_timer.is_changed() && cursor_timer.should_reset {
            cursor_timer.timer.reset();
            cursor_timer.should_reset = false;

            if let Some(inner) = inner_text.inner_entity(entity) {
                *writer.color(inner, 1) = color.0;
            };

            continue;
        }

        if !cursor_timer.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(inner) = inner_text.inner_entity(entity) else {
            continue;
        };

        if writer.color(inner, 1).0 != Color::NONE {
            *writer.color(inner, 1) = TextColor(Color::NONE);
        } else {
            *writer.color(inner, 1) = color.0;
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
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}

fn update_style(
    mut input_query: Query<(Entity, &TextInputTextFont), Changed<TextInputTextFont>>,
    inner_text: InnerText,
    mut writer: TextUiWriter,
) {
    for (entity, font) in &mut input_query {
        let Some(inner) = inner_text.inner_entity(entity) else {
            continue;
        };

        *writer.font(inner, 0) = font.0.clone();
        *writer.font(inner, 1) = TextFont {
            font: CURSOR_HANDLE,
            ..font.0.clone()
        };
        *writer.font(inner, 2) = font.0.clone();
    }
}

fn update_color(
    mut input_query: Query<
        (Entity, &TextInputTextColor, &TextInputInactive),
        Changed<TextInputTextColor>,
    >,
    inner_text: InnerText,
    mut writer: TextUiWriter,
) {
    for (entity, color, inactive) in &mut input_query {
        let Some(inner) = inner_text.inner_entity(entity) else {
            continue;
        };
        *writer.color(inner, 0) = color.0;
        *writer.color(inner, 1) = if inactive.0 {
            TextColor(Color::NONE)
        } else {
            color.0
        };
        *writer.color(inner, 2) = color.0;
    }
}

fn get_section_values(value: &str, cursor_pos: usize) -> (String, String, String) {
    let before = value.chars().take(cursor_pos).collect();
    let after = value.chars().skip(cursor_pos).collect();

    // If the cursor is between two characters, use the zero-width cursor.
    let cursor = if cursor_pos >= value.chars().count() {
        "}".to_string()
    } else {
        "|".to_string()
    };

    (before, cursor, after)
}

fn remove_char_at(input: &str, index: usize) -> String {
    input
        .chars()
        .enumerate()
        .filter_map(|(i, c)| if i != index { Some(c) } else { None })
        .collect()
}

fn byte_pos(input: &str, char_pos: usize) -> usize {
    let mut char_indices = input.char_indices();
    char_indices
        .nth(char_pos)
        .map(|(pos, _)| pos)
        .unwrap_or(input.len())
}

fn masked_value(value: &str, mask: Option<char>) -> String {
    mask.map_or_else(
        || value.to_string(),
        |c| value.chars().map(|_| c).collect::<String>(),
    )
}

fn placeholder_color(color: &TextColor) -> TextColor {
    TextColor(color.with_alpha(color.alpha() * 0.25))
}
