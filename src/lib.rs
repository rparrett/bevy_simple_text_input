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

mod target_camera_helper;

use bevy::{
    ecs::{event::ManualEventReader, system::SystemParam},
    input::keyboard::{Key, KeyboardInput},
    prelude::*,
    text::{
        cosmic_text::{Action, Edit, Editor},
        BreakLineOn, CosmicBuffer, TextPipeline,
    },
    ui::FocusPolicy,
};
use once_cell::unsync::Lazy;
use target_camera_helper::TargetCameraHelper;

/// A Bevy `Plugin` providing the systems and assets required to make a [`TextInputBundle`] work.
pub struct TextInputPlugin;

/// Label for systems that update text inputs.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct TextInputSystem;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TextInputNavigationBindings>()
            .add_event::<TextInputSubmitEvent>()
            .observe(create)
            .add_systems(
                Update,
                (
                    keyboard,
                    update_value.after(keyboard),
                    blink_cursor,
                    set_positions,
                    update_style,
                    show_hide_placeholder,
                )
                    .in_set(TextInputSystem),
            )
            .register_type::<TextInputSettings>()
            .register_type::<TextInputTextStyle>()
            .register_type::<TextInputInactive>()
            .register_type::<TextInputCursorTimer>()
            .register_type::<TextInputInner>()
            .register_type::<TextInputValue>();
    }
}

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
    /// multiline
    pub multiline: bool,
    /// If true, text is not cleared after pressing enter.
    pub retain_on_submit: bool,
    /// Mask text with the provided character.
    pub mask_character: Option<char>,
}

/// text navigation actions that can be bound via TextInputNavigationBindings
#[derive(Debug)]
pub enum TextInputAction {
    /// char left
    CharLeft,
    /// char right
    CharRight,
    /// word left
    WordLeft,
    /// word right
    WordRight,
    /// start of line
    LineStart,
    /// end of line
    LineEnd,
    /// move up one line
    LineUp,
    /// move down one line
    LineDown,
    /// document start
    TextStart,
    /// document end
    TextEnd,
    /// backspace
    DeletePrev,
    /// delete
    DeleteNext,
    /// enter
    Submit,
    /// add a new line
    NewLine,
}
/// A resource in which key bindings can be specified. Bindings are given as a tuple of (Primary Key, Modifiers).
/// All modifiers must be held when the primary key is pressed to perform the action.
/// The first matching action in the list will be performed, so a binding that is the same as another with additional
/// modifier keys should be earlier in the vector to be applied.
#[derive(Resource)]
pub struct TextInputNavigationBindings(pub Vec<(TextInputAction, TextInputBinding)>);

/// A binding for text navigation
pub struct TextInputBinding {
    /// primary key
    key: KeyCode,
    /// required modifiers
    modifiers: Vec<KeyCode>,
}

