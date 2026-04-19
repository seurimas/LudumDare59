use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use std::collections::HashMap;

use crate::GameState;
use crate::rune_words::battle::{BattleSet, RowLetterGraded, RowResolved, RuneMatchState};
use crate::ui::hud_root::InscribedPanel;
use crate::ui::palette::*;

/// Marker for the active attempt card.
#[derive(Component)]
pub struct ActiveAttemptCard;

/// Flex row container inside `ActiveAttemptCard` where rune slot entities are parented.
#[derive(Component)]
pub struct RuneSlotRow;

/// Scrollable ledger of completed attempt rows (up to 4 visible).
#[derive(Component)]
pub struct LedgerList;

/// A single resolved attempt row displayed in the ledger.
#[derive(Component)]
pub struct AttemptRow {
    pub row_id: u32,
}

/// A single colored tile inside an `AttemptRow`.
#[derive(Component)]
pub struct AttemptRowTile;

/// Accumulates `RowLetterGraded` events per row_id until the row is resolved.
#[derive(Resource, Default)]
pub struct PendingLedgerData {
    pub rows: HashMap<u32, Vec<(char, RuneMatchState)>>,
}

pub fn configure_inscribed(app: &mut App) {
    app.init_resource::<PendingLedgerData>();
    app.add_systems(
        OnEnter(GameState::Ready),
        spawn_inscribed_ui.after(crate::ui::hud_root::spawn_battle_hud_root),
    );
    app.add_systems(
        Update,
        (accumulate_row_grading, populate_ledger_on_row_resolved)
            .chain()
            .after(BattleSet::PostAnimation)
            .run_if(in_state(GameState::Ready)),
    );
}

pub fn spawn_inscribed_ui(
    mut commands: Commands,
    panel_query: Query<Entity, With<InscribedPanel>>,
    game_assets: Option<Res<crate::GameAssets>>,
) {
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    let font_im_fell = game_assets.as_ref().map(|a| a.font_im_fell_sc.clone());

    commands.entity(panel_entity).with_children(|panel| {
        // ── Active Attempt Card ──
        panel
            .spawn((
                ActiveAttemptCard,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect {
                        left: Val::Percent(3.0),
                        right: Val::Percent(3.0),
                        top: Val::Percent(5.0),
                        bottom: Val::Percent(3.0),
                    },
                    border: UiRect::all(Val::Px(1.0)),
                    row_gap: Val::Percent(1.5),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.04, 0.02, 0.80)),
                BorderColor {
                    top: BLOOD,
                    right: BLOOD,
                    bottom: BLOOD,
                    left: BLOOD,
                },
            ))
            .with_children(|card| {
                // Floating "INSCRIBING" badge
                card.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Px(-10.0),
                        left: Val::Percent(6.0),
                        padding: UiRect {
                            left: Val::Px(5.0),
                            right: Val::Px(5.0),
                            top: Val::Px(1.0),
                            bottom: Val::Px(1.0),
                        },
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(NIGHT),
                    BorderColor {
                        top: BLOOD,
                        right: BLOOD,
                        bottom: BLOOD,
                        left: BLOOD,
                    },
                    children![(
                        Text::new("INSCRIBING"),
                        TextFont {
                            font: font_im_fell.clone().unwrap_or_default(),
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(BLOOD_BRIGHT),
                    )],
                ));

                // Rune slot row container
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    min_width: Val::Px(0.0),
                    column_gap: Val::Percent(2.0),
                    min_height: Val::Px(50.0),
                    overflow: Overflow::clip(),
                    ..default()
                })
                .with_children(|row_area| {
                    // RuneSlotRow: battle systems parent BattleRuneSlot entities here at runtime
                    row_area.spawn((
                        RuneSlotRow,
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            min_width: Val::Px(0.0),
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Percent(3.0),
                            row_gap: Val::Px(4.0),
                            ..default()
                        },
                    ));
                });
            });

        // ── Divider ──
        panel
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Percent(3.0),
                ..default()
            })
            .with_children(|divider| {
                divider.spawn((
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(1.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.55, 0.43, 0.17, 0.5)),
                ));
                divider.spawn((
                    Text::new("previous strokes"),
                    TextFont {
                        font: font_im_fell.clone().unwrap_or_default(),
                        font_size: 9.0,
                        ..default()
                    },
                    TextColor(PARCHMENT_DARK),
                ));
                divider.spawn((
                    Node {
                        flex_grow: 1.0,
                        height: Val::Px(1.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.55, 0.43, 0.17, 0.5)),
                ));
            });

        // ── Ledger ──
        panel.spawn((
            LedgerList,
            Node {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                flex_basis: Val::Px(0.0),
                row_gap: Val::Percent(2.0),
                overflow: Overflow::clip(),
                ..default()
            },
        ));
    });
}

