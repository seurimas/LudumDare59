use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use bevy_aspect_ratio_mask::Hud;
use rand::seq::SliceRandom;

use crate::GameAssets;
use crate::GameState;
use crate::futhark::{SPRITE_RUNE_OFFSET, letter_to_index};
use crate::rune_words::battle_states::binding::BindingSucceeded;
use crate::spellbook::{Book, LearnedSpells, SpellDef, SpellEffect};
use crate::tutorial::TutorialState;
use crate::ui::palette::*;

const UNLEARN_THRESHOLD: usize = 8;
const RELEARN_THRESHOLD: usize = 12;

/// The action a candidate card represents when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    Learn,
    Unlearn,
    Relearn,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub word: String,
    pub mode: SelectionMode,
}

/// Tracks the current spell-selection modal state. `candidates` is `Some` while
/// the modal is open and pauses the NPC spawn timer.
#[derive(Resource, Default)]
pub struct SpellSelection {
    pub candidates: Option<Vec<Candidate>>,
}

impl SpellSelection {
    pub fn is_open(&self) -> bool {
        self.candidates.is_some()
    }

    pub fn close(&mut self) {
        self.candidates = None;
    }
}

#[derive(Component)]
struct SpellSelectionModal;

#[derive(Component)]
struct SpellSelectionChoice {
    word: String,
    mode: SelectionMode,
}

#[derive(Component)]
struct SpellSelectionSkip;

pub fn configure_spell_selection(app: &mut App) {
    app.init_resource::<SpellSelection>();
    app.add_systems(
        Update,
        (
            open_selection_on_binding_success,
            spawn_selection_modal_when_open,
            handle_selection_click,
            despawn_selection_modal_when_closed,
        )
            .chain()
            .run_if(in_state(GameState::Adventure)),
    );
}

/// When a binding succeeds outside the tutorial, build a set of candidates
/// (up to 2 un-learned Learn options, plus Unlearn/Relearn once thresholds
/// are reached) and open the selection modal.
fn open_selection_on_binding_success(
    mut events: MessageReader<BindingSucceeded>,
    tutorial: Option<Res<TutorialState>>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
    learned: Res<LearnedSpells>,
    mut selection: ResMut<SpellSelection>,
) {
    if events.read().count() == 0 {
        return;
    }
    if tutorial.as_ref().is_some_and(|t| t.active) {
        return;
    }
    if selection.is_open() {
        return;
    }
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };

    let mut rng = rand::thread_rng();
    let mut candidates: Vec<Candidate> = Vec::new();

    let unlearned: Vec<String> = book
        .spells()
        .iter()
        .filter(|s| !learned.contains(&s.word))
        .map(|s| s.word.clone())
        .collect();
    let mut learn_pool = unlearned;
    learn_pool.shuffle(&mut rng);
    learn_pool.truncate(2);
    for word in learn_pool {
        candidates.push(Candidate {
            word,
            mode: SelectionMode::Learn,
        });
    }

    let unique_learned = learned.unique_words();
    if learned.words.len() >= UNLEARN_THRESHOLD
        && let Some(word) = unique_learned.choose(&mut rng).cloned()
    {
        candidates.push(Candidate {
            word,
            mode: SelectionMode::Unlearn,
        });
    }
    if learned.words.len() >= RELEARN_THRESHOLD
        && let Some(word) = unique_learned.choose(&mut rng).cloned()
    {
        candidates.push(Candidate {
            word,
            mode: SelectionMode::Relearn,
        });
    }

    let has_any_spell = candidates
        .iter()
        .any(|c| book.spells().iter().any(|s| s.word == c.word));
    if !has_any_spell {
        return;
    }

    selection.candidates = Some(candidates);
}

