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
    ecs::{event::EventCursor, system::SystemParam},
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    render::camera::RenderTarget,
    text::{BreakLineOn, TextLayoutInfo},
    ui::FocusPolicy,
    window::{PrimaryWindow, WindowRef},
};

/// A Bevy `Plugin` providing the systems and assets required to make a [`TextInputBundle`] work.
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
            .observe(create)
            .add_systems(
                Update,
                (
                    keyboard,
                    update_value.after(keyboard),
                    blink_cursor,
                    show_hide_cursor,
                    update_style,
                    show_hide_placeholder,
                    scroll_with_cursor,
                )
                    .in_set(TextInputSystem),
            )
            .register_type::<TextInputSettings>()
            .register_type::<TextInputTextStyle>()
            .register_type::<TextInputInactive>()
            .register_type::<TextInputCursorTimer>()
            .register_type::<TextInputInner>()
            .register_type::<TextInputValue>()
            .register_type::<TextInputPlaceholder>()
            .register_type::<TextInputCursorPos>();
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
    /// A component containing the placeholder text that is displayed when the text input is empty and not focused.
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
    /// The style to use when rendering the placeholder text.
    ///
    /// If `None`, the text input style will be used with alpha value of `0.25`.
    pub text_style: Option<TextStyle>,
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
            submit_writer.send(TextInputSubmitEvent {
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
    mut inner_text: InnerText,
) {
    for (entity, text_input, settings, mut cursor_pos) in &mut input_query {
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

        set_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            cursor_pos.0,
            &mut text.sections,
        );
    }
}

fn scroll_with_cursor(
    mut inner_text_query: Query<
        (
            &TextLayoutInfo,
            &mut Style,
            &Node,
            &Parent,
            Option<&TargetCamera>,
        ),
        (With<TextInputInner>, Changed<TextLayoutInfo>),
    >,
    mut style_query: Query<(&Node, &mut Style), Without<TextInputInner>>,
    camera_query: Query<&Camera>,
    window_query: Query<&Window>,
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
) {
    for (layout, mut style, child_node, parent, target_camera) in inner_text_query.iter_mut() {
        let Ok((parent_node, mut parent_style)) = style_query.get_mut(parent.get()) else {
            continue;
        };

        match layout.glyphs.last().map(|g| g.section_index) {
            // no text -> do nothing
            None => return,
            // if cursor is at the end, position at FlexEnd so newly typed text does not take a frame to move into view
            Some(1) => {
                style.left = Val::Auto;
                parent_style.justify_content = JustifyContent::FlexEnd;
                return;
            }
            _ => (),
        }

        // if cursor is in the middle, we use FlexStart + `left` px for consistent behaviour when typing the middle
        let child_size = child_node.size().x;
        let parent_size = parent_node.size().x;

        let Some(cursor_pos) = layout
            .glyphs
            .iter()
            .find(|g| g.section_index == 1)
            .map(|p| p.position.x)
        else {
            continue;
        };

        // glyph positions are not adjusted for scale factor so we do that here
        let window_ref = match target_camera {
            Some(target) => {
                let Ok(camera) = camera_query.get(target.0) else {
                    continue;
                };

                match camera.target {
                    RenderTarget::Window(window_ref) => Some(window_ref),
                    _ => None,
                }
            }
            None => Some(WindowRef::Primary),
        };

        let scale_factor = match window_ref {
            Some(window_ref) => {
                let window = match window_ref {
                    WindowRef::Entity(w) => window_query.get(w).ok(),
                    WindowRef::Primary => primary_window_query.get_single().ok(),
                };

                let Some(window) = window else {
                    continue;
                };

                window.scale_factor()
            }
            None => 1.0,
        };
        let cursor_pos = cursor_pos / scale_factor;

        let box_pos = match style.left {
            Val::Px(px) => -px,
            _ => child_size - parent_size,
        };

        let relative_pos = cursor_pos - box_pos;

        if relative_pos < 0.0 || relative_pos > parent_size {
            let req_px = parent_size * 0.5 - cursor_pos;
            let req_px = req_px.clamp(parent_size - child_size, 0.0);
            style.left = Val::Px(req_px);
            parent_style.justify_content = JustifyContent::FlexStart;
        }
    }
}

fn create(
    trigger: Trigger<OnAdd, TextInputValue>,
    mut commands: Commands,
    query: Query<(
        &TextInputTextStyle,
        &TextInputValue,
        &TextInputCursorPos,
        &TextInputInactive,
        &TextInputSettings,
        &TextInputPlaceholder,
    )>,
) {
    if let Ok((style, text_input, cursor_pos, inactive, settings, placeholder)) =
        &query.get(trigger.entity())
    {
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

        set_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            cursor_pos.0,
            &mut sections,
        );

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
                Name::new("TextInputInner"),
                TextInputInner,
            ))
            .id();

        let placeholder_style = placeholder
            .text_style
            .clone()
            .unwrap_or_else(|| placeholder_style(&style.0));

        let placeholder_visible = inactive.0 && text_input.0.is_empty();

        let placeholder_text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: BreakLineOn::NoWrap,
                        sections: vec![TextSection {
                            value: placeholder.value.clone(),
                            style: placeholder_style,
                        }],
                        ..default()
                    },
                    visibility: if placeholder_visible {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    },
                    style: Style {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
                Name::new("TextInputPlaceholderInner"),
                TextInputPlaceholderInner,
            ))
            .id();

        let overflow_container = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        overflow: Overflow::clip(),
                        justify_content: JustifyContent::FlexEnd,
                        max_width: Val::Percent(100.),
                        ..default()
                    },
                    ..default()
                },
                Name::new("TextInputOverflowContainer"),
            ))
            .id();

        commands.entity(overflow_container).add_child(text);
        commands
            .entity(trigger.entity())
            .push_children(&[overflow_container, placeholder_text]);

        // Prevent clicks from registering on UI elements underneath the text input.
        commands.entity(trigger.entity()).insert(FocusPolicy::Block);
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
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}

fn update_style(
    mut input_query: Query<
        (Entity, &TextInputTextStyle, &TextInputInactive),
        Changed<TextInputTextStyle>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, style, inactive) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        text.sections[0].style = style.0.clone();
        text.sections[1].style = TextStyle {
            font: CURSOR_HANDLE,
            color: if inactive.0 {
                Color::NONE
            } else {
                style.0.color
            },
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

fn placeholder_style(style: &TextStyle) -> TextStyle {
    let color = style.color.with_alpha(style.color.alpha() * 0.25);
    TextStyle {
        color,
        ..style.clone()
    }
}
