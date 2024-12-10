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
        cosmic_text::{Action, Change, Cursor, Edit, Editor, Selection},
        BreakLineOn, CosmicBuffer, TextPipeline,
    },
    ui::FocusPolicy,
};
use once_cell::unsync::Lazy;
use target_camera_helper::TargetCameraHelper;

#[cfg(feature = "clipboard")]
use copypasta::{ClipboardContext, ClipboardProvider};

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
                    blink_cursor,
                    set_positions,
                    set_selection,
                    show_hide_placeholder,
                    update_style,
                    keyboard,
                    update_value,
                )
                    .chain()
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
    /// selection colors
    pub selection_style: TextInputSelectionStyle,
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

    /// Returns this [`TextInputBundle`] with a new [`TextInputSelectionStyle`] containing the provided colors.
    pub fn with_selection_style(mut self, color: Option<Color>, background: Option<Color>) -> Self {
        self.selection_style = TextInputSelectionStyle { color, background };
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

/// selection color and background color
#[derive(Component, Default, Reflect)]
pub struct TextInputSelectionStyle {
    /// text color for selected text
    pub color: Option<Color>,
    /// background color for selected text
    pub background: Option<Color>,
}

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
    /// select full buffer
    SelectAll,
    /// cut
    CutAction,
    /// copy
    CopyAction,
    /// pasta
    PasteAction,
    /// undo
    UndoAction,
    /// redo
    RedoAction,
}
/// A resource in which key bindings can be specified. Bindings are given as a tuple of (Primary Key, Modifiers).
/// All modifiers must be held when the primary key is pressed to perform the action.
/// The first matching action in the list will be performed, so a binding that is the same as another with additional
/// modifier keys should be earlier in the vector to be applied.
#[derive(Resource)]
pub struct TextInputNavigationBindings {
    /// list of bindings
    pub action_bindings: Vec<(TextInputAction, TextInputBinding)>,
    /// modifiers to imply selection action
    pub selection_modifiers: Vec<KeyCode>,
}

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
        Self {
            action_bindings: vec![
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
                (SelectAll, TextInputBinding::new(KeyA, [ControlLeft])),
                (SelectAll, TextInputBinding::new(KeyA, [ControlRight])),
                (CutAction, TextInputBinding::new(KeyX, [ControlLeft])),
                (CutAction, TextInputBinding::new(KeyX, [ControlRight])),
                (CopyAction, TextInputBinding::new(KeyC, [ControlLeft])),
                (CopyAction, TextInputBinding::new(KeyC, [ControlRight])),
                (PasteAction, TextInputBinding::new(KeyV, [ControlLeft])),
                (PasteAction, TextInputBinding::new(KeyV, [ControlRight])),
                (UndoAction, TextInputBinding::new(KeyZ, [ControlLeft])),
                (UndoAction, TextInputBinding::new(KeyZ, [ControlRight])),
                (RedoAction, TextInputBinding::new(KeyY, [ControlLeft])),
                (RedoAction, TextInputBinding::new(KeyY, [ControlRight])),
            ],
            selection_modifiers: vec![ShiftLeft, ShiftRight],
        }
    }
}

