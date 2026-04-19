use bevy::prelude::*;

use crate::GameAssets;
use crate::GameState;
use crate::futhark::{SPRITE_RUNE_OFFSET, letter_to_index};
use crate::health::PlayerCombatState;
use crate::ui::clock::BattleUiClock;
use crate::ui::hud_root::BookPanel;
use crate::ui::palette::*;

// ─── Components ───────────────────────────────────────────────────────────────

/// Inner parchment page node.
#[derive(Component)]
struct BookPage;

/// One row entry in the Book of Combat.
#[derive(Component)]
struct SpellEntry {
    index: usize,
}

/// Dropcap text inside a SpellEntry.
#[derive(Component)]
struct SpellEntryDropcap {
    index: usize,
}

/// Word name text inside a SpellEntry.
#[derive(Component)]
struct SpellEntryWord {
    index: usize,
}

/// Rune glyph row container inside a SpellEntry.
#[derive(Component)]
struct SpellEntryRuneRow {
    index: usize,
}

/// Sigil circle placeholder inside a SpellEntry.
#[derive(Component)]
struct SpellEntrySigil {
    index: usize,
}

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_book(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_book_panel.after(crate::ui::hud_root::spawn_battle_hud_root),
    );
    app.add_systems(
        Update,
        sync_book_panel.run_if(in_state(GameState::Adventure)),
    );
    app.add_systems(
        Update,
        pulse_active_pointer.run_if(in_state(GameState::Adventure)),
    );
}

// ─── Spawn ────────────────────────────────────────────────────────────────────

