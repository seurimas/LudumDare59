use bevy::prelude::*;

use crate::GameAssets;
use crate::GameState;
use crate::health::{NpcCombatState, PlayerCombatState};
use crate::rune_words::battle::BattleState;
use crate::ui::hud_root::CombatBar;
use crate::ui::palette::*;

// ─── Components ───────────────────────────────────────────────────────────────

/// Left side combatant block (player).
#[derive(Component)]
struct PlayerCombatantBlock;

/// Right side combatant block (enemy).
#[derive(Component)]
struct EnemyCombatantBlock;

/// HP bar container node with overflow clipping.
#[derive(Component)]
struct HpBarOuter;

/// The fill node inside HP bar that shows current health.
#[derive(Component)]
struct HpBarFill;

#[derive(Component)]
struct PlayerHpBarFill;

#[derive(Component)]
struct PlayerShieldBarFill;

#[derive(Component)]
struct EnemyHpBarFill;

/// The tick overlay with 10 dividers.
#[derive(Component)]
struct TickOverlay;

/// The text label showing HP numbers.
#[derive(Component)]
struct HpLabel;

#[derive(Component)]
struct PlayerHpLabel;

#[derive(Component)]
struct EnemyHpLabel;

/// The phase banner in the center of the combat bar.
#[derive(Component)]
struct PhaseBannerNode;

/// The phase name text inside PhaseBannerNode.
#[derive(Component)]
struct PhaseNameText;

/// The three pips showing phase progress (inactive/active).
#[derive(Component)]
struct PhasePips;

/// Individual pip marker.
#[derive(Component)]
struct PhasePip {
    index: usize,
}

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_health_bars(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::Ready),
        spawn_combat_bar.after(crate::ui::hud_root::spawn_battle_hud_root),
    );
    app.add_systems(
        Update,
        (sync_hp_bars, sync_phase_banner).run_if(in_state(GameState::Ready)),
    );
}

// ─── Spawn Combat Bar ────────────────────────────────────────────────────────