#[cfg(target_os = "macos")]
impl Default for TextInputNavigationBindings {
    fn default() -> Self {
        use KeyCode::*;
        use TextInputAction::*;
        Self {
            action_bindings: vec![
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
                (SelectAll, TextInputBinding::new(KeyA, [SuperLeft])),
                (SelectAll, TextInputBinding::new(KeyA, [SuperRight])),
                (CutAction, TextInputBinding::new(KeyX, [SuperLeft])),
                (CutAction, TextInputBinding::new(KeyX, [SuperRight])),
                (CopyAction, TextInputBinding::new(KeyC, [SuperLeft])),
                (CopyAction, TextInputBinding::new(KeyC, [SuperRight])),
                (PasteAction, TextInputBinding::new(KeyV, [SuperLeft])),
                (PasteAction, TextInputBinding::new(KeyV, [SuperRight])),
                // redo must be before undo as it is the same but with modifiers
                (
                    RedoAction,
                    TextInputBinding::new(KeyY, [SuperLeft, ShiftLeft]),
                ),
                (
                    RedoAction,
                    TextInputBinding::new(KeyY, [SuperRight, ShiftLeft]),
                ),
                (
                    RedoAction,
                    TextInputBinding::new(KeyY, [SuperLeft, ShiftRight]),
                ),
                (
                    RedoAction,
                    TextInputBinding::new(KeyY, [SuperRight, ShiftRight]),
                ),
                (UndoAction, TextInputBinding::new(KeyZ, [SuperLeft])),
                (UndoAction, TextInputBinding::new(KeyZ, [SuperRight])),
            ],
            selection_modifiers: vec![ShiftLeft, ShiftRight],
        }
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
    mut input_reader: Local<ManualEventReader<KeyboardInput>>,
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
        .action_bindings
        .iter()
        .filter(|(_, TextInputBinding { modifiers, .. })| {
            modifiers.iter().all(|m| key_input.pressed(*m))
        })
        .map(|(action, TextInputBinding { key, .. })| (*key, action));

    let select = key_input.any_pressed(navigation.selection_modifiers.iter().copied());

    for (input_entity, settings, inactive, mut text_input, mut cursor_timer, mut editor) in
        &mut text_input_query
    {
        if inactive.0 {
            continue;
        }

        let mut submitted_value = None;
        let mut is_undo_redo = false;

        // use a lazy cell to avoid initializing the editor if not required (copying the buffer is expensive)
        let mut editor = Lazy::new(|| {
            inner_text.set_editor_buffer(&mut editor.editor, input_entity);
            // we need to reset the cursor position if it's invalid, else some actions (backspace) will panic
            if editor.editor.cursor_position().is_none() {
                editor.editor.set_cursor(Cursor::default());
            }
            editor.editor.start_change();
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
                let mut select = select;

                if select && editor.editor.selection() == Selection::None {
                    let cursor = editor.editor.cursor();
                    editor.editor.set_selection(Selection::Normal(cursor));
                }

                use bevy::text::cosmic_text::Motion;
                use TextInputAction::*;
                let mut timer_should_reset = true;
                let editor_action = match action {
                    CharLeft => Some(Action::Motion(Motion::Left)),
                    CharRight => Some(Action::Motion(Motion::Right)),
                    TextStart => Some(Action::Motion(Motion::BufferStart)),
                    TextEnd => Some(Action::Motion(Motion::BufferEnd)),
                    LineStart => Some(Action::Motion(Motion::Home)),
                    LineEnd => Some(Action::Motion(Motion::End)),
                    WordLeft => Some(Action::Motion(Motion::LeftWord)),
                    WordRight => Some(Action::Motion(Motion::RightWord)),
                    LineUp => Some(Action::Motion(Motion::Up)),
                    LineDown => Some(Action::Motion(Motion::Down)),
                    DeletePrev => Some(Action::Backspace),
                    DeleteNext => Some(Action::Delete),
                    NewLine => settings.multiline.then_some(Action::Enter),
                    Submit => {
                        if settings.retain_on_submit {
                            submitted_value = Some(text_input.0.clone());
                        } else {
                            submitted_value = Some(std::mem::take(&mut text_input.0));
                        };
                        timer_should_reset = false;
                        Some(Action::Motion(Motion::BufferStart))
                    }
                    SelectAll => {
                        editor
                            .editor
                            .set_selection(Selection::Normal(Cursor::default()));
                        select = true;
                        Some(Action::Motion(Motion::BufferEnd))
                    }
                    CutAction | CopyAction => {
                        #[cfg(feature = "clipboard")]
                        {
                            if let Some(selection) = editor.editor.copy_selection() {
                                if let Ok(mut ctx) = ClipboardContext::new() {
                                    if let Err(e) = ctx.set_contents(selection) {
                                        warn!("failed to copy : {e}");
                                    }
                                }
                            }
                        }

                        if let CutAction = action {
                            editor.editor.delete_selection();
                        } else {
                            // avoid clearing selection on copy
                            select = true;
                        }

                        None
                    }
                    PasteAction => {
                        #[cfg(feature = "clipboard")]
                        if let Ok(mut ctx) = ClipboardContext::new() {
                            if let Ok(selection) = ctx.get_contents() {
                                editor.editor.insert_string(&selection, None);
                            }
                        }

                        None
                    }
                    UndoAction => {
                        if let Some(mut undo) = editor.undo.pop() {
                            undo.reverse();
                            editor.editor.finish_change();
                            editor.editor.apply_change(&undo);
                            editor.editor.start_change();
                            editor.redo.push(undo);
                        }

                        is_undo_redo = true;
                        None
                    }
                    RedoAction => {
                        if let Some(mut redo) = editor.redo.pop() {
                            redo.reverse();
                            editor.editor.finish_change();
                            editor.editor.apply_change(&redo);
                            editor.editor.start_change();
                            editor.undo.push(redo);
                        }

                        is_undo_redo = true;
                        None
                    }
                };

                if let Some(action) = editor_action {
                    editor.editor.action(font_system, action);
                }

                if !select {
                    editor.editor.set_selection(Selection::None);
                }

                cursor_timer.should_reset |= timer_should_reset;
                continue;
            }

            match input.logical_key {
                Key::Space => {
                    editor.editor.insert_string(" ", None);
                    cursor_timer.should_reset = true;
                }
                Key::Character(ref s) => {
                    editor.editor.insert_string(s, None);
                    cursor_timer.should_reset = true;
                }
                _ => (),
            }
        }

        let mut submitted = false;
        if let Some(value) = submitted_value {
            submit_writer.send(TextInputSubmitEvent {
                entity: input_entity,
                value,
            });
            submitted = true;
        }

        if let Ok(mut editor) = Lazy::into_value(editor) {
            if let Some(change) = editor.editor.finish_change() {
                if !change.items.is_empty() && !is_undo_redo {
                    editor.redo.clear();
                    editor.undo.push(change);
                }
            }
            editor.editor.shape_as_needed(font_system, false);
            if !submitted {
                editor.editor.with_buffer(|b| {
                    text_input.0 = b
                        .lines
                        .iter()
                        .map(|line| format!("{}{}", line.text(), line.ending().as_str()))
                        .collect::<Vec<_>>().join("");
                });
            }
            debug!("edit -> `{}`", text_input.0);
            debug!("select -> `{:?}`", editor.editor.copy_selection());
            debug!("undo -> {:?}", editor.undo);
            debug!("redo -> {:?}", editor.redo);
            editor.selection_bounds = editor.editor.selection_bounds().map(|(from, to)| {
                let index = |c: Cursor| -> usize {
                    editor.editor.with_buffer(|b| {
                        let mut lines = b.lines.iter();
                        let prior_sum: usize = lines
                            .by_ref()
                            .take(c.line)
                            .map(|line| line.text().len() + 1)
                            .sum();
                        let line_sum = lines
                            .next()
                            .map(|line| {
                                line.text()
                                    .char_indices()
                                    .enumerate()
                                    .find(|(_, ci)| ci.0 == c.index)
                                    .map(|(ix, _)| ix)
                                    .unwrap_or(line.text().len())
                            })
                            .unwrap_or(0);
                        prior_sum + line_sum
                    })
                };

                (index(from), index(to))
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
            &CosmicEditor,
        ),
        Changed<TextInputValue>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, text_input, settings, editor) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        set_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            &mut text.sections,
            editor.selection_bounds,
        );
    }
}

#[derive(Component)]
struct CosmicEditor {
    editor: Editor<'static>,
    selection_bounds: Option<(usize, usize)>,
    undo: Vec<Change>,
    redo: Vec<Change>,
}

impl CosmicEditor {
    fn new() -> Self {
        Self {
            editor: Editor::new(CosmicBuffer::default().0),
            selection_bounds: None,
            undo: Vec::default(),
            redo: Vec::default(),
        }
    }
}

#[derive(Component)]
struct TextInputContainer;

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
            // Pre-selection
            TextSection {
                style: style.0.clone(),
                ..default()
            },
            // selection
            TextSection {
                style: style.0.clone(),
                ..default()
            },
            // Post-selection
            TextSection {
                style: style.0.clone(),
                ..default()
            },
        ];

        set_section_values(
            &masked_value(&text_input.0, settings.mask_character),
            &mut sections,
            None,
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

        let selection_hilight = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        display: Display::Flex,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                    z_index: ZIndex::Local(-1),
                    ..Default::default()
                },
                TextInputSelection,
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
                    background_color: style.0.color.into(),
                    ..Default::default()
                },
                TextInputCursorDisplay,
            ))
            .id();

        let container = commands
            .spawn((NodeBundle::default(), TextInputContainer))
            .push_children(&[text, selection_hilight, cursor])
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
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::FlexEnd,
                        min_width: Val::Percent(100.),
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

        commands.entity(overflow_container).add_child(container);
        commands
            .entity(trigger.entity())
            .push_children(&[overflow_container, placeholder_text]);

        commands
            .entity(trigger.entity())
            // Prevent clicks from registering on UI elements underneath the text input.
            .insert(FocusPolicy::Block)
            .insert(CosmicEditor::new());
    }
}

