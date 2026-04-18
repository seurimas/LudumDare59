use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use rand::Rng;
use serde::Deserialize;

use super::params::SoundParams;

/// JSON asset: a 25-element array (one per futhark letter in LETTERS order),
/// each element an array of one or more SoundParams variants.
/// When a key is pressed, one variant is chosen at random.
#[derive(Asset, TypePath, Deserialize)]
pub struct FutharkSoundConfig(pub Vec<Vec<SoundParams>>);

#[derive(Default, TypePath)]
pub struct FutharkSoundConfigLoader;

impl AssetLoader for FutharkSoundConfigLoader {
    type Asset = FutharkSoundConfig;
    type Settings = ();
    type Error = FutharkSoundConfigError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<FutharkSoundConfig, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

#[derive(Debug)]
pub enum FutharkSoundConfigError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for FutharkSoundConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for FutharkSoundConfigError {}

impl From<std::io::Error> for FutharkSoundConfigError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for FutharkSoundConfigError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// Pick a random variant from the config for the given rune index and resolve
/// the reverb selection. Returns a fully-resolved `SoundParams`.
pub fn pick_params(config: Option<&FutharkSoundConfig>, index: usize) -> SoundParams {
    let variants = config
        .and_then(|c| c.0.get(index))
        .filter(|v| !v.is_empty());

    match variants {
        None => SoundParams::default(),
        Some(v) if v.len() == 1 => v[0].clone(),
        Some(v) => v[rand::thread_rng().gen_range(0..v.len())].clone(),
    }
}
