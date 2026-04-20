use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::GameAssets;
use crate::futhark::{index_to_letter, letter_to_index};
use crate::ui::hud_root::LeftColumn;
use crate::ui::palette::{GOLD_DARK, GOLD_LIGHT, PARCHMENT_DARK};

#[allow(dead_code)]
const KEYBOARD_ROW_OFFSETS: [f32; 3] = [0.0, 96.0, 128.0];
/// Per-row left padding as a percentage of the keyboard panel width.
/// Derived from the legacy pixel offsets above against a ~640 px reference panel
/// (top row: tab key + 10 keys + gaps = 80 + 8 + 10*48 + 9*8).
const KEYBOARD_ROW_PERCENT_OFFSETS: [f32; 3] = [0.0, 15.0, 20.0];
const KEYBOARD_TOP_ROW: [usize; 10] = [12, 7, 18, 4, 16, 2, 1, 10, 23, 13];
const KEYBOARD_MIDDLE_ROW: [usize; 9] = [3, 15, 22, 0, 6, 8, 11, 5, 20];
const KEYBOARD_BOTTOM_ROW: [usize; 7] = [14, 24, 21, usize::MAX, 17, 9, 19];

const SPRITE_KEYBOARD_BG: usize = 254;
const SPRITE_TAB_ACTION: usize = 252;
const SPRITE_BACKSPACE_ACTION: usize = 251;
pub const SPRITE_RUNE_OFFSET: usize = 32;

#[derive(Component)]
pub struct FutharkKeyboard;

#[derive(Component)]
pub struct KeyboardPanel;

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

    pub fn len(&self) -> usize {
        self.letters.len()
    }

    pub fn snapshot(&self) -> HashSet<char> {
        self.letters.clone()
    }

    pub fn restore(&mut self, letters: &HashSet<char>) {
        for &ch in letters {
            self.letters.insert(ch);
        }
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

pub fn keyboard_rows_by_index() -> [Vec<usize>; 3] {
    [
        KEYBOARD_TOP_ROW.to_vec(),
        KEYBOARD_MIDDLE_ROW.to_vec(),
        KEYBOARD_BOTTOM_ROW.to_vec(),
    ]
}

#[derive(Clone, Copy)]
struct KeyMetrics {
    key_width: Val,
    key_height: Val,
    key_aspect_ratio: Option<f32>,
    wide_key_width: Val,
    wide_key_height: Val,
    wide_key_aspect_ratio: Option<f32>,
    column_gap: Val,
    row_gap: Val,
    rune_size: Val,
    rune_aspect_ratio: Option<f32>,
    letter_font_size: f32,
    panel_padding: UiRect,
    panel_row_gap: Val,
    header_margin_bottom: Val,
    header_title_font_size: f32,
    header_aside_font_size: f32,
}

const PARENTED_METRICS: KeyMetrics = KeyMetrics {
    key_width: Val::Percent(9.0),
    key_height: Val::Auto,
    key_aspect_ratio: Some(1.0),
    wide_key_width: Val::Percent(13.3),
    wide_key_height: Val::Auto,
    wide_key_aspect_ratio: Some(80.0 / 48.0),
    column_gap: Val::Percent(0.6),
    row_gap: Val::Percent(1.4),
    rune_size: Val::Percent(140.0),
    rune_aspect_ratio: Some(1.0),
    letter_font_size: 16.0,
    panel_padding: UiRect::all(Val::Percent(2.0)),
    panel_row_gap: Val::Percent(1.0),
    header_margin_bottom: Val::Percent(2.0),
    header_title_font_size: 14.0,
    header_aside_font_size: 10.0,
};

const ABSOLUTE_METRICS: KeyMetrics = KeyMetrics {
    key_width: Val::Px(48.0),
    key_height: Val::Px(48.0),
    key_aspect_ratio: None,
    wide_key_width: Val::Px(80.0),
    wide_key_height: Val::Px(48.0),
    wide_key_aspect_ratio: None,
    column_gap: Val::Px(8.0),
    row_gap: Val::Px(10.0),
    rune_size: Val::Px(32.0),
    rune_aspect_ratio: None,
    letter_font_size: 24.0,
    panel_padding: UiRect::all(Val::Px(10.0)),
    panel_row_gap: Val::Px(8.0),
    header_margin_bottom: Val::Px(6.0),
    header_title_font_size: 16.0,
    header_aside_font_size: 12.0,
};

pub fn spawn_futhark_keyboard(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    left_column: Query<Entity, With<LeftColumn>>,
) {
    let rows = keyboard_rows_by_index();
    let host = left_column.single().ok();

    let metrics = if host.is_some() {
        PARENTED_METRICS
    } else {
        ABSOLUTE_METRICS
    };

    let panel_node = if host.is_some() {
        Node {
            width: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: metrics.panel_row_gap,
            border: UiRect::all(Val::Px(1.0)),
            padding: metrics.panel_padding,
            ..default()
        }
    } else {
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(24.0),
            bottom: Val::Px(24.0),
            flex_direction: FlexDirection::Column,
            row_gap: metrics.panel_row_gap,
            border: UiRect::all(Val::Px(1.0)),
            padding: metrics.panel_padding,
            ..default()
        }
    };

    let mut panel = commands.spawn((
        KeyboardPanel,
        panel_node,
        BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.85)),
        BorderColor {
            top: GOLD_DARK,
            right: GOLD_DARK,
            bottom: GOLD_DARK,
            left: GOLD_DARK,
        },
    ));

    if let Some(parent) = host {
        panel.insert(ChildOf(parent));
    }

    let header_title_font = game_assets.font_cormorant_unicase_semibold.clone();
    let header_aside_font = game_assets.font_im_fell_sc.clone();

    panel.with_children(|panel| {
        panel
            .spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Baseline,
                margin: UiRect::bottom(metrics.header_margin_bottom),
                ..default()
            })
            .with_children(|header| {
                header.spawn((
                    Text::new("Rune Keyboard"),
                    TextFont {
                        font: header_title_font,
                        font_size: metrics.header_title_font_size,
                        ..default()
                    },
                    TextColor(GOLD_LIGHT),
                ));
            });

        panel
            .spawn((
                FutharkKeyboard,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: metrics.row_gap,
                    ..default()
                },
            ))
            .with_children(|parent| {
                for (row_index, row) in rows.iter().enumerate() {
                    parent
                        .spawn(Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::left(Val::Percent(
                                KEYBOARD_ROW_PERCENT_OFFSETS[row_index],
                            )),
                            column_gap: metrics.column_gap,
                            ..default()
                        })
                        .with_children(|row_parent| {
                            for &index in row {
                                if index == usize::MAX {
                                    row_parent.spawn(Node {
                                        width: metrics.key_width,
                                        height: metrics.key_height,
                                        aspect_ratio: metrics.key_aspect_ratio,
                                        ..default()
                                    });
                                    continue;
                                }

                                spawn_letter_key(row_parent, &game_assets, &metrics, index);
                            }

                            if row_index == 2 {
                                spawn_action_key(
                                    row_parent,
                                    &game_assets,
                                    &metrics,
                                    SPRITE_BACKSPACE_ACTION,
                                    FutharkKeyboardCommandType::Backspace,
                                    false,
                                );
                            }
                        });
                }
            });
    });
}

