use bevy::{
    asset::{load_internal_binary_asset, HandleUntyped},
    input::keyboard::KeyboardInput,
    prelude::*,
    reflect::TypeUuid,
    text::BreakLineOn,
};

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
            .add_systems(Update, (create, keyboard, cursor));
    }
}

const CURSOR_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Font::TYPE_UUID, 10482756907980398621);

#[derive(Component, Default)]
pub struct TextInput {
    pub text_style: TextStyle,
    /// The text input does not respond to keyboard events
    pub inactive: bool,
}
#[derive(Component)]
struct TextInputInner;

#[derive(Component)]
struct CursorTimer(Timer);
impl Default for CursorTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(0.5, TimerMode::Repeating))
    }
}

#[derive(Event)]
pub struct TextInputSubmitEvent {
    pub entity: Entity,
    pub value: String,
}

fn keyboard(
    mut events: EventReader<KeyboardInput>,
    mut character_events: EventReader<ReceivedCharacter>,
    text_input_query: Query<(Entity, &TextInput)>,
    mut text_query: Query<&mut Text, With<TextInputInner>>,
    children_query: Query<&Children>,
    mut submit_writer: EventWriter<TextInputSubmitEvent>,
) {
    if events.is_empty() && character_events.is_empty() {
        return;
    }

    for (input_entity, input) in &text_input_query {
        if input.inactive {
            continue;
        }

        for descendant in children_query.iter_descendants(input_entity) {
            if let Ok(mut text) = text_query.get_mut(descendant) {
                for event in events.iter() {
                    if event.state.is_pressed() {
                        continue;
                    };

                    match event.key_code {
                        Some(KeyCode::Left) => {
                            if let Some(behind) = text.sections[0].value.pop() {
                                text.sections[2].value.insert(0, behind);
                            }
                        }
                        Some(KeyCode::Right) => {
                            if !text.sections[2].value.is_empty() {
                                let ahead = text.sections[2].value.remove(0);
                                text.sections[0].value.push(ahead);
                            }
                        }
                        _ => {}
                    }
                }

                for event in character_events.iter() {
                    if event.char == '\u{7f}' {
                        text.sections[0].value.pop();
                        continue;
                    }

                    if event.char == '\r' {
                        submit_writer.send(TextInputSubmitEvent {
                            entity: input_entity,
                            value: format!("{}{}", text.sections[0].value, text.sections[2].value),
                        });
                        text.sections[0].value.clear();
                        text.sections[2].value.clear();
                        continue;
                    }

                    text.sections[0].value.push(event.char);
                }

                // If the cursor is between two characters, use the zero-width cursor.
                if text.sections[2].value.is_empty() {
                    text.sections[1].value = "}".to_string();
                } else {
                    text.sections[1].value = "|".to_string();
                }
            }
        }
    }
}

fn create(mut commands: Commands, query: Query<(Entity, &TextInput), Added<TextInput>>) {
    for (entity, input) in &query {
        commands
            .entity(entity)
            .insert((CursorTimer::default(), Interaction::None));

        let text = commands
            .spawn((
                TextBundle {
                    text: Text {
                        linebreak_behavior: BreakLineOn::NoWrap,
                        sections: vec![
                            // Pre-cursor
                            TextSection {
                                value: "".to_string(),
                                style: input.text_style.clone(),
                            },
                            // cursor
                            TextSection {
                                value: "}".to_string(),
                                style: TextStyle {
                                    font: CURSOR_HANDLE.typed(),
                                    ..input.text_style.clone()
                                },
                            },
                            // Post-cursor
                            TextSection {
                                value: "".to_string(),
                                style: input.text_style.clone(),
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

fn cursor(
    mut input_query: Query<(Entity, &TextInput, &mut CursorTimer)>,
    mut text_query: Query<&mut Text, With<TextInputInner>>,
    children_query: Query<&Children>,
    time: Res<Time>,
) {
    for (entity, text_input, mut timer) in &mut input_query {
        if !timer.0.tick(time.delta()).just_finished() {
            continue;
        }

        for descendant in children_query.iter_descendants(entity) {
            if let Ok(mut text) = text_query.get_mut(descendant) {
                if text.sections[1].style.color != Color::NONE {
                    text.sections[1].style.color = Color::NONE;
                } else {
                    text.sections[1].style.color = text_input.text_style.color;
                }
            }
        }
    }
}