fn spawn_selection_modal_when_open(
    mut commands: Commands,
    selection: Res<SpellSelection>,
    existing: Query<Entity, With<SpellSelectionModal>>,
    hud: Res<Hud>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
) {
    if !selection.is_open() || !existing.is_empty() {
        return;
    }
    let Some(candidates) = selection.candidates.as_ref() else {
        return;
    };
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };

    let resolved: Vec<(Candidate, &SpellDef)> = candidates
        .iter()
        .filter_map(|c| {
            book.spells()
                .iter()
                .find(|s| s.word == c.word)
                .map(|s| (c.clone(), s))
        })
        .collect();

    let font_heading = game_assets.font_cormorant_unicase_semibold.clone();
    let font_aside = game_assets.font_im_fell_sc.clone();
    let font_word = game_assets.font_cormorant_unicase_bold.clone();
    let font_dropcap = game_assets.font_unifraktur.clone();

    commands.entity(hud.0).with_children(|hud_root| {
        hud_root
            .spawn((
                SpellSelectionModal,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
                ZIndex(100),
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(16.0),
                        padding: UiRect::all(Val::Px(24.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        max_width: Val::Px(960.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.96)),
                    BorderColor {
                        top: GOLD_DARK,
                        right: GOLD_DARK,
                        bottom: GOLD_DARK,
                        left: GOLD_DARK,
                    },
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Inscribe a new spell"),
                        TextFont {
                            font: font_heading.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(GOLD_LIGHT),
                    ));
                    panel.spawn((
                        Text::new("choose one to add to thy book"),
                        TextFont {
                            font: font_aside.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(PARCHMENT_DARK),
                    ));

                    panel
                        .spawn(Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(20.0),
                            ..default()
                        })
                        .with_children(|row| {
                            for (candidate, spell) in &resolved {
                                spawn_choice_card(
                                    row,
                                    spell,
                                    candidate.mode,
                                    &game_assets,
                                    font_heading.clone(),
                                    font_word.clone(),
                                    font_dropcap.clone(),
                                    font_aside.clone(),
                                );
                            }
                        });

                    panel
                        .spawn((
                            Button,
                            SpellSelectionSkip,
                            Node {
                                margin: UiRect::top(Val::Px(8.0)),
                                padding: UiRect::axes(Val::Px(24.0), Val::Px(10.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.12, 0.10, 0.06, 0.9)),
                            BorderColor {
                                top: GOLD_DARK,
                                right: GOLD_DARK,
                                bottom: GOLD_DARK,
                                left: GOLD_DARK,
                            },
                        ))
                        .with_children(|btn| {
                            btn.spawn((
                                Text::new("Skip"),
                                TextFont {
                                    font: font_aside.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(PARCHMENT_WARM),
                            ));
                        });
                });
            });
    });
}

