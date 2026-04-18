pub mod config;
pub mod params;
pub mod playback;
pub mod processing;

pub use config::{FutharkSoundConfig, pick_params};
pub use params::{ReverbParams, SoundParams};
pub use playback::{ProcessedAudio, samples_to_wav};
pub use processing::process_audio;

use bevy::prelude::*;

pub fn configure_audio(app: &mut App) {
    app.init_asset::<FutharkSoundConfig>()
        .register_asset_loader(config::FutharkSoundConfigLoader);
}
