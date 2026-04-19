use bevy::image::ImageSampler;
use bevy::prelude::*;

use crate::{GameAssets, GameState};

pub fn configure_samplers(app: &mut App) {
    app.add_systems(OnEnter(GameState::Adventure), set_nearest_samplers);
}

fn set_nearest_samplers(mut images: ResMut<Assets<Image>>, game_assets: Res<GameAssets>) {
    for handle in [
        &game_assets.backdrop,
        &game_assets.goblin,
        &game_assets.robed,
    ] {
        if let Some(img) = images.get_mut(handle) {
            img.sampler = ImageSampler::nearest();
        }
    }
}
