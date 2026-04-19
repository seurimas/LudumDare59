use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use rand::Rng;
use std::collections::{HashMap, HashSet};

use crate::GameAssets;

pub const LETTERS: [char; 25] = [
    'f', // 0
    'u', // 1
    'T', // 2
    'a', // 3
    'r', // 4
    'k', // 5
    'g', // 6
    'w', // 7
    'h', // 8
    'n', // 9
    'i', // 10
    'j', // 11
    'A', // 12
    'p', // 13
    'z', // 14
    's', // 15
    't', // 16
    'b', // 17
    'e', // 18
    'm', // 19
    'l', // 20
    'N', // 21
    'd', // 22
    'o', // 23
    'S', // 24
];

const KEYBOARD_ROW_OFFSETS: [f32; 3] = [0.0, 96.0, 128.0];
// Each number shows up once, in a similar place to a QWERTY keyboard.
const KEYBOARD_TOP_ROW: [usize; 10] = [12, 7, 18, 4, 16, 2, 1, 10, 23, 13];
const KEYBOARD_MIDDLE_ROW: [usize; 9] = [3, 15, 22, 0, 6, 8, 11, 5, 20];
const KEYBOARD_BOTTOM_ROW: [usize; 7] = [14, 24, 21, usize::MAX, 17, 9, 19];

#[derive(Component)]
pub struct FutharkKeyboard;

#[derive(Component)]
pub struct FutharkKeyboardButton;

#[derive(Component)]
pub struct FutharkKeyButton {
    pub index: usize,
}

#[derive(Component)]
pub struct FutharkKeyFade {
    pub alpha: f32,
}

impl Default for FutharkKeyFade {
    fn default() -> Self {
        Self { alpha: 1.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FutharkKeyboardCommandType {
    ToggleLegendMode,
    Backspace,
}

#[derive(Component)]
pub struct FutharkActionButton {
    pub command: FutharkKeyboardCommandType,
}

#[derive(Component)]
pub struct FutharkKeyLabel {
    pub index: usize,
}

#[derive(Component)]
pub struct FutharkKeyBackground {
    pub base_color: Color,
}

#[derive(Component)]
pub struct FutharkKeyRuneVisual;

#[derive(Component)]
pub struct FutharkKeyLetterVisual;

#[derive(Component)]
pub struct FutharkTabActionVisual;

const SPRITE_KEYBOARD_BG: usize = 254;
const SPRITE_TAB_ACTION: usize = 252;
const SPRITE_BACKSPACE_ACTION: usize = 251;
pub const SPRITE_RUNE_OFFSET: usize = 32;

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
            hue_degrees_per_second: 30.0,
        }
    }
}

#[derive(Message)]
pub struct TypedFutharkInput(pub char);

#[derive(Message, Default)]
pub struct EliminatedKeyPressed;

#[derive(Message, Clone, Copy)]
pub struct FutharkKeyboardCommand(pub FutharkKeyboardCommandType);

#[derive(Resource, Clone)]
pub struct FutharkKeyboardAliases {
    alias_to_rune: HashMap<char, char>,
}

#[derive(Resource, Default)]
pub struct EliminatedFutharkKeys {
    letters: HashSet<char>,
}

impl EliminatedFutharkKeys {
    pub fn clear(&mut self) {
        self.letters.clear();
    }

    pub fn contains(&self, letter: char) -> bool {
        self.letters.contains(&letter)
    }

    pub fn insert(&mut self, letter: char) {
        self.letters.insert(letter);
    }
}

impl Default for FutharkKeyboardAliases {
    fn default() -> Self {
        let mut alias_to_rune = HashMap::new();

        alias_to_rune.insert('q', 'A');
        alias_to_rune.insert('Q', 'A');
        alias_to_rune.insert('y', 'T');
        alias_to_rune.insert('Y', 'T');
        alias_to_rune.insert('x', 'S');
        alias_to_rune.insert('X', 'S');
        alias_to_rune.insert('c', 'N');
        alias_to_rune.insert('C', 'N');

        Self { alias_to_rune }
    }
}

impl FutharkKeyboardAliases {
    pub fn map_alias(&self, key: char) -> Option<char> {
        self.alias_to_rune.get(&key).copied()
    }