impl TextInputBinding {
    /// new
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
            // TextStart/End must be before LineStart/End as they are the same but with modifiers
            (TextStart, TextInputBinding::new(Home, [ControlLeft])),
            (TextStart, TextInputBinding::new(Home, [ControlRight])),
            (TextEnd, TextInputBinding::new(End, [ControlLeft])),
            (TextEnd, TextInputBinding::new(End, [ControlRight])),
            (LineStart, TextInputBinding::new(Home, [])),
            (LineEnd, TextInputBinding::new(End, [])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [ControlLeft])),
            (WordLeft, TextInputBinding::new(ArrowLeft, [ControlRight])),
            (WordRight, TextInputBinding::new(ArrowRight, [ControlLeft])),
            (WordRight, TextInputBinding::new(ArrowRight, [ControlRight])),
            (CharLeft, TextInputBinding::new(ArrowLeft, [])),
            (CharRight, TextInputBinding::new(ArrowRight, [])),
            (LineUp, TextInputBinding::new(ArrowUp, [])),
            (LineDown, TextInputBinding::new(ArrowDown, [])),
            (DeletePrev, TextInputBinding::new(Backspace, [])),
            (DeletePrev, TextInputBinding::new(NumpadBackspace, [])),
            (DeleteNext, TextInputBinding::new(Delete, [])),
            // newline must be before submit as it is the same but with modifiers
            (NewLine, TextInputBinding::new(Enter, [ShiftLeft])),
            (NewLine, TextInputBinding::new(Enter, [ShiftRight])),
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
            (LineUp, TextInputBinding::new(ArrowUp, [])),
            (LineDown, TextInputBinding::new(ArrowDown, [])),
            (DeletePrev, TextInputBinding::new(Backspace, [])),
            (DeletePrev, TextInputBinding::new(NumpadBackspace, [])),
            (DeleteNext, TextInputBinding::new(Delete, [])),
            // newline must be before submit as it is the same but with modifiers
            (NewLine, TextInputBinding::new(Enter, [ShiftLeft])),
            (NewLine, TextInputBinding::new(Enter, [ShiftRight])),
            (NewLine, TextInputBinding::new(Enter, [AltLeft])),
            (NewLine, TextInputBinding::new(Enter, [AltRight])),
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
    buffer_query: Query<'w, 's, &'static mut CosmicBuffer, With<TextInputInner>>,
    children_query: Query<'w, 's, &'static Children>,
    cursor_query: Query<'w, 's, &'static mut Style, With<TextInputCursorDisplay>>,
}
impl<'w, 's> InnerText<'w, 's> {
    fn get_mut(&mut self, entity: Entity) -> Option<Mut<'_, Text>> {
        self.text_query.get_mut(self.inner_entity(entity)?).ok()
    }

    fn set_editor_buffer(&mut self, editor: &mut Editor<'static>, entity: Entity) {
        if let Some(buffer) = self
            .inner_entity(entity)
            .and_then(|inner| self.buffer_query.get_mut(inner).ok())
        {
            *editor.buffer_ref_mut() = buffer.0.clone().into();
        }
    }

    fn cursor_style(&mut self, entity: Entity) -> Option<&mut Style> {
        self.cursor_query
            .get_mut(
                self.children_query
                    .iter_descendants(entity)
                    .find(|d| self.cursor_query.get(*d).is_ok())?,
            )
            .ok()
            .map(Mut::into_inner)
    }

    fn inner_entity(&self, entity: Entity) -> Option<Entity> {
        self.children_query
            .iter_descendants(entity)
            .find(|descendant_entity| self.text_query.get(*descendant_entity).is_ok())
    }
}

#[allow(clippy::too_many_arguments)]
fn keyboard(
    key_input: Res<ButtonInput<KeyCode>>,
    input_events: Res<Events<KeyboardInput>>,
    input_reader: Local<ManualEventReader<KeyboardInput>>,
    mut text_input_query: Query<(
        Entity,
        &TextInputSettings,
        &TextInputInactive,
        &mut TextInputValue,
        &mut TextInputCursorTimer,
        &mut CosmicEditor,
    )>,
    mut submit_writer: EventWriter<TextInputSubmitEvent>,
    navigation: Res<TextInputNavigationBindings>,
    mut inner_text: InnerText,
    mut text_pipeline: ResMut<TextPipeline>,
) {
    if input_reader.clone().read(&input_events).next().is_none() {
        return;
    }

    let font_system = text_pipeline.font_system_mut();

    // collect actions that have all required modifiers held
    let valid_actions = navigation
        .0
        .iter()
        .filter(|(_, TextInputBinding { modifiers, .. })| {
            modifiers.iter().all(|m| key_input.pressed(*m))
        })
        .map(|(action, TextInputBinding { key, .. })| (*key, action));

    for (input_entity, settings, inactive, mut text_input, mut cursor_timer, mut editor) in
        &mut text_input_query
    {
        if inactive.0 {
            continue;
        }

        let mut submitted_value = None;

        // use a lazy cell to avoid initializing the editor if not required (copying the buffer is expensive)
        let mut editor = Lazy::new(|| {
            inner_text.set_editor_buffer(&mut editor.0, input_entity);
            editor.0.start_change();
            editor
        });

        for input in input_reader.clone().read(&input_events) {
            if !input.state.is_pressed() {
                continue;
            };

            if let Some((_, action)) = valid_actions
                .clone()
                .find(|(key, _)| *key == input.key_code)
            {
                use TextInputAction::*;
                let mut timer_should_reset = true;
                let editor_action = match action {
                    CharLeft => Some(Action::Motion(bevy::text::cosmic_text::Motion::Left)),
                    CharRight => Some(Action::Motion(bevy::text::cosmic_text::Motion::Right)),
                    TextStart => Some(Action::Motion(bevy::text::cosmic_text::Motion::BufferStart)),
                    TextEnd => Some(Action::Motion(bevy::text::cosmic_text::Motion::BufferEnd)),
                    LineStart => Some(Action::Motion(bevy::text::cosmic_text::Motion::Home)),
                    LineEnd => Some(Action::Motion(bevy::text::cosmic_text::Motion::End)),
                    WordLeft => Some(Action::Motion(bevy::text::cosmic_text::Motion::LeftWord)),
                    WordRight => Some(Action::Motion(bevy::text::cosmic_text::Motion::RightWord)),
                    LineUp => Some(Action::Motion(bevy::text::cosmic_text::Motion::Up)),
                    LineDown => Some(Action::Motion(bevy::text::cosmic_text::Motion::Down)),
                    DeletePrev => Some(Action::Backspace),
                    DeleteNext => Some(Action::Delete),
                    NewLine => {
                        println!("newline");
                        editor.0.insert_string("\n", None);
                        None
                    }
                    Submit => {
                        if settings.retain_on_submit {
                            submitted_value = Some(text_input.0.clone());
                        } else {
                            submitted_value = Some(std::mem::take(&mut text_input.0));
                        };
                        timer_should_reset = false;
                        Some(Action::Motion(bevy::text::cosmic_text::Motion::BufferStart))
                    }
                };

                if let Some(action) = editor_action {
                    editor.0.action(font_system, action);
                }

                cursor_timer.should_reset |= timer_should_reset;
                continue;
            }

            match input.logical_key {
                Key::Space => {
                    editor.0.insert_string(" ", None);
                    cursor_timer.should_reset = true;
                }
                Key::Character(ref s) => {
                    editor.0.insert_string(s, None);
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

        if let Ok(mut editor) = Lazy::into_value(editor) {
            if let Some(_change) = editor.0.finish_change() {
                // todo record changes for undo buffer
                editor.0.shape_as_needed(font_system, false);
                editor.0.with_buffer(|b| {
                    text_input.0 = b
                        .lines
                        .iter()
                        .map(|line| line.text())
                        .collect::<Vec<_>>()
                        .join("\n");
                })
            }
            println!("edit ->`{}`", text_input.0);
        }
    }
}

fn update_value(
    mut input_query: Query<
        (Entity, Ref<TextInputValue>, &TextInputSettings),
        Changed<TextInputValue>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, text_input, settings) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        set_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            &mut text.sections,
        );
    }
}