pub fn spawn_book_panel(
    mut commands: Commands,
    panel_query: Query<(Entity, Option<&Children>), With<BookPanel>>,
    game_assets: Res<GameAssets>,
) {
    let Ok((panel_entity, maybe_children)) = panel_query.single() else {
        return;
    };

    // Remove placeholder children.
    if let Some(children) = maybe_children {
        for child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let font_heading = game_assets.font_cormorant_unicase_semibold.clone();
    let font_aside = game_assets.font_im_fell_sc.clone();
    let font_word = game_assets.font_cormorant_unicase_bold.clone();
    let font_dropcap = game_assets.font_unifraktur.clone();
    let font_page = game_assets.font_cormorant_garamond_italic.clone();

    // Upgrade the panel node: dark leather, flex-col.
    commands
        .entity(panel_entity)
        .insert((
            Node {
                grid_column: GridPlacement::start(3),
                grid_row: GridPlacement::start(2),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Percent(1.5),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect {
                    left: Val::Percent(3.0),
                    right: Val::Percent(3.0),
                    top: Val::Percent(2.5),
                    bottom: Val::Percent(2.5),
                },
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.90)),
            BorderColor {
                top: GOLD_DARK,
                right: GOLD_DARK,
                bottom: GOLD_DARK,
                left: GOLD_DARK,
            },
        ))
        .with_children(|panel| {
            // ── Panel header ─────────────────────────────────────────────────
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    padding: UiRect {
                        bottom: Val::Px(4.0),
                        ..default()
                    },
                    border: UiRect {
                        bottom: Val::Px(1.0),
                        ..default()
                    },
                    ..default()
                })
                .insert(BorderColor {
                    bottom: GOLD_DARK.with_alpha(0.5),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("Book of Combat"),
                        TextFont {
                            font: font_heading.clone(),
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(GOLD_LIGHT),
                    ));
                    header.spawn((
                        Text::new("choose · inscribe"),
                        TextFont {
                            font: font_aside.clone(),
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(PARCHMENT_DARK),
                    ));
                });

            // ── BookPage ──────────────────────────────────────────────────────
            panel
                .spawn((
                    BookPage,
                    Node {
                        flex_grow: 1.0,
                        flex_basis: Val::Percent(0.0),
                        flex_direction: FlexDirection::Column,
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect {
                            left: Val::Percent(4.0),
                            right: Val::Percent(4.0),
                            top: Val::Percent(3.0),
                            bottom: Val::Percent(3.0),
                        },
                        row_gap: Val::Percent(1.5),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(PARCHMENT_WARM),
                    BorderColor {
                        top: Color::srgba(0.31, 0.22, 0.10, 0.4),
                        right: Color::srgba(0.31, 0.22, 0.10, 0.4),
                        bottom: Color::srgba(0.31, 0.22, 0.10, 0.4),
                        left: Color::srgba(0.31, 0.22, 0.10, 0.4),
                    },
                ))
                .with_children(|page| {
                    // Red bookmark tab (absolute, top edge)
                    page.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Px(0.0),
                            right: Val::Percent(20.0),
                            width: Val::Percent(6.0),
                            height: Val::Percent(7.0),
                            ..default()
                        },
                        BackgroundColor(BLOOD),
                    ));

                    // Page head rule
                    page.spawn((
                        Text::new("⸺ grimoire · folio xxiv ⸺"),
                        TextFont {
                            font: font_page.clone(),
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(INK.with_alpha(0.55)),
                        Node {
                            margin: UiRect {
                                bottom: Val::Percent(2.0),
                                ..default()
                            },
                            ..default()
                        },
                    ));

                    // Spells list: 4 SpellEntry rows
                    page.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        flex_basis: Val::Percent(0.0),
                        min_height: Val::Px(0.0),
                        overflow: Overflow::clip(),
                        ..default()
                    })
                    .with_children(|spells| {
                        for index in 0..4usize {
                            spells
                                .spawn((
                                    SpellEntry { index },
                                    Node {
                                        height: Val::Percent(25.0),
                                        min_height: Val::Px(0.0),
                                        overflow: Overflow::clip(),
                                        display: Display::Grid,
                                        grid_template_columns: vec![
                                            RepeatedGridTrack::auto(1),
                                            RepeatedGridTrack::fr(1, 1.0),
                                            RepeatedGridTrack::auto(1),
                                        ],
                                        column_gap: Val::Percent(3.0),
                                        align_items: AlignItems::Center,
                                        padding: UiRect {
                                            top: Val::Percent(2.0),
                                            bottom: Val::Percent(2.0),
                                            ..default()
                                        },
                                        border: UiRect {
                                            bottom: Val::Px(1.0),
                                            left: Val::Px(2.0),
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    BackgroundColor(Color::NONE),
                                    BorderColor {
                                        bottom: Color::srgba(0.48, 0.37, 0.19, 0.4),
                                        left: Color::NONE,
                                        ..default()
                                    },
                                ))
                                .with_children(|entry| {
                                    // ── Dropcap ──
                                    entry.spawn((
                                        SpellEntryDropcap { index },
                                        Text::new(""),
                                        TextFont {
                                            font: font_dropcap.clone(),
                                            font_size: 28.0,
                                            ..default()
                                        },
                                        TextColor(BLOOD),
                                    ));

                                    // ── Content ──
                                    entry
                                        .spawn(Node {
                                            flex_direction: FlexDirection::Column,
                                            row_gap: Val::Px(2.0),
                                            ..default()
                                        })
                                        .with_children(|content| {
                                            content.spawn((
                                                SpellEntryWord { index },
                                                Text::new("· · ·"),
                                                TextFont {
                                                    font: font_word.clone(),
                                                    font_size: 13.0,
                                                    ..default()
                                                },
                                                TextColor(INK.with_alpha(0.35)),
                                            ));
                                            content.spawn((
                                                SpellEntryRuneRow { index },
                                                Node {
                                                    flex_direction: FlexDirection::Row,
                                                    column_gap: Val::Px(1.0),
                                                    ..default()
                                                },
                                            ));
                                        });

                                    // ── Sigil ──
                                    entry
                                        .spawn((
                                            SpellEntrySigil { index },
                                            Node {
                                                width: Val::Px(22.0),
                                                height: Val::Px(22.0),
                                                border: UiRect::all(Val::Px(1.5)),
                                                border_radius: BorderRadius::all(Val::Percent(
                                                    50.0,
                                                )),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
                                                overflow: Overflow::clip(),
                                                ..default()
                                            },
                                            BackgroundColor(PARCHMENT_SHADOW.with_alpha(0.3)),
                                            BorderColor {
                                                top: GOLD_DARK,
                                                right: GOLD_DARK,
                                                bottom: GOLD_DARK,
                                                left: GOLD_DARK,
                                            },
                                        ))
                                        .with_children(|sigil| {
                                            sigil.spawn((
                                                Node {
                                                    width: Val::Px(16.0),
                                                    height: Val::Px(16.0),
                                                    ..default()
                                                },
                                                ImageNode::from_atlas_image(
                                                    game_assets.sigils.clone(),
                                                    TextureAtlas {
                                                        layout: game_assets.sigils_layout.clone(),
                                                        index,
                                                    },
                                                ),
                                            ));
                                        });
                                });
                        }
                    });
                });
        });
}