    pub fn set_alias(&mut self, alias: char, rune: char) {
        self.alias_to_rune.insert(alias, rune);
    }
}

#[derive(Resource, Default)]
pub struct PrebakedFutharkAudio {
    pub handles_by_index: Vec<Vec<Handle<AudioSource>>>,
}

#[derive(Resource, Default)]
pub struct PrebakedFutharkConversationalAudio {
    pub handles_by_index: Vec<Vec<Handle<AudioSource>>>,
}

/// Raw f32 sample buffers for every baked letter variant.
/// Used mid-game when multiple letters need to be concatenated before playback.
#[derive(Resource, Default)]
pub struct BakedAudioSamples {
    pub regular: Vec<Vec<crate::audio::ProcessedAudio>>,
    pub conversational: Vec<Vec<crate::audio::ProcessedAudio>>,
}

pub fn configure_futhark_keyboard(app: &mut App) {
    app.init_resource::<FutharkKeyboardLegendMode>();
    app.init_resource::<FutharkKeyboardAnimationSpeed>();
    app.init_resource::<FutharkKeyboardAliases>();
    app.init_resource::<EliminatedFutharkKeys>();
    app.add_message::<TypedFutharkInput>();
    app.add_message::<EliminatedKeyPressed>();
    app.add_message::<FutharkKeyboardCommand>();
}

fn is_eliminated_for_binding(
    letter: char,
    eliminated_keys: &EliminatedFutharkKeys,
    battle_state: Option<&crate::rune_words::battle::BattleState>,
) -> bool {
    battle_state
        .map(|state| state.phase == crate::rune_words::battle::BattlePhase::Binding)
        .unwrap_or(false)
        && eliminated_keys.contains(letter)
}

fn map_typed_char_to_futhark(letter: char, aliases: &FutharkKeyboardAliases) -> Option<char> {
    if letter_to_index(letter).is_some() {
        return Some(letter);
    }

    aliases
        .map_alias(letter)
        .filter(|mapped| letter_to_index(*mapped).is_some())
}

