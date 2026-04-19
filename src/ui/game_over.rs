use bevy::prelude::*;
use bevy_aspect_ratio_mask::Hud;

use crate::ui::palette::{BLOOD, GOLD_DARK, INK, NIGHT, PARCHMENT, PARCHMENT_WARM};
use crate::{GameAssets, GameState, RunStats};

#[derive(Component)]
struct GameOverRoot;

#[derive(Component)]
struct RestartButton;

pub fn configure_game_over(app: &mut App) {
    app.add_systems(OnEnter(GameState::GameOver), spawn_game_over);
    app.add_systems(OnExit(GameState::GameOver), despawn_game_over);
    app.add_systems(
        Update,
        handle_restart_button.run_if(in_state(GameState::GameOver)),
    );
}

fn spawn_game_over(
    mut commands: Commands,
    hud: Res<Hud>,
    game_assets: Res<GameAssets>,
    run_stats: Res<RunStats>,
) {
    let defeated_text = format!("Foes vanquished: {}", run_stats.enemies_defeated);

    commands.entity(hud.0).with_children(|hud_root| {
        hud_root
            .spawn((
                GameOverRoot,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(24.0),
                    ..default()
                },
                BackgroundColor(NIGHT),
            ))
            .with_children(|menu| {
                // Title
                menu.spawn((
                    Text::new("Thy Journey Ends"),
                    TextFont {
                        font: game_assets.font_unifraktur.clone(),
                        font_size: 56.0,
                        ..default()
                    },
                    TextColor(BLOOD),
                ));

                // Stats
                menu.spawn((
                    Text::new(defeated_text),
                    TextFont {
                        font: game_assets.font_cormorant_unicase_semibold.clone(),
                        font_size: 28.0,
                        ..default()
                    },
                    TextColor(PARCHMENT_WARM),
                ));

                // Spacer
                menu.spawn(Node {
                    height: Val::Px(48.0),
                    ..default()
                });

                // Restart button
                menu.spawn((
                    RestartButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(48.0), Val::Px(16.0)),
                        border: UiRect::all(Val::Px(2.0)),
                        ..default()
                    },
                    BackgroundColor(INK),
                    BorderColor::from(GOLD_DARK),
                    children![(
                        Text::new("Rise Again"),
                        TextFont {
                            font: game_assets.font_cormorant_unicase_semibold.clone(),
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(PARCHMENT),
                    )],
                ));
            });
    });
}

fn despawn_game_over(mut commands: Commands, roots: Query<Entity, With<GameOverRoot>>) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
}

fn handle_restart_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<RestartButton>)>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
        }
    }
}
