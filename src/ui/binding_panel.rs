use bevy::prelude::*;

use crate::GameAssets;
use crate::GameState;
use crate::health::NpcCombatState;
use crate::rune_words::battle::{BattlePhase, BattleState};
use crate::ui::hud_root::BindingPanel;
use crate::ui::palette::*;

const BINDING_ICON_INDEX: usize = 246;

/// Legend entries: (sprite atlas index, name, description of the numbers).
const LEGEND_ENTRIES: &[(usize, &str, &str)] = &[
    (250, "Damage", "hp"),
    (249, "Shield", "hp / seconds"),
    (248, "Stun", "seconds"),
    (247, "Buff", "+dmg / seconds"),
    (246, "Binding", "stacks"),
];

// ─── Components ───────────────────────────────────────────────────────────────

/// Container for the binding icons (always visible).
#[derive(Component)]
struct BindingIconsArea;

/// Container for the word list (only visible during Binding phase).
#[derive(Component)]
struct BindingWordListArea;

/// Container for the spell-icon legend (visible outside Binding phase).
#[derive(Component)]
struct BindingLegendArea;

/// Marker for individual binding icon nodes so we can despawn/rebuild them.
#[derive(Component)]
struct BindingIcon;

/// Marker for the word list text node.
#[derive(Component)]
struct BindingWordListText;

/// Header text above the word list.
#[derive(Component)]
struct BindingWordListHeader;

/// Inner flex-wrap container that holds the word text nodes.
#[derive(Component)]
struct BindingWordListContainer;

/// Tracks the last-rendered state to avoid despawning/rebuilding every frame.
#[derive(Resource, Default)]
struct BindingPanelState {
    last_binding_count: u32,
    last_is_binding_phase: bool,
}

// ─── Configure ────────────────────────────────────────────────────────────────

pub fn configure_binding_panel(app: &mut App) {
    app.init_resource::<BindingPanelState>();
    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_binding_panel.after(crate::ui::hud_root::spawn_battle_hud_root),
    );
    app.add_systems(
        Update,
        sync_binding_panel.run_if(in_state(GameState::Adventure)),
    );
}

// ─── Spawn ────────────────────────────────────────────────────────────────────

fn spawn_binding_panel(
    mut commands: Commands,
    panel_query: Query<(Entity, Option<&Children>), With<BindingPanel>>,
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

    // Style the panel itself.
    commands
        .entity(panel_entity)
        .insert((
            Node {
                grid_column: GridPlacement::span(3),
                grid_row: GridPlacement::start(3),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect {
                    left: Val::Percent(2.0),
                    right: Val::Percent(2.0),
                    top: Val::Percent(1.0),
                    bottom: Val::Percent(1.0),
                },
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.85)),
            BorderColor {
                top: GOLD_DARK,
                right: GOLD_DARK,
                bottom: GOLD_DARK,
                left: GOLD_DARK,
            },
        ))
        .with_children(|panel| {
            // ── Header ───────────────────────────────────────────────────────
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.0),
                    ..default()
                })
                .with_children(|header| {
                    header.spawn((
                        Text::new("Bindings"),
                        TextFont {
                            font: font_heading.clone(),
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(GOLD_LIGHT),
                    ));
                });

            // ── Content row: icons area + word list area ─────────────────────
            panel
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|content| {
                    // Icons area (flex-wrap row of binding icons)
                    content.spawn((
                        BindingIconsArea,
                        Node {
                            flex_direction: FlexDirection::Row,
                            flex_wrap: FlexWrap::Wrap,
                            column_gap: Val::Px(4.0),
                            row_gap: Val::Px(2.0),
                            align_items: AlignItems::Center,
                            min_width: Val::Percent(25.0),
                            ..default()
                        },
                    ));

                    // Word list area (hidden unless in binding phase)
                    content
                        .spawn(Node {
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Px(2.0),
                            display: Display::None,
                            ..default()
                        })
                        .insert(BindingWordListArea)
                        .with_children(|word_col| {
                            word_col.spawn((
                                BindingWordListHeader,
                                Text::new("Known Binding Words"),
                                TextFont {
                                    font: font_heading.clone(),
                                    font_size: 11.0,
                                    ..default()
                                },
                                TextColor(GOLD_LIGHT),
                            ));
                            word_col.spawn((
                                BindingWordListContainer,
                                Node {
                                    flex_direction: FlexDirection::Row,
                                    flex_wrap: FlexWrap::Wrap,
                                    column_gap: Val::Px(8.0),
                                    row_gap: Val::Px(2.0),
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                            ));
                        });

                    // Legend area (hidden during binding phase)
                    content
                        .spawn((
                            BindingLegendArea,
                            Node {
                                flex_grow: 1.0,
                                flex_direction: FlexDirection::Row,
                                flex_wrap: FlexWrap::Wrap,
                                column_gap: Val::Px(12.0),
                                row_gap: Val::Px(2.0),
                                align_items: AlignItems::Center,
                                ..default()
                            },
                        ))
                        .with_children(|legend| {
                            for (icon_index, name, description) in LEGEND_ENTRIES {
                                legend
                                    .spawn(Node {
                                        flex_direction: FlexDirection::Row,
                                        column_gap: Val::Px(4.0),
                                        align_items: AlignItems::Center,
                                        ..default()
                                    })
                                    .with_children(|entry| {
                                        entry.spawn((
                                            Node {
                                                width: Val::Px(16.0),
                                                height: Val::Px(16.0),
                                                ..default()
                                            },
                                            ImageNode::from_atlas_image(
                                                game_assets.futhark.clone(),
                                                TextureAtlas {
                                                    layout: game_assets.futhark_layout.clone(),
                                                    index: *icon_index,
                                                },
                                            ),
                                        ));
                                        entry.spawn((
                                            Text::new(format!("{name} — {description}")),
                                            TextFont {
                                                font: font_aside.clone(),
                                                font_size: 10.0,
                                                ..default()
                                            },
                                            TextColor(PARCHMENT),
                                        ));
                                    });
                            }
                        });
                });
        });
}

