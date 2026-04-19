pub mod acting;
pub mod binding;

use bevy::prelude::*;

use crate::dictionary::Futharkation;

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct QuickTime(pub Futharkation);

/// Stores the word associated with the most recently resolved row.
/// Set by acting/binding/reacting systems; read by `inscribed.rs` ledger.
#[derive(Resource, Default)]
pub struct LastGradedWord {
    pub word: Option<String>,
}

pub fn configure_battle_states(app: &mut App) {
    app.init_resource::<LastGradedWord>();
    binding::configure_binding(app);
    acting::configure_acting(app);
}