fn spawn_combat_bar(
    mut commands: Commands,
    query: Query<Entity, With<CombatBar>>,
    assets: Res<GameAssets>,
) {
    let Ok(combat_bar_entity) = query.single() else {
        return;
    };

    commands
        .entity(combat_bar_entity)
        .insert(Node {
            grid_column: GridPlacement::span(3),
            grid_row: GridPlacement::start(1),
            min_width: Val::Percent(0.0),
            display: Display::Grid,
            grid_template_columns: vec![
                RepeatedGridTrack::fr(1, 1.0),
                RepeatedGridTrack::auto(1),
                RepeatedGridTrack::fr(1, 1.0),
            ],
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(Val::Percent(0.12)),
            padding: UiRect::all(Val::Percent(1.0)),
            row_gap: Val::Percent(1.0),
            column_gap: Val::Percent(1.0),
            overflow: Overflow::clip(),
            ..default()
        })
        .insert((
            BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.85)),
            BorderColor {
                top: GOLD_DARK,
                right: GOLD_DARK,
                bottom: GOLD_DARK,
                left: GOLD_DARK,
            },
        ))
        .with_children(|combat_bar| {
            // ────────── Left: Player Combatant ──────────
            combat_bar
                .spawn((
                    PlayerCombatantBlock,
                    Node {
                        grid_column: GridPlacement::start(1),
                        grid_row: GridPlacement::start(1),
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Percent(2.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_children(|block| {
                    // Name and HP
                    block
                        .spawn((Node {
                            min_width: Val::Px(0.0),
                            max_width: Val::Percent(84.0),
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            row_gap: Val::Percent(1.0),
                            ..default()
                        },))
                        .with_children(|col| {
                            col.spawn((
                                Text::new("Player"),
                                TextFont {
                                    font: assets.font_cormorant_unicase_semibold.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(GOLD_LIGHT),
                            ));

                            // Player HP bar
                            col.spawn((
                                HpBarOuter,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_width: Val::Px(0.0),
                                    aspect_ratio: Some(30.0),
                                    border: UiRect::all(Val::Percent(0.12)),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BorderColor {
                                    top: GOLD_DARK,
                                    right: GOLD_DARK,
                                    bottom: GOLD_DARK,
                                    left: GOLD_DARK,
                                },
                                BackgroundColor(NIGHT),
                            ))
                            .with_children(|hp_bar| {
                                hp_bar.spawn((
                                    HpBarFill,
                                    PlayerHpBarFill,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Percent(0.0),
                                        bottom: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        width: Val::Percent(100.0),
                                        left: Val::Percent(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(BLOOD),
                                ));

                                // Shield bar overlay (blue, positioned after HP fill)
                                hp_bar.spawn((
                                    PlayerShieldBarFill,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Percent(0.0),
                                        bottom: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        width: Val::Percent(0.0),
                                        left: Val::Percent(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(MANA_BRIGHT.with_alpha(0.6)),
                                ));

                                hp_bar
                                    .spawn((
                                        TickOverlay,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Row,
                                            ..default()
                                        },
                                    ))
                                    .with_children(|ticks| {
                                        for i in 0..10 {
                                            ticks.spawn((
                                                Node {
                                                    flex_grow: 1.0,
                                                    border: if i < 9 {
                                                        UiRect {
                                                            right: Val::Percent(0.12),
                                                            ..default()
                                                        }
                                                    } else {
                                                        UiRect::default()
                                                    },
                                                    ..default()
                                                },
                                                BorderColor {
                                                    top: GOLD_DARK,
                                                    right: GOLD_DARK,
                                                    bottom: GOLD_DARK,
                                                    left: GOLD_DARK,
                                                },
                                            ));
                                        }
                                    });

                                hp_bar.spawn((
                                    HpLabel,
                                    PlayerHpLabel,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Percent(0.0),
                                        left: Val::Percent(0.0),
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        display: Display::Flex,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    Text::new("100 / 100"),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(PARCHMENT),
                                ));
                            });
                        });
                });

            // ────────── Center: Phase Banner ──────────
            combat_bar
                .spawn((
                    PhaseBannerNode,
                    Node {
                        grid_column: GridPlacement::start(2),
                        grid_row: GridPlacement::start(1),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Percent(1.0),
                        ..default()
                    },
                ))
                .with_children(|banner| {
                    banner.spawn((
                        Text::new("current phase"),
                        TextFont {
                            font: assets.font_im_fell_sc.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(PARCHMENT_DARK),
                    ));

                    banner.spawn((
                        PhaseNameText,
                        Text::new("Combat"),
                        TextFont {
                            font: assets.font_cormorant_unicase_bold.clone(),
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(GOLD_LIGHT),
                    ));

                    banner
                        .spawn((
                            PhasePips,
                            Node {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Row,
                                column_gap: Val::Percent(2.0),
                                margin: UiRect::top(Val::Percent(1.0)),
                                ..default()
                            },
                        ))
                        .with_children(|pips| {
                            for i in 0..3 {
                                pips.spawn((
                                    PhasePip { index: i },
                                    Node {
                                        width: Val::Percent(8.0),
                                        aspect_ratio: Some(1.0),
                                        border_radius: BorderRadius::MAX,
                                        ..default()
                                    },
                                    BorderColor {
                                        top: GOLD,
                                        right: GOLD,
                                        bottom: GOLD,
                                        left: GOLD,
                                    },
                                    BackgroundColor(if i == 0 { GOLD } else { NIGHT }),
                                ));
                            }
                        });
                });

            // ────────── Right: Enemy Combatant ────────
            combat_bar
                .spawn((
                    EnemyCombatantBlock,
                    Node {
                        grid_column: GridPlacement::start(3),
                        grid_row: GridPlacement::start(1),
                        min_width: Val::Px(0.0),
                        flex_direction: FlexDirection::RowReverse,
                        align_items: AlignItems::Center,
                        column_gap: Val::Percent(2.0),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_children(|block| {
                    // Name and HP (mirrored)
                    block
                        .spawn((Node {
                            min_width: Val::Px(0.0),
                            max_width: Val::Percent(84.0),
                            flex_direction: FlexDirection::Column,
                            flex_grow: 1.0,
                            flex_basis: Val::Px(0.0),
                            row_gap: Val::Percent(1.0),
                            align_items: AlignItems::FlexEnd,
                            ..default()
                        },))
                        .with_children(|col| {
                            col.spawn((
                                Text::new("Enemy"),
                                TextFont {
                                    font: assets.font_cormorant_unicase_semibold.clone(),
                                    font_size: 16.0,
                                    ..default()
                                },
                                TextColor(GOLD_LIGHT),
                            ));

                            // Enemy HP bar
                            col.spawn((
                                HpBarOuter,
                                Node {
                                    width: Val::Percent(100.0),
                                    min_width: Val::Px(0.0),
                                    aspect_ratio: Some(30.0),
                                    border: UiRect::all(Val::Percent(0.12)),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BorderColor {
                                    top: GOLD_DARK,
                                    right: GOLD_DARK,
                                    bottom: GOLD_DARK,
                                    left: GOLD_DARK,
                                },
                                BackgroundColor(NIGHT),
                            ))
                            .with_children(|hp_bar| {
                                hp_bar.spawn((
                                    HpBarFill,
                                    EnemyHpBarFill,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Percent(0.0),
                                        bottom: Val::Percent(0.0),
                                        height: Val::Percent(100.0),
                                        width: Val::Percent(100.0),
                                        right: Val::Percent(0.0),
                                        ..default()
                                    },
                                    BackgroundColor(BLOOD),
                                ));

                                hp_bar
                                    .spawn((
                                        TickOverlay,
                                        Node {
                                            position_type: PositionType::Absolute,
                                            width: Val::Percent(100.0),
                                            height: Val::Percent(100.0),
                                            display: Display::Flex,
                                            flex_direction: FlexDirection::Row,
                                            ..default()
                                        },
                                    ))
                                    .with_children(|ticks| {
                                        for i in 0..10 {
                                            ticks.spawn((
                                                Node {
                                                    flex_grow: 1.0,
                                                    border: if i < 9 {
                                                        UiRect {
                                                            right: Val::Percent(0.12),
                                                            ..default()
                                                        }
                                                    } else {
                                                        UiRect::default()
                                                    },
                                                    ..default()
                                                },
                                                BorderColor {
                                                    top: GOLD_DARK,
                                                    right: GOLD_DARK,
                                                    bottom: GOLD_DARK,
                                                    left: GOLD_DARK,
                                                },
                                            ));
                                        }
                                    });

                                hp_bar.spawn((
                                    HpLabel,
                                    EnemyHpLabel,
                                    Node {
                                        position_type: PositionType::Absolute,
                                        top: Val::Percent(0.0),
                                        left: Val::Percent(0.0),
                                        width: Val::Percent(100.0),
                                        height: Val::Percent(100.0),
                                        display: Display::Flex,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    Text::new("100 / 100"),
                                    TextFont {
                                        font_size: 12.0,
                                        ..default()
                                    },
                                    TextColor(PARCHMENT),
                                ));
                            });
                        });
                });
        });
}

// ─── Systems ──────────────────────────────────────────────────────────────────

fn sync_hp_bars(
    mut player_fill_query: Query<
        &mut Node,
        (
            With<HpBarFill>,
            With<PlayerHpBarFill>,
            Without<EnemyHpBarFill>,
            Without<PlayerShieldBarFill>,
        ),
    >,
    mut shield_fill_query: Query<
        &mut Node,
        (
            With<PlayerShieldBarFill>,
            Without<HpBarFill>,
            Without<EnemyHpBarFill>,
        ),
    >,
    mut enemy_fill_query: Query<
        &mut Node,
        (
            With<HpBarFill>,
            With<EnemyHpBarFill>,
            Without<PlayerHpBarFill>,
            Without<PlayerShieldBarFill>,
        ),
    >,
    mut player_label_query: Query<
        &mut Text,
        (With<HpLabel>, With<PlayerHpLabel>, Without<EnemyHpLabel>),
    >,
    mut enemy_label_query: Query<
        &mut Text,
        (With<HpLabel>, With<EnemyHpLabel>, Without<PlayerHpLabel>),
    >,
    player_health: Res<PlayerCombatState>,
    npc_query: Query<&NpcCombatState>,
) {
    let active_npc_health = npc_query.iter().next();

    if let Some(health) = active_npc_health {
        let hp = health.hp as f32;
        let max = health.max as f32;
        let fill_percent = (hp / max * 100.0).clamp(0.0, 100.0);

        if let Ok(mut node) = enemy_fill_query.single_mut() {
            node.width = Val::Percent(fill_percent);
        }

        if let Ok(mut text) = enemy_label_query.single_mut() {
            text.0 = format!("{} / {}", health.hp, health.max);
        }
    }

    let player_hp = player_health.hp as f32;
    let player_max = player_health.max as f32;
    let player_fill_percent = (player_hp / player_max * 100.0).clamp(0.0, 100.0);

    if let Ok(mut node) = player_fill_query.single_mut() {
        node.width = Val::Percent(player_fill_percent);
    }

    // Shield bar: shows as a blue overlay starting at the HP fill edge
    let total_shield: f32 = player_health.shields.iter().map(|s| s.amount as f32).sum();
    let shield_pct = (total_shield / player_max * 100.0).clamp(0.0, 100.0 - player_fill_percent);
    if let Ok(mut node) = shield_fill_query.single_mut() {
        node.left = Val::Percent(player_fill_percent);
        node.width = Val::Percent(shield_pct);
    }

    if let Ok(mut text) = player_label_query.single_mut() {
        if total_shield > 0.0 {
            text.0 = format!(
                "{} +{} / {}",
                player_health.hp, total_shield as u32, player_health.max
            );
        } else {
            text.0 = format!("{} / {}", player_health.hp, player_health.max);
        }
    }
}

fn sync_phase_banner(
    battle_state: Res<BattleState>,
    mut phase_text: Query<&mut Text, With<PhaseNameText>>,
    mut pips: Query<&mut BackgroundColor, With<PhasePip>>,
) {
    if let Ok(mut text) = phase_text.single_mut() {
        text.0 = "Combat".to_string();
    }

    let phase_index = battle_state.phase.phase_index();
    for (i, mut bg_color) in pips.iter_mut().enumerate() {
        bg_color.0 = if i <= phase_index { GOLD } else { NIGHT };
    }
}
