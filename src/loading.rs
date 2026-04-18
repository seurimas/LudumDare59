use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

use crate::GameAssets;
use crate::GameState;

#[derive(Component)]
struct LoadingScreen;

pub fn configure_loading(app: &mut App) {
    app.init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::Ready)
                .load_collection::<GameAssets>(),
        )
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(OnExit(GameState::Loading), despawn_loading_screen);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadingScreen,
        ))
        .with_child((
            Text::new("Loading..."),
            TextFont {
                font_size: 50.0,
                ..default()
            },
        ));
}

fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}