#[derive(Component)]
struct CosmicEditor(Editor<'static>);

fn create(
    trigger: Trigger<OnAdd, TextInputValue>,
    mut commands: Commands,
    query: Query<(
        &TextInputTextStyle,
        &TextInputValue,
        &TextInputInactive,
        &TextInputSettings,
        &TextInputPlaceholder,
    )>,
) {
    if let Ok((style, text_input, inactive, settings, placeholder)) = &query.get(trigger.entity()) {
        let mut sections = vec![
            // Pre-cursor
            TextSection {
                style: style.0.clone(),
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
            &mut sections,
        );

        let text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: if settings.multiline {
                            BreakLineOn::WordBoundary
                        } else {
                            BreakLineOn::NoWrap
                        },
                        sections,
                        ..default()
                    },
                    style: Style {
                        min_height: Val::Percent(100.0),
                        ..Default::default()
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
                        align_items: AlignItems::FlexEnd,
                        max_width: Val::Percent(100.),
                        min_height: Val::Percent(100.0),
                        max_height: Val::Percent(100.0),
                        ..default()
                    },
                    ..Default::default()
                },
                Name::new("TextInputOverflowContainer"),
            ))
            .id();

        let cursor = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        display: Display::None,
                        width: Val::Px(1f32.max(style.0.font_size * 0.05)),
                        height: Val::Px(style.0.font_size),
                        position_type: PositionType::Absolute,
                        ..Default::default()
                    },
                    background_color: Color::WHITE.into(),
                    ..Default::default()
                },
                TextInputCursorDisplay,
            ))
            .id();

        commands.entity(overflow_container).add_child(text);
        commands.entity(trigger.entity()).push_children(&[
            overflow_container,
            placeholder_text,
            cursor,
        ]);

        commands
            .entity(trigger.entity())
            // Prevent clicks from registering on UI elements underneath the text input.
            .insert(FocusPolicy::Block)
            .insert(CosmicEditor(Editor::new(CosmicBuffer::default().0)));
    }
}

#[derive(Component)]
struct TextInputCursorDisplay;

