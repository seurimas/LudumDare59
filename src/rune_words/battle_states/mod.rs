pub mod acting;
pub mod binding;
pub mod reacting;

use bevy::prelude::*;

pub fn configure_battle_states(app: &mut App) {
    binding::configure_binding(app);
    acting::configure_acting(app);
    reacting::configure_reacting(app);
}