fn spawn_action_key(
    row_parent: &mut ChildSpawnerCommands,
    game_assets: &GameAssets,
    metrics: &KeyMetrics,
    sprite_index: usize,
    command: FutharkKeyboardCommandType,
    tab_visual: bool,
) {
    row_parent
        .spawn((
            Button,
            Node {
                width: metrics.wide_key_width,
                height: metrics.wide_key_height,
                aspect_ratio: metrics.wide_key_aspect_ratio,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(Color::NONE),
            FutharkKeyboardButton,
            FutharkActionButton { command },
        ))
        .with_children(|key_parent| {
            key_parent.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
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

            let mut icon = key_parent.spawn((
                Node {
                    width: metrics.rune_size,
                    height: Val::Auto,
                    aspect_ratio: metrics.rune_aspect_ratio,
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: sprite_index,
                    },
                ),
            ));
            if tab_visual {
                icon.insert(FutharkTabActionVisual);
            }
        });
}

fn spawn_letter_key(
    row_parent: &mut ChildSpawnerCommands,
    game_assets: &GameAssets,
    metrics: &KeyMetrics,
    index: usize,
) {
    let letter = index_to_letter(index).expect("valid futhark index");

    row_parent
        .spawn((
            Button,
            Node {
                width: metrics.key_width,
                height: metrics.key_height,
                aspect_ratio: metrics.key_aspect_ratio,
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
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
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
                    width: metrics.rune_size,
                    height: Val::Auto,
                    aspect_ratio: metrics.rune_aspect_ratio,
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
                    font_size: metrics.letter_font_size,
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