fn is_vowel(letter: char) -> bool {
    matches!(letter.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
}

fn key_background_color(letter: char) -> Color {
    let is_uppercase = letter.is_ascii_uppercase();

    match (is_vowel(letter), is_uppercase) {
        (true, true) => Color::srgb(0.55, 0.82, 0.55),
        (true, false) => Color::srgb(0.96, 0.92, 0.50),
        (false, true) => Color::srgb(0.55, 0.72, 0.96),
        (false, false) => Color::WHITE,
    }
}

fn keyboard_label_for_letter(letter: char) -> (String, f32) {
    match letter {
        'A' => ("ah".to_owned(), 16.0),
        'S' => ("sh".to_owned(), 16.0),
        'T' => ("th".to_owned(), 16.0),
        'N' => ("ng".to_owned(), 16.0),
        _ => (letter.to_string(), 24.0),
    }
}

/// Bake all parameter variants for one futhark letter.
///
/// Returns:
/// - `Vec<Handle<AudioSource>>` — WAV-encoded handles ready for `AudioPlayer`.
/// - `Vec<ProcessedAudio>`     — raw f32 samples kept for mid-game concatenation.
pub fn bake_futhark_letter(
    letter_index: usize,
    game_assets: &GameAssets,
    config: Option<&crate::audio::FutharkSoundConfig>,
    audio_assets: &mut Assets<AudioSource>,
) -> (Vec<Handle<AudioSource>>, Vec<crate::audio::ProcessedAudio>) {
    let Some(raw) = game_assets.futhark_sounds.get(letter_index) else {
        panic!("invalid futhark letter index");
    };
    let Some(source) = audio_assets.get(&raw.clone().typed::<AudioSource>()) else {
        return (Vec::new(), Vec::new());
    };

    let processed_list: Vec<crate::audio::ProcessedAudio> =
        params_to_bake_for_index(config, letter_index)
            .into_iter()
            .map(|params| crate::audio::process_audio(&source.bytes, &params))
            .collect();

    let handles: Vec<Handle<AudioSource>> = processed_list
        .iter()
        .map(|p| {
            let wav = crate::audio::samples_to_wav(&p.samples, p.channels, p.sample_rate);
            audio_assets.add(AudioSource { bytes: wav.into() })
        })
        .collect();

    (handles, processed_list)
}

fn params_to_bake_for_index(
    config: Option<&crate::audio::FutharkSoundConfig>,
    index: usize,
) -> Vec<crate::audio::SoundParams> {
    let variants = config
        .and_then(|c| c.0.get(index))
        .filter(|v| !v.is_empty());

    let base_variants: Vec<crate::audio::SoundParams> = match variants {
        None => vec![crate::audio::SoundParams::default()],
        Some(v) => v.clone(),
    };

    base_variants
}

pub fn play_futhark_key_sound(
    mut typed_futhark_input: MessageReader<crate::rune_words::rune_slots::TypedInputDuringGrading>,
    prebaked_audio: Option<Res<PrebakedFutharkAudio>>,
    mut commands: Commands,
) {
    let Some(prebaked_audio) = prebaked_audio else {
        return;
    };

    for event in typed_futhark_input.read() {
        let Some(index) = letter_to_index(event.0) else {
            continue;
        };
        let Some(handles) = prebaked_audio
            .handles_by_index
            .get(index)
            .filter(|h| !h.is_empty())
        else {
            continue;
        };

        let handle = if handles.len() == 1 {
            handles[0].clone()
        } else {
            let i = rand::thread_rng().gen_range(0..handles.len());
            handles[i].clone()
        };
        commands.spawn((
            AudioPlayer::<AudioSource>(handle),
            PlaybackSettings::DESPAWN,
        ));
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
                left: Val::Px(24.0),
                bottom: Val::Px(24.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
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
                        if row_index == 0 {
                            row_parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        position_type: PositionType::Relative,
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    FutharkKeyboardButton,
                                    FutharkActionButton {
                                        command: FutharkKeyboardCommandType::ToggleLegendMode,
                                    },
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
                                        FutharkKeyBackground {
                                            base_color: Color::WHITE,
                                        },
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
                                                index: SPRITE_TAB_ACTION,
                                            },
                                        ),
                                        FutharkTabActionVisual,
                                    ));
                                });
                        }

                        for &index in row {
                            if index == usize::MAX {
                                row_parent.spawn(Node {
                                    width: Val::Px(48.0),
                                    height: Val::Px(48.0),
                                    ..default()
                                });
                                continue;
                            }

                            let letter = index_to_letter(index).expect("valid futhark index");

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
                                    FutharkKeyboardButton,
                                    FutharkKeyButton { index },
                                    FutharkKeyFade::default(),
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
                                        FutharkKeyBackground {
                                            base_color: key_background_color(letter),
                                        },
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
                                        Text::new(letter.to_string()),
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

                        if row_index == 2 {
                            row_parent
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(48.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        position_type: PositionType::Relative,
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    FutharkKeyboardButton,
                                    FutharkActionButton {
                                        command: FutharkKeyboardCommandType::Backspace,
                                    },
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
                                        FutharkKeyBackground {
                                            base_color: Color::WHITE,
                                        },
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
                                                index: SPRITE_BACKSPACE_ACTION,
                                            },
                                        ),
                                    ));
                                });
                        }
                    });
            }
        });
}

pub fn emit_futhark_keyboard_command_from_clicks(
    buttons: Query<
        (&Interaction, &FutharkActionButton),
        (
            Changed<Interaction>,
            With<Button>,
            With<FutharkKeyboardButton>,
        ),
    >,
    mut commands: MessageWriter<FutharkKeyboardCommand>,
) {
    for (interaction, action) in &buttons {
        if *interaction == Interaction::Pressed {
            commands.write(FutharkKeyboardCommand(action.command));
        }
    }
}

pub fn emit_typed_futhark_input_from_keyboard(
    mut keyboard_input: MessageReader<KeyboardInput>,
    aliases: Res<FutharkKeyboardAliases>,
    eliminated_keys: Res<EliminatedFutharkKeys>,
    battle_state: Option<Res<crate::rune_words::battle::BattleState>>,
    mut typed_futhark_input: MessageWriter<TypedFutharkInput>,
    mut eliminated_key_pressed: MessageWriter<EliminatedKeyPressed>,
) {
    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        let Some(text) = &event.text else {
            continue;
        };

        for c in text.chars() {
            if let Some(mapped) = map_typed_char_to_futhark(c, &aliases) {
                if is_eliminated_for_binding(mapped, &eliminated_keys, battle_state.as_deref()) {
                    eliminated_key_pressed.write_default();
                } else {
                    typed_futhark_input.write(TypedFutharkInput(mapped));
                }
            }
        }
    }
}