fn accumulate_row_grading(
    mut events: MessageReader<RowLetterGraded>,
    mut pending: ResMut<PendingLedgerData>,
) {
    for event in events.read() {
        pending
            .rows
            .entry(event.row_id)
            .or_default()
            .push((event.letter, event.match_state));
    }
}

fn populate_ledger_on_row_resolved(
    mut commands: Commands,
    mut resolved: MessageReader<RowResolved>,
    mut pending: ResMut<PendingLedgerData>,
    last_word: Res<crate::rune_words::battle_states::LastGradedWord>,
    ledger_query: Query<Entity, With<LedgerList>>,
    existing_rows: Query<Entity, With<AttemptRow>>,
    game_assets: Option<Res<crate::GameAssets>>,
) {
    let row_ids: Vec<u32> = resolved.read().map(|r| r.0).collect();
    if row_ids.is_empty() {
        return;
    }

    let Ok(ledger_entity) = ledger_query.single() else {
        return;
    };

    let font_im_fell = game_assets.as_ref().map(|a| a.font_im_fell_sc.clone());
    let font_garamond = game_assets
        .as_ref()
        .map(|a| a.font_cormorant_garamond_italic.clone());

    for row_id in row_ids {
        let tiles = pending.rows.remove(&row_id).unwrap_or_default();
        let word = last_word.word.clone();

        // Evict oldest row when ledger is at capacity
        let existing: Vec<Entity> = existing_rows.iter().collect();
        if existing.len() >= 4 {
            if let Some(&oldest) = existing.first() {
                commands.entity(oldest).despawn();
            }
        }

        let index_str = match row_id {
            0 => "I.",
            1 => "II.",
            2 => "III.",
            3 => "IV.",
            4 => "V.",
            _ => "·",
        };

        let subtitle = match &word {
            Some(w) => format!("\"{}\"", w),
            None => "— unknown —".to_string(),
        };

        let attempt_row = commands
            .spawn((
                AttemptRow { row_id },
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::FlexStart,
                    column_gap: Val::Percent(3.0),
                    ..default()
                },
            ))
            .with_children(|row| {
                // Index numeral
                row.spawn((
                    Text::new(index_str.to_string()),
                    TextFont {
                        font: font_im_fell.clone().unwrap_or_default(),
                        font_size: 10.0,
                        ..default()
                    },
                    TextColor(PARCHMENT_DARK),
                ));

                // Tiles column + word subtitle
                row.spawn(Node {
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    row_gap: Val::Percent(1.0),
                    ..default()
                })
                .with_children(|col| {
                    // Tiles row
                    col.spawn(Node {
                        flex_direction: FlexDirection::Row,
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: Val::Px(2.0),
                        ..default()
                    })
                    .with_children(|tiles_row| {
                        for (_letter, match_state) in &tiles {
                            let tile_color = match_state.background_color();
                            tiles_row.spawn((
                                AttemptRowTile,
                                Node {
                                    width: Val::Px(14.0),
                                    height: Val::Px(14.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(tile_color),
                                BorderColor {
                                    top: Color::srgba(0.0, 0.0, 0.0, 0.5),
                                    right: Color::srgba(0.0, 0.0, 0.0, 0.5),
                                    bottom: Color::srgba(0.0, 0.0, 0.0, 0.5),
                                    left: Color::srgba(0.0, 0.0, 0.0, 0.5),
                                },
                            ));
                        }
                    });

                    // Word subtitle
                    col.spawn((
                        Text::new(subtitle),
                        TextFont {
                            font: font_garamond.clone().unwrap_or_default(),
                            font_size: 9.0,
                            ..default()
                        },
                        TextColor(PARCHMENT_DARK),
                    ));
                });
            })
            .id();

        commands.entity(ledger_entity).add_child(attempt_row);
    }

    // Fade oldest row if ledger has more than one entry.
    // Bevy has no inherited opacity, so we walk descendants and set alpha to 0.55.
    let current_rows: Vec<Entity> = existing_rows.iter().collect();
    if current_rows.len() > 1 {
        if let Some(&oldest) = current_rows.first() {
            // Fade marker is handled by a separate system if needed — stub only
            let _ = oldest;
        }
    }
}
