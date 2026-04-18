use bevy::prelude::*;
use bevy_asset_loader::prelude::*;

pub mod acceptance;
pub mod audio_params;
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
    // Ordered by futhark::LETTERS: f u 7 a r k g w h n i j A p z s t b e m l N d o
    #[asset(paths(
        "sound/f.ogg",
        "sound/u.ogg",
        "sound/7.ogg",
        "sound/a.ogg",
        "sound/r.ogg",
        "sound/k.ogg",
        "sound/g.ogg",
        "sound/w.ogg",
        "sound/h.ogg",
        "sound/n.ogg",
        "sound/i.ogg",
        "sound/j.ogg",
        "sound/A-2.ogg",
        "sound/p.ogg",
        "sound/z.ogg",
        "sound/s.ogg",
        "sound/t.ogg",
        "sound/b.ogg",
        "sound/e.ogg",
        "sound/m.ogg",
        "sound/l.ogg",
        "sound/N-2.ogg",
        "sound/d.ogg",
        "sound/o.ogg"
    ))]
    pub futhark_sounds: Vec<UntypedHandle>,
    #[asset(path = "sound/params.json")]
    pub futhark_sound_params: Handle<audio_params::FutharkSoundConfig>,
}

pub fn configure_app(app: &mut App) {
    app.insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 1.0)));
}

pub fn configure_loading(app: &mut App) {
    audio_params::configure_audio_params(app);
    loading::configure_loading(app);
}
