use bevy::prelude::*;

pub mod arena;
pub mod binding_panel;
pub mod book;
pub mod clock;
pub mod effects;
pub mod game_over;
pub mod health_bars;
pub mod hud_root;
pub mod inscribed;
pub mod keyboard;
pub mod main_menu;
pub mod palette;
pub mod samplers;
pub mod spell_selection;

pub fn configure_ui(app: &mut App) {
    clock::configure_clock(app);
    hud_root::configure_hud_root(app);
    arena::configure_arena(app);
    inscribed::configure_inscribed(app);
    health_bars::configure_health_bars(app);
    book::configure_book(app);
    binding_panel::configure_binding_panel(app);
    samplers::configure_samplers(app);
    effects::configure_effects(app);
    main_menu::configure_main_menu(app);
    game_over::configure_game_over(app);
    spell_selection::configure_spell_selection(app);
}
