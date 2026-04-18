use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub mod acceptance;
pub mod futhark;
pub mod loading;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    RuneReveal,
    Ready,
}

#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(path = "images/futhark.png")]
    pub futhark: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 32, tile_size_y = 32, columns = 16, rows = 16))]
    pub futhark_layout: Handle<TextureAtlasLayout>,
}

pub fn configure_app(app: &mut App) {
    app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 1.0)));
}

pub fn configure_loading(app: &mut App) {
    loading::configure_loading(app);
}
