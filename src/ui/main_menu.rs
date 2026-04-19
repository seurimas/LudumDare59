use bevy::prelude::*;
use bevy_aspect_ratio_mask::Hud;

use crate::GameState;
use crate::tutorial::TutorialState;
use crate::ui::palette::{GOLD, GOLD_DARK, INK, NIGHT, PARCHMENT, PARCHMENT_WARM};

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct TutorialButton;

pub fn configure_main_menu(app: &mut App) {
    app.add_systems(OnEnter(GameState::MainMenu), spawn_main_menu);
    app.add_systems(OnExit(GameState::MainMenu), despawn_main_menu);
    app.add_systems(
        Update,
        (handle_start_button, handle_tutorial_button).run_if(in_state(GameState::MainMenu)),
    );
}

fn spawn_main_menu(mut commands: Commands, hud: Res<Hud>, game_assets: Res<crate::GameAssets>) {
    commands.entity(hud.0).with_children(|hud_root| {
        hud_root
            .spawn((
                MainMenuRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(32.0),
                    ..default()
                },
                BackgroundColor(NIGHT),
            ))
            .with_children(|menu| {
                // Title
                menu.spawn((
                    Text::new("Runic Ascendancy"),
                    TextFont {
                        font: game_assets.font_unifraktur.clone(),
                        font_size: 72.0,
                        ..default()
                    },
                    TextColor(GOLD),
                ));

                // Subtitle
                menu.spawn((
                    Text::new("Master the elder futhark. Bind your foes."),
                    TextFont {
                        font: game_assets.font_cormorant_garamond_italic.clone(),
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(PARCHMENT_WARM),
                ));

                // Spacer
                menu.spawn(Node {
                    height: Val::Px(48.0),
                    ..default()
                });

                // Start button
                menu.spawn((
                    StartButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(48.0), Val::Px(16.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(INK),
                    BorderColor::from(GOLD_DARK),
                    children![(
                        Text::new("Begin Adventure"),
                        TextFont {
                            font: game_assets.font_cormorant_unicase_semibold.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(PARCHMENT),
                    )],
                ));

                // Tutorial button
                menu.spawn((
                    TutorialButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(48.0), Val::Px(16.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        margin: UiRect::top(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(INK),
                    BorderColor::from(GOLD_DARK),
                    children![(
                        Text::new("Tutorial"),
                        TextFont {
                            font: game_assets.font_cormorant_unicase_semibold.clone(),
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(PARCHMENT),
                    )],
                ));
            });
    });
}

fn despawn_main_menu(mut commands: Commands, roots: Query<Entity, With<MainMenuRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn handle_start_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<StartButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::Adventure);
        }
    }
}

fn handle_tutorial_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<TutorialButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut tutorial: ResMut<TutorialState>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            *tutorial = TutorialState::start();
            next_state.set(GameState::Adventure);
        }
    }
}
