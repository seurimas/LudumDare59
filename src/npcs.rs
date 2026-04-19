use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::Deserialize;

use crate::health::NpcAttackSpec;
use crate::rune_words::battle::NpcType;

#[derive(Asset, TypePath, Deserialize, Debug, Clone)]
pub struct NpcSpec {
    pub max_health: u32,
    pub npc_type: NpcType,
    pub attacks: Vec<NpcAttackSpec>,
}

#[derive(Default, TypePath)]
pub struct NpcSpecLoader;

impl AssetLoader for NpcSpecLoader {
    type Asset = NpcSpec;
    type Settings = ();
    type Error = NpcSpecError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<NpcSpec, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["npc.json"]
    }
}

#[derive(Debug)]
pub enum NpcSpecError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for NpcSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
        }
    }
}

impl std::error::Error for NpcSpecError {}

impl From<std::io::Error> for NpcSpecError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for NpcSpecError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub fn configure_npcs(app: &mut App) {
    app.init_asset::<NpcSpec>()
        .register_asset_loader(NpcSpecLoader);
}