pub fn emit_typed_futhark_input_from_keyboard_clicks(
    keys: Query<(&Interaction, &FutharkKeyButton), (Changed<Interaction>, With<Button>)>,
    eliminated_keys: Res<EliminatedFutharkKeys>,
    battle_state: Option<Res<crate::rune_words::battle::BattleState>>,
    mut typed_futhark_input: MessageWriter<TypedFutharkInput>,
    mut eliminated_key_pressed: MessageWriter<EliminatedKeyPressed>,
) {
    for (interaction, key) in &keys {
        if *interaction == Interaction::Pressed {
            if let Some(letter) = index_to_letter(key.index) {
                if is_eliminated_for_binding(letter, &eliminated_keys, battle_state.as_deref()) {
                    eliminated_key_pressed.write_default();
                } else {
                    typed_futhark_input.write(TypedFutharkInput(letter));
                }
            }
        }
    }
}

pub fn sync_eliminated_futhark_keys(
    time: Res<Time>,
    battle_state: Option<Res<crate::rune_words::battle::BattleState>>,
    eliminated_keys: Res<EliminatedFutharkKeys>,
    mut keys: Query<(&FutharkKeyButton, &mut FutharkKeyFade, &Children), With<Button>>,
    mut images: Query<&mut ImageNode>,
    mut text_colors: Query<&mut TextColor>,
) {
    let fade_step = (time.delta_secs() / 0.2).clamp(0.0, 1.0);

    for (key_button, mut fade, children) in &mut keys {
        let Some(letter) = index_to_letter(key_button.index) else {
            continue;
        };
        let should_eliminate =
            is_eliminated_for_binding(letter, &eliminated_keys, battle_state.as_deref());
        let target_alpha = if should_eliminate { 0.0 } else { 1.0 };

        if fade.alpha < target_alpha {
            fade.alpha = (fade.alpha + fade_step).min(target_alpha);
        } else if fade.alpha > target_alpha {
            fade.alpha = (fade.alpha - fade_step).max(target_alpha);
        }

        for child in children.iter() {
            if let Ok(mut image) = images.get_mut(child) {
                let srgb = image.color.to_srgba();
                image.color = Color::srgba(srgb.red, srgb.green, srgb.blue, fade.alpha);
            }

            if let Ok(mut text_color) = text_colors.get_mut(child) {
                let srgb = text_color.0.to_srgba();
                text_color.0 = Color::srgba(srgb.red, srgb.green, srgb.blue, fade.alpha);
            }
        }
    }
}

pub fn sync_futhark_key_hover(
    buttons: Query<(&Interaction, &Children), (Changed<Interaction>, With<FutharkKeyButton>)>,
    mut backgrounds: Query<(&mut ImageNode, &FutharkKeyBackground)>,
) {
    for (interaction, children) in &buttons {
        for child in children.iter() {
            if let Ok((mut image, background)) = backgrounds.get_mut(child) {
                image.color = match *interaction {
                    Interaction::Hovered | Interaction::Pressed => Color::srgb(0.6, 0.7, 1.0),
                    Interaction::None => background.base_color,
                };
            }
        }
    }
}

pub fn animate_futhark_keyboard_colors(
    time: Res<Time>,
    speed: Res<FutharkKeyboardAnimationSpeed>,
    mut rune_images: Query<&mut ImageNode, With<FutharkKeyRuneVisual>>,
) {
    let hue = (time.elapsed_secs() * speed.hue_degrees_per_second) % 360.0;
    let color = Color::hsl(hue, 1.0, 0.5);

    for mut image in &mut rune_images {
        image.color = color;
    }
}