// Sets the container position and cursor position.
// Shows or hides the cursor based on the text input's [`TextInputInactive`] property.
fn set_positions(
    mut input_query: Query<
        (
            Entity,
            &mut TextInputCursorTimer,
            &TextInputInactive,
            &mut CosmicEditor,
        ),
        Or<(Changed<TextInputInactive>, Changed<CosmicEditor>)>,
    >,
    mut inner_text: InnerText,
    mut inner_style_query: Query<
        (&mut Style, &Node, &Parent),
        (Without<TextInputCursorDisplay>, With<TextInputInner>),
    >,
    mut container_style_query: Query<
        (&mut Style, &Node),
        (Without<TextInputCursorDisplay>, Without<TextInputInner>),
    >,
    mut text_pipeline: ResMut<TextPipeline>,
    camera_helper: TargetCameraHelper,
) {
    let px = |val: Val| match val {
        Val::Px(px) => px,
        _ => 0.0,
    };

    for (entity, mut cursor_timer, inactive, mut editor) in &mut input_query {
        let Some(inner_entity) = inner_text.inner_entity(entity) else {
            continue;
        };

        let Ok((mut container_style, child_node, parent)) = inner_style_query.get_mut(inner_entity)
        else {
            continue;
        };

        let Ok((mut parent_style, parent_node)) = container_style_query.get_mut(parent.get())
        else {
            continue;
        };

        let editor = editor.bypass_change_detection();

        editor
            .0
            .shape_as_needed(text_pipeline.font_system_mut(), false);
        let cursor_position = editor.0.cursor_position().unwrap_or((0, 0));

        let Some(cursor_style) = inner_text.cursor_style(entity) else {
            continue;
        };

        let Some(camera_props) = camera_helper.get_props(inner_entity) else {
            continue;
        };

        let cursor_position =
            IVec2::new(cursor_position.0, cursor_position.1).as_vec2() / camera_props.scale_factor;

        let child_size = child_node.size();
        let parent_size = parent_node.size();

        let box_pos_x = match container_style.left {
            Val::Px(px) => -px,
            _ => child_size.x - parent_size.x,
        };

        let box_pos_y = match container_style.top {
            Val::Px(px) => -px,
            _ => child_size.y - parent_size.y,
        };

        let mut relative_cursor_position = cursor_position - Vec2::new(box_pos_x, box_pos_y);
        let cursor_size = Vec2::new(px(cursor_style.width), px(cursor_style.height));

        if relative_cursor_position.cmplt(Vec2::ZERO).any()
            || (relative_cursor_position + cursor_size)
                .cmpgt(parent_size)
                .any()
        {
            let req_px = parent_size * 0.5 - cursor_position;
            let req_px = req_px.clamp(parent_size - child_size, Vec2::ZERO);
            container_style.left = Val::Px(req_px.x);
            container_style.top = Val::Px(req_px.y);
            parent_style.justify_content = JustifyContent::FlexStart;
            parent_style.align_items = AlignItems::FlexStart;

            relative_cursor_position = cursor_position + req_px;
        }

        cursor_style.display = if inactive.0 {
            Display::None
        } else {
            Display::Flex
        };

        cursor_style.left = Val::Px(relative_cursor_position.x + px(cursor_style.height) * 0.07);
        cursor_style.top = Val::Px(relative_cursor_position.y + px(cursor_style.height) * 0.2);

        cursor_timer.timer.reset();
    }
}

// Blinks the cursor on a timer.
fn blink_cursor(
    mut input_query: Query<(Entity, &mut TextInputCursorTimer, Ref<TextInputInactive>)>,
    mut inner_text: InnerText,
    time: Res<Time>,
) {
    for (entity, mut cursor_timer, inactive) in &mut input_query {
        if inactive.0 {
            continue;
        }

        if cursor_timer.is_changed() && cursor_timer.should_reset {
            cursor_timer.timer.reset();
            cursor_timer.should_reset = false;

            continue;
        }

        if !cursor_timer.timer.tick(time.delta()).just_finished() {
            continue;
        }

        let Some(style) = inner_text.cursor_style(entity) else {
            continue;
        };

        style.display = match style.display {
            Display::Flex => Display::None,
            Display::None => Display::Flex,
            _ => unreachable!(),
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
    mut input_query: Query<(Entity, &TextInputTextStyle, &mut TextInputInactive), Changed<TextInputTextStyle>>,
    mut inner_text: InnerText,
) {
    for (entity, style, mut inactive) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        text.sections[0].style = style.0.clone();

        let Some(cursor) = inner_text.cursor_style(entity) else {
            continue;
        };

        cursor.width = Val::Px(1f32.max(style.0.font_size * 0.05));
        cursor.height = Val::Px(style.0.font_size);

        // mark so other systems update correctly
        inactive.set_changed()
    }
}

fn set_section_values(value: &str, sections: &mut [TextSection]) {
    sections[0].value = value.to_owned();
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