// ─── Sync ─────────────────────────────────────────────────────────────────────

fn sync_book_panel(
    mut commands: Commands,
    player: Res<PlayerCombatState>,
    game_assets: Option<Res<GameAssets>>,
    mut dropcap_query: Query<(&SpellEntryDropcap, &mut Text, &mut TextColor)>,
    mut word_query: Query<(&SpellEntryWord, &mut Text, &mut TextColor), Without<SpellEntryDropcap>>,
    rune_row_query: Query<(Entity, &SpellEntryRuneRow)>,
    children_query: Query<&Children>,
) {
    if !player.is_changed() {
        return;
    }

    let Some(game_assets) = game_assets else {
        return;
    };

    let entries = first_four_entries(&player.hand);

    // Update dropcaps.
    for (dropcap, mut text, mut color) in &mut dropcap_query {
        if let Some(entry) = entries[dropcap.index].as_ref() {
            let first_char = entry.word.chars().next().unwrap_or('?');
            **text = first_char.to_uppercase().to_string();
            color.0 = BLOOD;
        } else {
            **text = String::new();
        }
    }

    // Update word names.
    for (word_comp, mut text, mut color) in &mut word_query {
        if let Some(entry) = entries[word_comp.index].as_ref() {
            **text = entry.word.to_uppercase();
            color.0 = INK;
        } else {
            **text = "· · ·".to_string();
            color.0 = INK.with_alpha(0.35);
        }
    }

    // Update rune rows: despawn old glyph sprites, spawn new.
    for (rune_row_entity, rune_row) in &rune_row_query {
        // Despawn existing glyph children.
        if let Ok(children) = children_query.get(rune_row_entity) {
            for child in children.iter() {
                commands.entity(child).despawn();
            }
        }

        let Some(entry) = entries[rune_row.index].as_ref() else {
            continue;
        };

        // Spawn one futhark sprite per letter.
        commands.entity(rune_row_entity).with_children(|row| {
            for letter in entry.letters.chars() {
                let atlas_index = letter_to_index(letter)
                    .map(|i| i + SPRITE_RUNE_OFFSET)
                    .unwrap_or(SPRITE_RUNE_OFFSET);

                row.spawn((
                    Node {
                        width: Val::Px(12.0),
                        height: Val::Px(12.0),
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
    }
}

fn first_four_entries(
    hand: &[crate::spellbook::SpellDef],
) -> [Option<crate::dictionary::Futharkation>; 4] {
    let mut entries: [Option<crate::dictionary::Futharkation>; 4] = [None, None, None, None];
    for (i, spell) in hand.iter().take(4).enumerate() {
        entries[i] = Some(spell.as_futharkation());
    }
    entries
}

// ─── Active pointer pulse ─────────────────────────────────────────────────────

/// Marker for the `☛` pointer node shown beside the active spell entry.
#[derive(Component)]
struct ActiveSpellPointer;

fn pulse_active_pointer(
    clock: Res<BattleUiClock>,
    mut pointer_query: Query<&mut TextColor, With<ActiveSpellPointer>>,
) {
    use std::f32::consts::TAU;
    let alpha = 0.4 + 0.6 * (0.5 - 0.5 * ((clock.elapsed * 2.0) % TAU).cos());
    for mut color in &mut pointer_query {
        color.0 = BLOOD_BRIGHT.with_alpha(alpha);
    }
}
