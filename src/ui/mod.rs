use bevy::prelude::*;

pub mod clock;
pub mod health;
pub mod palette;
pub mod samplers;

pub fn configure_ui(app: &mut App) {
    clock::configure_clock(app);
    health::configure_health(app);
    samplers::configure_samplers(app);
}
