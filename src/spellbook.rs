use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use serde::Deserialize;

use crate::dictionary;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SpellEffect {
    Damage { amount: u32 },
    Stun { amount: f32 },
    Shield { amount: u32, duration: f32 },
    Buff { amount: i32, duration: f32 },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SpellDef {
    pub word: String,
    pub effects: Vec<SpellEffect>,
    pub futharkation: String,
}

impl SpellDef {
    pub fn as_futharkation(&self) -> dictionary::Futharkation {
        dictionary::Futharkation {
            word: self.word.clone(),
            letters: self.futharkation.clone(),
        }
    }
}

#[derive(Asset, TypePath, Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Book(pub Vec<SpellDef>);

impl Book {
    pub fn spells(&self) -> &[SpellDef] {
        &self.0
    }
}

#[derive(Default, TypePath)]
pub struct BookLoader;

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawSpellDef {
    word: String,
    effects: Vec<SpellEffect>,
    #[serde(default, alias = "letters", alias = "futhark", alias = "futharkation")]
    futharkation_spec: Option<String>,
}

impl AssetLoader for BookLoader {
    type Asset = Book;
    type Settings = ();
    type Error = BookError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Book, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let raw_spells: Vec<RawSpellDef> = serde_json::from_slice(&bytes)?;
        let mut spells = Vec::with_capacity(raw_spells.len());

        for raw in raw_spells {
            let mapped = dictionary::futharkation_from_word_with_override(
                &raw.word,
                raw.futharkation_spec.as_deref(),
            )
            .map_err(BookError::Futharkation)?;

            spells.push(SpellDef {
                word: raw.word,
                effects: raw.effects,
                futharkation: mapped.letters,
            });
        }

        Ok(Book(spells))
    }

    fn extensions(&self) -> &[&str] {
        &["book.json"]
    }
}

#[derive(Debug)]
pub enum BookError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Futharkation(String),
}

impl std::fmt::Display for BookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "io: {e}"),
            Self::Json(e) => write!(f, "json: {e}"),
            Self::Futharkation(e) => write!(f, "futharkation: {e}"),
        }
    }
}

impl std::error::Error for BookError {}

impl From<std::io::Error> for BookError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for BookError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

pub fn configure_book_asset(app: &mut App) {
    app.init_asset::<Book>().register_asset_loader(BookLoader);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_book_json() {
        let bytes = include_bytes!("../assets/spellbook.book.json");
        let raw: Vec<RawSpellDef> = serde_json::from_slice(bytes).expect("parses");
        assert!(!raw.is_empty());

        let spells: Vec<SpellDef> = raw
            .into_iter()
            .map(|raw| {
                let mapped = dictionary::futharkation_from_word_with_override(
                    &raw.word,
                    raw.futharkation_spec.as_deref(),
                )
                .expect("futharkation");

                SpellDef {
                    word: raw.word,
                    effects: raw.effects,
                    futharkation: mapped.letters,
                }
            })
            .collect();
        let book = Book(spells);
        assert!(!book.spells().is_empty());

        let icebolt = book
            .spells()
            .iter()
            .find(|s| s.word == "iceblast")
            .expect("iceblast");
        assert_eq!(icebolt.effects[0], SpellEffect::Damage { amount: 20 },);
        assert_eq!(icebolt.effects[1], SpellEffect::Stun { amount: 5.0 });
        assert!(!icebolt.futharkation.is_empty());
    }

    #[test]
    fn parses_shield_and_buff_effects() {
        let bytes = include_bytes!("../assets/spellbook.book.json");
        let raw: Vec<RawSpellDef> = serde_json::from_slice(bytes).expect("parses");
        let spells: Vec<SpellDef> = raw
            .into_iter()
            .map(|raw| {
                let mapped = dictionary::futharkation_from_word_with_override(
                    &raw.word,
                    raw.futharkation_spec.as_deref(),
                )
                .expect("futharkation");

                SpellDef {
                    word: raw.word,
                    effects: raw.effects,
                    futharkation: mapped.letters,
                }
            })
            .collect();
        let book = Book(spells);

        let shield = book.spells().iter().find(|s| s.word == "shield").unwrap();
        assert_eq!(
            shield.effects[0],
            SpellEffect::Shield {
                amount: 30,
                duration: 15.0
            },
        );

        let evoke = book.spells().iter().find(|s| s.word == "evoke").unwrap();
        assert_eq!(
            evoke.effects[0],
            SpellEffect::Buff {
                amount: 3,
                duration: 8.0
            },
        );
    }

    #[test]
    fn supports_explicit_futharkation_in_json() {
        let raw: Vec<RawSpellDef> = serde_json::from_str(
            r#"[
                {
                    "word": "custom",
                    "letters": "futar",
                    "effects": [{"type": "damage", "amount": 1}]
                }
            ]"#,
        )
        .expect("parses");

        let mapped = dictionary::futharkation_from_word_with_override(
            &raw[0].word,
            raw[0].futharkation_spec.as_deref(),
        )
        .expect("mapped");

        assert_eq!(mapped.letters, "futar");
    }
}
