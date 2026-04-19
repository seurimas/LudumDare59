use bevy::prelude::*;

pub mod arena;
pub mod book;
pub mod clock;
pub mod health;
pub mod health_bars;
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
    health_bars::configure_health_bars(app);
    book::configure_book(app);
    samplers::configure_samplers(app);
}