fn spawn_choice_card(
    parent: &mut ChildSpawnerCommands,
    spell: &SpellDef,
    mode: SelectionMode,
    game_assets: &GameAssets,
    font_heading: Handle<Font>,
    font_word: Handle<Font>,
    font_dropcap: Handle<Font>,
    font_num: Handle<Font>,
) {
    let (mode_label, mode_color, card_bg) = match mode {
        SelectionMode::Learn => ("Learn", GOLD_LIGHT, PARCHMENT_WARM),
        SelectionMode::Unlearn => ("Unlearn", BLOOD, Color::srgba(0.28, 0.10, 0.08, 0.95)),
        SelectionMode::Relearn => (
            "Relearn (+1)",
            GOLD_LIGHT,
            Color::srgba(0.10, 0.14, 0.22, 0.95),
        ),
    };
    let (heading_color, body_color) = match mode {
        SelectionMode::Learn => (INK, INK),
        _ => (PARCHMENT_WARM, PARCHMENT_WARM),
    };
    parent
        .spawn((
            Button,
            SpellSelectionChoice {
                word: spell.word.clone(),
                mode,
            },
            Node {
                width: Val::Px(240.0),
                min_height: Val::Px(220.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(card_bg),
            BorderColor {
                top: GOLD_DARK,
                right: GOLD_DARK,
                bottom: GOLD_DARK,
                left: GOLD_DARK,
            },
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(mode_label),
                TextFont {
                    font: font_heading.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(mode_color),
            ));

            // Dropcap (first letter, uppercase)
            let first_char = spell
                .word
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default();
            card.spawn((
                Text::new(first_char),
                TextFont {
                    font: font_dropcap,
                    font_size: 44.0,
                    ..default()
                },
                TextColor(BLOOD),
            ));

            // Word name
            card.spawn((
                Text::new(spell.word.to_uppercase()),
                TextFont {
                    font: font_word,
                    font_size: 20.0,
                    ..default()
                },
                TextColor(heading_color),
            ));

            // Rune glyph row
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(2.0),
                ..default()
            })
            .with_children(|row| {
                for letter in spell.futharkation.chars() {
                    let atlas_index = letter_to_index(letter)
                        .map(|i| i + SPRITE_RUNE_OFFSET)
                        .unwrap_or(SPRITE_RUNE_OFFSET);
                    row.spawn((
                        Node {
                            width: Val::Px(22.0),
                            height: Val::Px(22.0),
                            ..default()
                        },
                        ImageNode::from_atlas_image(
                            game_assets.futhark.clone(),
                            TextureAtlas {
                                layout: game_assets.futhark_layout.clone(),
                                index: atlas_index,
                            },
                        ),
                    ));
                }
            });

            // Effects row
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            })
            .with_children(|row| {
                for effect in &spell.effects {
                    let icon_index = effect_sprite_index(effect);
                    let labels = effect_labels(effect);
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|col| {
                        col.spawn((
                            Node {
                                width: Val::Px(20.0),
                                height: Val::Px(20.0),
                                ..default()
                            },
                            ImageNode::from_atlas_image(
                                game_assets.futhark.clone(),
                                TextureAtlas {
                                    layout: game_assets.futhark_layout.clone(),
                                    index: icon_index,
                                },
                            ),
                        ));
                        for label in labels {
                            col.spawn((
                                Text::new(label),
                                TextFont {
                                    font: font_num.clone(),
                                    font_size: 12.0,
                                    ..default()
                                },
                                TextColor(body_color.with_alpha(0.85)),
                            ));
                        }
                    });
                }
            });
        });
}

fn handle_selection_click(
    choices: Query<(&Interaction, &SpellSelectionChoice), (Changed<Interaction>, With<Button>)>,
    skip: Query<&Interaction, (Changed<Interaction>, With<Button>, With<SpellSelectionSkip>)>,
    mut learned: ResMut<LearnedSpells>,
    mut selection: ResMut<SpellSelection>,
) {
    for (interaction, choice) in &choices {
        if *interaction == Interaction::Pressed {
            match choice.mode {
                SelectionMode::Learn | SelectionMode::Relearn => {
                    learned.insert(choice.word.clone());
                }
                SelectionMode::Unlearn => {
                    learned.remove_one(&choice.word);
                }
            }
            selection.close();
            return;
        }
    }
    for interaction in &skip {
        if *interaction == Interaction::Pressed {
            selection.close();
            return;
        }
    }
}

fn despawn_selection_modal_when_closed(
    mut commands: Commands,
    selection: Res<SpellSelection>,
    modals: Query<Entity, With<SpellSelectionModal>>,
) {
    if selection.is_open() {
        return;
    }
    for entity in &modals {
        commands.entity(entity).despawn();
    }
}

fn effect_sprite_index(effect: &SpellEffect) -> usize {
    match effect {
        SpellEffect::Damage { .. } => 250,
        SpellEffect::Shield { .. } => 249,
        SpellEffect::Stun { .. } => 248,
        SpellEffect::Buff { .. } => 247,
        SpellEffect::Binding { .. } => 246,
    }
}

fn effect_labels(effect: &SpellEffect) -> Vec<String> {
    match effect {
        SpellEffect::Damage { amount } => vec![format!("{amount}")],
        SpellEffect::Stun { amount } => vec![format!("{amount:.0}")],
        SpellEffect::Shield { amount, duration } => {
            vec![format!("{amount}"), format!("{duration:.0}s")]
        }
        SpellEffect::Buff { amount, duration } => {
            vec![format!("{amount}"), format!("{duration:.0}s")]
        }
        SpellEffect::Binding { amount } => vec![format!("{amount}")],
    }
}
