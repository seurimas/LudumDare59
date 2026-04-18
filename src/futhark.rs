use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use crate::GameAssets;

pub const LETTERS: [char; 24] = [
    'f', 'u', '7', 'a', 'r', 'k', 'g', 'w', 'h', 'n', 'i', 'j', 'A', 'p', 'z', 's', 't', 'b', 'e',
    'm', 'l', 'N', 'd', 'o',
];

const KEYBOARD_ROW_OFFSETS: [f32; 3] = [0.0, 40.0, 80.0];
const KEYBOARD_TOP_ROW: [usize; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];
const KEYBOARD_MIDDLE_ROW: [usize; 8] = [10, 11, 12, 0, 13, 14, 15, 16];
const KEYBOARD_BOTTOM_ROW: [usize; 7] = [17, 18, 19, 20, 21, 22, 23];

#[derive(Component)]
pub struct FutharkKeyboard;

#[derive(Component)]
pub struct FutharkKeyButton {
    pub index: usize,
}

#[derive(Component)]
pub struct FutharkKeyLabel {
    pub index: usize,
}

#[derive(Component)]
pub struct FutharkKeyBackground;

#[derive(Component)]
pub struct FutharkKeyRuneVisual;

#[derive(Component)]
pub struct FutharkKeyLetterVisual;

const SPRITE_KEYBOARD_BG: usize = 254;
const SPRITE_RUNE_OFFSET: usize = 24;

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq)]
pub enum FutharkKeyboardLegendMode {
    #[default]
    Runes,
    Letters,
}

#[derive(Resource, Clone, Copy)]
pub struct FutharkKeyboardAnimationSpeed {
    pub hue_degrees_per_second: f32,
}

impl Default for FutharkKeyboardAnimationSpeed {
    fn default() -> Self {
        Self {
            hue_degrees_per_second: 60.0,
        }
    }
}

#[derive(Message)]
pub struct TypedFutharkInput(pub char);

pub fn configure_futhark_keyboard(app: &mut App) {
    app.init_resource::<FutharkKeyboardLegendMode>();
    app.init_resource::<FutharkKeyboardAnimationSpeed>();
    app.add_message::<TypedFutharkInput>();
}

pub fn play_futhark_key_sound(
    mut typed_futhark_input: MessageReader<TypedFutharkInput>,
    game_assets: Res<crate::GameAssets>,
    audio_sources: Res<Assets<AudioSource>>,
    sound_configs: Res<Assets<crate::audio_params::FutharkSoundConfig>>,
    mut processed_audios: ResMut<Assets<crate::audio_params::ProcessedAudio>>,
    mut commands: Commands,
) {
    for event in typed_futhark_input.read() {
        let Some(index) = letter_to_index(event.0) else {
            continue;
        };
        let Some(raw) = game_assets.futhark_sounds.get(index) else {
            continue;
        };
        let Some(source) = audio_sources.get(&raw.clone().typed::<AudioSource>()) else {
            continue;
        };

        let config = sound_configs.get(&game_assets.futhark_sound_params);
        let params = crate::audio_params::pick_params(config, index);
        let handle = processed_audios.add(crate::audio_params::ProcessedAudio {
            bytes: source.bytes.clone(),
            params,
        });
        commands.spawn(AudioPlayer::<crate::audio_params::ProcessedAudio>(handle));
    }
}

pub fn index_to_letter(index: usize) -> Option<char> {
    LETTERS.get(index).copied()
}

pub fn letter_to_index(letter: char) -> Option<usize> {
    LETTERS
        .iter()
        .position(|mapped_letter| *mapped_letter == letter)
}

pub fn keyboard_rows_by_index() -> [Vec<usize>; 3] {
    [
        KEYBOARD_TOP_ROW.to_vec(),
        KEYBOARD_MIDDLE_ROW.to_vec(),
        KEYBOARD_BOTTOM_ROW.to_vec(),
    ]
}

pub fn spawn_futhark_keyboard(mut commands: Commands, game_assets: Res<GameAssets>) {
    let rows = keyboard_rows_by_index();

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                bottom: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            Transform::from_xyz(-260.0, 0.0, 0.0),
            FutharkKeyboard,
        ))
        .with_children(|parent| {
            for (row_index, row) in rows.iter().enumerate() {
                parent
                    .spawn(Node {
                        margin: UiRect::left(Val::Px(KEYBOARD_ROW_OFFSETS[row_index])),
                        column_gap: Val::Px(8.0),
                        ..default()
                    })
                    .with_children(|row_parent| {
                        for &index in row {
                            row_parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(48.0),
                                        height: Val::Px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        position_type: PositionType::Relative,
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    FutharkKeyButton { index },
                                ))
                                .with_children(|key_parent| {
                                    key_parent.spawn((
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Px(48.0),
                                            height: Val::Px(48.0),
                                            ..default()
                                        },
                                        ImageNode::from_atlas_image(
                                            game_assets.futhark.clone(),
                                            TextureAtlas {
                                                layout: game_assets.futhark_layout.clone(),
                                                index: SPRITE_KEYBOARD_BG,
                                            },
                                        ),
                                        FutharkKeyBackground,
                                    ));

                                    key_parent.spawn((
                                        Node {
                                            width: Val::Px(32.0),
                                            height: Val::Px(32.0),
                                            ..default()
                                        },
                                        ImageNode::from_atlas_image(
                                            game_assets.futhark.clone(),
                                            TextureAtlas {
                                                layout: game_assets.futhark_layout.clone(),
                                                index: index + SPRITE_RUNE_OFFSET,
                                            },
                                        ),
                                        FutharkKeyRuneVisual,
                                    ));

                                    key_parent.spawn((
                                        Text::new(
                                            index_to_letter(index)
                                                .expect("valid futhark index")
                                                .to_string(),
                                        ),
                                        TextFont {
                                            font_size: 24.0,
                                            ..default()
                                        },
                                        TextColor(Color::BLACK),
                                        Node {
                                            display: Display::None,
                                            ..default()
                                        },
                                        FutharkKeyLabel { index },
                                        FutharkKeyLetterVisual,
                                    ));
                                });
                        }
                    });
            }
        });
}