// ─── Sync ─────────────────────────────────────────────────────────────────────

fn sync_binding_panel(
    mut commands: Commands,
    battle_state: Res<BattleState>,
    npcs: Query<&NpcCombatState>,
    game_assets: Option<Res<GameAssets>>,
    icons_area_query: Query<Entity, With<BindingIconsArea>>,
    word_list_container_query: Query<Entity, With<BindingWordListContainer>>,
    existing_icons: Query<Entity, With<BindingIcon>>,
    existing_words: Query<Entity, With<BindingWordListText>>,
    mut icons_area_node_query: Query<
        &mut Node,
        (
            With<BindingIconsArea>,
            Without<BindingWordListArea>,
            Without<BindingLegendArea>,
        ),
    >,
    mut word_list_node_query: Query<
        &mut Node,
        (
            With<BindingWordListArea>,
            Without<BindingIconsArea>,
            Without<BindingLegendArea>,
        ),
    >,
    mut legend_node_query: Query<
        &mut Node,
        (
            With<BindingLegendArea>,
            Without<BindingIconsArea>,
            Without<BindingWordListArea>,
        ),
    >,
    mut panel_state: ResMut<BindingPanelState>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };

    let Ok(icons_area_entity) = icons_area_query.single() else {
        return;
    };
    let Ok(word_list_container) = word_list_container_query.single() else {
        return;
    };

    let is_binding_phase = matches!(battle_state.phase, BattlePhase::Binding);

    // Get NPC binding count.
    let binding_count: u32 = npcs.iter().map(|npc| npc.bindings).sum();

    // Skip rebuild if nothing changed.
    if binding_count == panel_state.last_binding_count
        && is_binding_phase == panel_state.last_is_binding_phase
    {
        return;
    }
    panel_state.last_binding_count = binding_count;
    panel_state.last_is_binding_phase = is_binding_phase;

    // ── Update icons area ────────────────────────────────────────────────────
    // Despawn old icons and rebuild.
    for entity in &existing_icons {
        commands.entity(entity).despawn();
    }

    commands.entity(icons_area_entity).with_children(|area| {
        for _ in 0..binding_count {
            area.spawn((
                BindingIcon,
                Node {
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    ..default()
                },
                ImageNode::from_atlas_image(
                    game_assets.futhark.clone(),
                    TextureAtlas {
                        layout: game_assets.futhark_layout.clone(),
                        index: BINDING_ICON_INDEX,
                    },
                ),
            ));
        }
    });

    // ── Update word list / legend visibility ─────────────────────────────────
    if let Ok(mut word_list_node) = word_list_node_query.single_mut() {
        word_list_node.display = if is_binding_phase {
            Display::Flex
        } else {
            Display::None
        };
    }
    if let Ok(mut legend_node) = legend_node_query.single_mut() {
        legend_node.display = if is_binding_phase {
            Display::None
        } else {
            Display::Flex
        };
    }

    // Icons area stays at a fixed share now that something always sits beside it.
    if let Ok(mut icons_node) = icons_area_node_query.single_mut() {
        icons_node.flex_grow = 0.0;
        icons_node.width = Val::Percent(25.0);
    }

    // ── Update word list content ─────────────────────────────────────────────
    // Despawn old word texts.
    for entity in &existing_words {
        commands.entity(entity).despawn();
    }

    if is_binding_phase {
        // Get binding words from the NPC spec.
        let binding_words: Vec<String> = battle_state
            .npc
            .as_ref()
            .map(|spec| spec.binding_words.clone())
            .unwrap_or_default();

        let font = game_assets.font_im_fell_sc.clone();
        commands.entity(word_list_container).with_children(|area| {
            for word in &binding_words {
                area.spawn((
                    BindingWordListText,
                    Text::new(word.as_str()),
                    TextFont {
                        font: font.clone(),
                        font_size: 11.0,
                        ..default()
                    },
                    TextColor(PARCHMENT),
                    Node {
                        margin: UiRect {
                            right: Val::Px(4.0),
                            ..default()
                        },
                        ..default()
                    },
                ));
            }
        });
    }
}
