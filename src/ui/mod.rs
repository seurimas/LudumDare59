use bevy::prelude::*;

pub mod arena;
pub mod clock;
pub mod health;
pub mod hud_root;
pub mod inscribed;
pub mod keyboard;
pub mod palette;
pub mod samplers;

pub fn configure_ui(app: &mut App) {
    clock::configure_clock(app);
    health::configure_health(app);
    hud_root::configure_hud_root(app);
    arena::configure_arena(app);
    inscribed::configure_inscribed(app);
    samplers::configure_samplers(app);
}
