use bevy::prelude::*;

use crate::GameState;
use crate::ui::palette::{GOLD_DARK, PARCHMENT_DARK};

#[derive(Component)]
pub struct BattleHudWrapper;

#[derive(Component)]
pub struct BattleHudRoot;

#[derive(Component)]
pub struct CombatBar;

#[derive(Component)]
pub struct LeftColumn;

#[derive(Component)]
pub struct InscribedPanel;

#[derive(Component)]
pub struct ArenaPanel;

#[derive(Component)]
pub struct BookPanel;

#[derive(Component)]
pub struct BindingPanel;

pub fn configure_hud_root(app: &mut App) {
    app.add_systems(OnEnter(GameState::Ready), spawn_battle_hud_root);
}

pub fn spawn_battle_hud_root(mut commands: Commands) {
    commands
        .spawn((
            BattleHudWrapper,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|wrapper| {
            wrapper
                .spawn((
                    BattleHudRoot,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        aspect_ratio: Some(16.0 / 9.0),
                        display: Display::Grid,
                        grid_template_columns: vec![
                            RepeatedGridTrack::fr(1, 22.0),
                            RepeatedGridTrack::fr(1, 50.0),
                            RepeatedGridTrack::fr(1, 22.0),
                        ],
                        grid_template_rows: vec![
                            RepeatedGridTrack::auto(1),
                            RepeatedGridTrack::fr(1, 1.0),
                            RepeatedGridTrack::auto(1),
                        ],
                        column_gap: Val::Percent(1.0),
                        row_gap: Val::Percent(1.0),
                        padding: UiRect::all(Val::Percent(1.4)),
                        ..default()
                    },
                ))
                .with_children(|grid| {
                    grid.spawn((
                        CombatBar,
                        placeholder_node(GridPlacement::span(3), GridPlacement::start(1)),
                        placeholder_background(),
                        placeholder_border_color(),
                        children![placeholder_label("Combat Bar")],
                    ));

                    grid.spawn((
                        LeftColumn,
                        Node {
                            grid_column: GridPlacement::start(1),
                            grid_row: GridPlacement::start(2),
                            flex_direction: FlexDirection::Column,
                            row_gap: Val::Percent(1.0),
                            ..default()
                        },
                    ))
                    .with_children(|left_column| {
                        left_column.spawn((
                            InscribedPanel,
                            Node {
                                flex_grow: 1.0,
                                flex_basis: Val::Px(0.0),
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Percent(1.5),
                                padding: UiRect::all(Val::Percent(1.5)),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                        ));
                    });

                    grid.spawn((
                        ArenaPanel,
                        Node {
                            grid_column: GridPlacement::start(2),
                            grid_row: GridPlacement::start(2),
                            overflow: Overflow::clip(),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                    ));

                    grid.spawn((
                        BookPanel,
                        placeholder_node(GridPlacement::start(3), GridPlacement::start(2)),
                        placeholder_background(),
                        placeholder_border_color(),
                        children![placeholder_label("Book Panel")],
                    ));

                    grid.spawn((
                        BindingPanel,
                        placeholder_node(GridPlacement::span(3), GridPlacement::start(3)),
                        placeholder_background(),
                        placeholder_border_color(),
                        children![placeholder_label("Binding Panel")],
                    ));
                });
        });
}

fn placeholder_node(column: GridPlacement, row: GridPlacement) -> Node {
    Node {
        grid_column: column,
        grid_row: row,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        border: UiRect::all(Val::Px(1.0)),
        padding: UiRect::all(Val::Percent(0.6)),
        ..default()
    }
}

fn placeholder_background() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.07, 0.05, 0.02, 0.85))
}

fn placeholder_border_color() -> BorderColor {
    BorderColor {
        top: GOLD_DARK,
        right: GOLD_DARK,
        bottom: GOLD_DARK,
        left: GOLD_DARK,
    }
}

fn placeholder_label(label: &str) -> impl Bundle {
    (
        Text::new(label),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(PARCHMENT_DARK),
    )
}