#[derive(Component)]
struct TextInputCursorDisplay;

#[derive(Component)]
struct RetryPositions;

// Sets the container position and cursor position.
// Shows or hides the cursor based on the text input's [`TextInputInactive`] property.
fn set_positions(
    mut commands: Commands,
    mut input_query: Query<
        (
            Entity,
            &mut TextInputCursorTimer,
            &TextInputInactive,
            &mut CosmicEditor,
        ),
        Or<(
            Changed<TextInputInactive>,
            Changed<TextInputTextStyle>,
            Changed<CosmicEditor>,
            With<RetryPositions>,
        )>,
    >,
    mut inner_text: InnerText,
    mut inner_style_query: Query<
        (&mut Style, &Node),
        (Without<TextInputCursorDisplay>, With<TextInputContainer>),
    >,
    mut container_style_query: Query<
        (&mut Style, &Node),
        (Without<TextInputCursorDisplay>, Without<TextInputContainer>),
    >,
    children: Query<&Children>,
    camera_helper: TargetCameraHelper,
) {
    let px = |val: Val| match val {
        Val::Px(px) => px,
        _ => 0.0,
    };

    for (entity, mut cursor_timer, inactive, mut editor) in &mut input_query {
        commands.entity(entity).remove::<RetryPositions>();
        let Some(inner_entity) = inner_text.inner_entity(entity) else {
            continue;
        };

        let Some((mut container_style, child_node)) = children
            .iter_descendants(entity)
            .find(|e| inner_style_query.get(*e).is_ok())
            .and_then(|e| inner_style_query.get_mut(e).ok())
        else {
            continue;
        };

        let Some((mut parent_style, parent_node)) = children
            .iter_descendants(entity)
            .find(|e| container_style_query.get(*e).is_ok())
            .and_then(|e| container_style_query.get_mut(e).ok())
        else {
            continue;
        };

        let editor = editor.bypass_change_detection();
        inner_text.set_editor_buffer(&mut editor.editor, entity);

        let cursor_position = editor.editor.cursor_position();

        if cursor_position.is_none() {
            // sometimes it just fails ... retry next frame after copying over the original
            // (we don't have enough info here to perform the layout ourselves)
            inner_text.set_editor_buffer(&mut editor.editor, entity);
            commands.entity(entity).insert(RetryPositions);
            return;
        }

        let cursor_position = cursor_position.unwrap_or((0, 0));

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

        let relative_cursor_position = cursor_position - Vec2::new(box_pos_x, box_pos_y);
        let cursor_size = Vec2::new(px(cursor_style.width) + 1.0, px(cursor_style.height) + 1.0);

        if relative_cursor_position.cmplt(Vec2::ZERO).any()
            || (relative_cursor_position + cursor_size)
                .cmpgt(parent_size)
                .any()
        {
            let req_px = parent_size * 0.5 - cursor_position;
            let req_px = req_px.clamp(parent_size - child_size - cursor_size * Vec2::X, Vec2::ZERO);
            container_style.left = Val::Px(req_px.x);
            container_style.top = Val::Px(req_px.y);
            parent_style.justify_content = JustifyContent::FlexStart;
            parent_style.align_items = AlignItems::FlexStart;
        }

        cursor_style.display = if inactive.0 {
            Display::None
        } else {
            Display::Flex
        };

        cursor_style.left = Val::Px(cursor_position.x + px(cursor_style.height) * 0.03);
        cursor_style.top = Val::Px(cursor_position.y + px(cursor_style.height) * 0.1);

        cursor_timer.timer.reset();
    }
}