pub fn emit_typed_futhark_input_from_keyboard(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut typed_futhark_input: MessageWriter<TypedFutharkInput>,
) {
    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        let Some(text) = &event.text else {
            continue;
        };

        for c in text.chars() {
            typed_futhark_input.write(TypedFutharkInput(c));
        }
    }
}

pub fn emit_typed_futhark_input_from_keyboard_clicks(
    keys: Query<(&Interaction, &FutharkKeyButton), (Changed<Interaction>, With<Button>)>,
    mut typed_futhark_input: MessageWriter<TypedFutharkInput>,
) {
    for (interaction, key) in &keys {
        if *interaction == Interaction::Pressed {
            if let Some(letter) = index_to_letter(key.index) {
                typed_futhark_input.write(TypedFutharkInput(letter));
            }
        }
    }
}

pub fn sync_futhark_key_hover(
    buttons: Query<(&Interaction, &Children), (Changed<Interaction>, With<FutharkKeyButton>)>,
    mut backgrounds: Query<&mut ImageNode, With<FutharkKeyBackground>>,
) {
    for (interaction, children) in &buttons {
        for child in children.iter() {
            if let Ok(mut image) = backgrounds.get_mut(child) {
                image.color = match *interaction {
                    Interaction::Hovered | Interaction::Pressed => Color::srgb(0.6, 0.7, 1.0),
                    Interaction::None => Color::WHITE,
                };
            }
        }
    }
}

pub fn animate_futhark_keyboard_colors(
    time: Res<Time>,
    speed: Res<FutharkKeyboardAnimationSpeed>,
    mut rune_images: Query<&mut ImageNode, With<FutharkKeyRuneVisual>>,
    mut letter_colors: Query<&mut TextColor, With<FutharkKeyLetterVisual>>,
) {
    let hue = (time.elapsed_secs() * speed.hue_degrees_per_second) % 360.0;
    let color = Color::hsl(hue, 1.0, 0.5);

    for mut image in &mut rune_images {
        image.color = color;
    }
    for mut text_color in &mut letter_colors {
        text_color.0 = color;
    }
}

pub fn toggle_futhark_keyboard_legend_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<FutharkKeyboardLegendMode>,
) {
    if keyboard.just_pressed(KeyCode::Tab) {
        *mode = match *mode {
            FutharkKeyboardLegendMode::Runes => FutharkKeyboardLegendMode::Letters,
            FutharkKeyboardLegendMode::Letters => FutharkKeyboardLegendMode::Runes,
        };
    }
}

pub fn sync_futhark_keyboard_labels(
    mode: Res<FutharkKeyboardLegendMode>,
    mut runes: Query<&mut Node, (With<FutharkKeyRuneVisual>, Without<FutharkKeyLetterVisual>)>,
    mut letters: Query<
        (&FutharkKeyLabel, &mut Text, &mut Node),
        (With<FutharkKeyLetterVisual>, Without<FutharkKeyRuneVisual>),
    >,
) {
    if !mode.is_changed() {
        return;
    }

    let (rune_display, letter_display) = match *mode {
        FutharkKeyboardLegendMode::Runes => (Display::Flex, Display::None),
        FutharkKeyboardLegendMode::Letters => (Display::None, Display::Flex),
    };

    for mut node in &mut runes {
        node.display = rune_display;
    }

    for (label, mut text, mut node) in &mut letters {
        if let Some(letter) = index_to_letter(label.index) {
            *text = Text::new(letter.to_string());
        }
        node.display = letter_display;
    }
}

pub fn last_typed_futhark_character(
    typed_futhark_input: &mut MessageReader<TypedFutharkInput>,
) -> Option<char> {
    let mut typed = None;

    for event in typed_futhark_input.read() {
        typed = Some(event.0);
    }

    typed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_letter_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(index_to_letter(index), Some(*letter));
        }
    }

    #[test]
    fn letter_to_index_maps_all_futhark_entries() {
        for (index, letter) in LETTERS.iter().enumerate() {
            assert_eq!(letter_to_index(*letter), Some(index));
        }
    }

    #[test]
    fn unknown_values_are_rejected() {
        assert_eq!(index_to_letter(24), None);
        assert_eq!(letter_to_index('x'), None);
    }

    #[test]
    fn keyboard_rows_match_expected_staggered_shape() {
        let rows = keyboard_rows_by_index();
        assert_eq!(rows[0].len(), 9);
        assert_eq!(rows[1].len(), 8);
        assert_eq!(rows[2].len(), 7);
    }

    #[test]
    fn rune_zero_is_fourth_key_in_middle_row() {
        let rows = keyboard_rows_by_index();
        assert_eq!(rows[1][3], 0);
    }

    #[test]
    fn keyboard_uses_all_futhark_entries_once() {
        let rows = keyboard_rows_by_index();
        let mut all_indices = rows
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect::<Vec<_>>();
        all_indices.sort_unstable();

        let expected_indices = (0..24).collect::<Vec<_>>();
        assert_eq!(all_indices, expected_indices);
    }
}