pub fn toggle_futhark_keyboard_legend_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: MessageReader<FutharkKeyboardCommand>,
    mut mode: ResMut<FutharkKeyboardLegendMode>,
) {
    let tab_pressed = keyboard.just_pressed(KeyCode::Tab)
        || commands
            .read()
            .any(|command| command.0 == FutharkKeyboardCommandType::ToggleLegendMode);

    if tab_pressed {
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
        (&FutharkKeyLabel, &mut Text, &mut TextFont, &mut Node),
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

    for (label, mut text, mut text_font, mut node) in &mut letters {
        if let Some(letter) = index_to_letter(label.index) {
            let (legend, font_size) = keyboard_label_for_letter(letter);
            *text = Text::new(legend);
            text_font.font_size = font_size;
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
    use crate::rune_words::battle::{BattlePhase, BattleState};

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
        assert_eq!(index_to_letter(25), None);
        assert_eq!(letter_to_index('x'), None);
    }

    #[test]
    fn keyboard_uses_all_futhark_entries_once() {
        let rows = keyboard_rows_by_index();
        let mut all_indices = rows
            .iter()
            .flat_map(|row| row.iter().copied())
            .filter(|&i| i != usize::MAX)
            .collect::<Vec<_>>();
        all_indices.sort_unstable();

        let expected_indices = (0..25).collect::<Vec<_>>();
        assert_eq!(all_indices, expected_indices);
    }

    #[test]
    fn highlights_vowels_in_yellow() {
        assert_eq!(key_background_color('a'), Color::srgb(0.96, 0.92, 0.50));
        assert_eq!(key_background_color('u'), Color::srgb(0.96, 0.92, 0.50));
    }

    #[test]
    fn highlights_uppercase_in_blue() {
        assert_eq!(key_background_color('T'), Color::srgb(0.55, 0.72, 0.96));
        assert_eq!(key_background_color('S'), Color::srgb(0.55, 0.72, 0.96));
    }

    #[test]
    fn highlights_uppercase_vowels_in_green() {
        assert_eq!(key_background_color('A'), Color::srgb(0.55, 0.82, 0.55));
    }

    #[test]
    fn positional_aliases_map_to_uppercase_runes() {
        let aliases = FutharkKeyboardAliases::default();

        assert_eq!(map_typed_char_to_futhark('q', &aliases), Some('A'));
        assert_eq!(map_typed_char_to_futhark('Q', &aliases), Some('A'));
        assert_eq!(map_typed_char_to_futhark('y', &aliases), Some('T'));
        assert_eq!(map_typed_char_to_futhark('x', &aliases), Some('S'));
        assert_eq!(map_typed_char_to_futhark('c', &aliases), Some('N'));
    }

    #[test]
    fn direct_rune_letter_input_still_works() {
        let aliases = FutharkKeyboardAliases::default();

        assert_eq!(map_typed_char_to_futhark('a', &aliases), Some('a'));
        assert_eq!(map_typed_char_to_futhark('A', &aliases), Some('A'));
        assert_eq!(map_typed_char_to_futhark('z', &aliases), Some('z'));
    }

    #[test]
    fn keyboard_labels_use_phonetics_for_uppercase_letters() {
        assert_eq!(keyboard_label_for_letter('A'), ("ah".to_owned(), 16.0));
        assert_eq!(keyboard_label_for_letter('S'), ("sh".to_owned(), 16.0));
        assert_eq!(keyboard_label_for_letter('T'), ("th".to_owned(), 16.0));
        assert_eq!(keyboard_label_for_letter('N'), ("ng".to_owned(), 16.0));
        assert_eq!(keyboard_label_for_letter('r'), ("r".to_owned(), 24.0));
    }

    #[test]
    fn eliminated_keys_only_block_during_binding() {
        let mut eliminated = EliminatedFutharkKeys::default();
        eliminated.insert('r');

        let mut battle_state = BattleState::default();
        battle_state.phase = BattlePhase::Binding;
        assert!(is_eliminated_for_binding(
            'r',
            &eliminated,
            Some(&battle_state)
        ));

        battle_state.phase = BattlePhase::Acting;
        assert!(!is_eliminated_for_binding(
            'r',
            &eliminated,
            Some(&battle_state)
        ));
    }

    #[test]
    fn non_eliminated_keys_are_never_blocked() {
        let eliminated = EliminatedFutharkKeys::default();
        let mut battle_state = BattleState::default();
        battle_state.phase = BattlePhase::Binding;

        assert!(!is_eliminated_for_binding(
            'r',
            &eliminated,
            Some(&battle_state)
        ));
    }
}
