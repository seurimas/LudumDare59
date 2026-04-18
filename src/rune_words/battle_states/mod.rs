pub mod acting;
pub mod binding;
pub mod reacting;

use bevy::prelude::*;

use crate::dictionary::Futharkation;

#[derive(Resource, Default)]
pub struct WordBook {
    pub words: Vec<Futharkation>,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct QuickTime(pub Futharkation);

pub fn configure_battle_stages(app: &mut App) {
    app.init_resource::<WordBook>();
    app.add_message::<QuickTime>();
}

pub fn configure_battle_states(app: &mut App) {
    binding::configure_binding(app);
    acting::configure_acting(app);
    reacting::configure_reacting(app);
}