#[derive(Component)]
struct TextInputSelection;

fn set_selection(
    mut query: Query<(Entity, &mut CosmicEditor, &TextInputSelectionStyle), Changed<CosmicEditor>>,
    children: Query<&Children>,
    sel: Query<&TextInputSelection>,
    mut commands: Commands,
    mut text_pipeline: ResMut<TextPipeline>,
) {
    for (entity, mut editor, style) in query.iter_mut() {
        let Some(selection) = children
            .iter_descendants(entity)
            .find(|c| sel.get(*c).is_ok())
        else {
            debug!("no sel");
            continue;
        };

        let editor = editor.bypass_change_detection();
        commands.entity(selection).despawn_descendants();

        if let Some((from, to)) = editor.editor.selection_bounds() {
            let mut segments = Vec::default();
            editor.editor.with_buffer_mut(|b| {
                b.shape_until_cursor(text_pipeline.font_system_mut(), to, false);
                let mut segment_y = f32::NEG_INFINITY;
                let runs = b
                    .layout_runs()
                    .skip_while(|run| run.line_i < from.line)
                    .take_while(|run| run.line_i <= to.line);

                for run in runs {
                    let glyphs = run
                        .glyphs
                        .iter()
                        .skip_while(|g| run.line_i == from.line && g.start < from.index)
                        .take_while(|g| run.line_i < to.line || g.end <= to.index);

                    for glyph in glyphs {
                        debug!("g: {},{}", glyph.x, glyph.y);
                        if run.line_top + glyph.y != segment_y {
                            segments.push(Vec4::new(
                                glyph.x,
                                run.line_top + glyph.y,
                                glyph.w,
                                run.line_height,
                            ));
                            segment_y = run.line_top + glyph.y;
                        } else {
                            let segment = segments.last_mut().unwrap();
                            segment.z = glyph.x + glyph.w - segment.x;
                        }
                    }
                }
            });

            commands.entity(selection).with_children(|c| {
                for segment in segments {
                    c.spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(segment.x),
                            top: Val::Px(segment.y),
                            width: Val::Px(segment.z),
                            height: Val::Px(segment.w),
                            ..Default::default()
                        },
                        background_color: style
                            .background
                            .unwrap_or(Color::srgb(0.3, 0.3, 1.0))
                            .into(),
                        ..Default::default()
                    });
                }
            });
        }
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
        };

        debug!("{:?}", style.display);
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
        (
            Entity,
            &TextInputTextStyle,
            &mut TextInputSelectionStyle,
            &mut TextInputInactive,
        ),
        Changed<TextInputTextStyle>,
    >,
    mut inner_text: InnerText,
) {
    for (entity, style, selection_style, mut inactive) in &mut input_query {
        let Some(mut text) = inner_text.get_mut(entity) else {
            continue;
        };

        text.sections[0].style = style.0.clone();
        text.sections[1].style = TextStyle {
            color: selection_style.color.unwrap_or(style.0.color),
            ..style.0.clone()
        };
        text.sections[2].style = style.0.clone();

        let Some(cursor) = inner_text.cursor_style(entity) else {
            continue;
        };

        cursor.width = Val::Px(1f32.max(style.0.font_size * 0.05));
        cursor.height = Val::Px(style.0.font_size);

        // mark so other systems update correctly
        inactive.set_changed()
    }
}

fn set_section_values(value: &str, sections: &mut [TextSection], bounds: Option<(usize, usize)>) {
    match bounds {
        Some((from, to)) => {
            let start = from.min(to);
            let end = from.max(to);
            sections[0].value = value[0..start].to_owned();
            sections[1].value = value[start..end].to_owned();
            sections[2].value = value[end..].to_owned();
        }
        None => {
            sections[0].value = value.to_owned();
            sections[1].value.clear();
            sections[2].value.clear();
        }
    }
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
