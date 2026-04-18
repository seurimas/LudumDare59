use LudumDare59::{GameAssets, GameState, acceptance, configure_app, configure_loading};
use bevy::prelude::*;

const TEST_ID: u8 = 2;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    app.add_systems(OnEnter(GameState::Ready), spawn_futhark_rune);
    acceptance::initialize_app(&mut app, TEST_ID.into(), "Displays a single futhark rune");
    app.run();
}

fn spawn_futhark_rune(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn(Sprite {
        image: game_assets.futhark.clone(),
        texture_atlas: Some(TextureAtlas {
            layout: game_assets.futhark_layout.clone(),
            index: 0,
        }),
        ..default()
    });
}
