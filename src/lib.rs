use bevy::{input::common_conditions::input_toggle_active, prelude::*};
use bevy_aspect_ratio_mask::{AspectRatioPlugin, Resolution};
use bevy_asset_loader::prelude::*;
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use wasm_bindgen::prelude::*;

use crate::health::PlayerCombatState;

pub mod acceptance;
pub mod audio;
pub mod combat;
pub mod dictionary;
pub mod futhark;
pub mod health;
pub mod loading;
pub mod npcs;
pub mod rune_words;
pub mod spellbook;
pub mod tutorial;
pub mod ui;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Loading,
    Processing,
    MainMenu,
    Adventure,
    GameOver,
}

/// Tracks how many enemies the player has defeated across the entire run.
#[derive(Resource, Default)]
pub struct RunStats {
    pub enemies_defeated: u32,
    /// How many times each NPC type has been killed this run.
    pub kills_by_type: std::collections::HashMap<crate::rune_words::battle::NpcType, u32>,
}

#[derive(AssetCollection, Resource)]
pub struct GameAssets {
    #[asset(path = "images/futhark.png")]
    pub futhark: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 32, tile_size_y = 32, columns = 16, rows = 16))]
    pub futhark_layout: Handle<TextureAtlasLayout>,
    // Ordered by futhark::LETTERS: f u T a r k g w h n i j A p z s t b e m l N d o S
    #[asset(paths(
        "sound/f.ogg",
        "sound/u.ogg",
        "sound/T-2.ogg",
        "sound/a.ogg",
        "sound/r.ogg",
        "sound/k.ogg",
        "sound/g.ogg",
        "sound/w.ogg",
        "sound/h.ogg",
        "sound/n.ogg",
        "sound/i.ogg",
        "sound/j.ogg",
        "sound/A-2.ogg",
        "sound/p.ogg",
        "sound/z.ogg",
        "sound/s.ogg",
        "sound/t.ogg",
        "sound/b.ogg",
        "sound/e.ogg",
        "sound/m.ogg",
        "sound/l.ogg",
        "sound/N-2.ogg",
        "sound/d.ogg",
        "sound/o.ogg",
        "sound/S-2.ogg"
    ))]
    pub futhark_sounds: Vec<UntypedHandle>,
    #[asset(path = "sound/params.json")]
    pub futhark_sound_params: Handle<audio::FutharkSoundConfig>,
    #[asset(path = "sound/conversational_params.json")]
    pub futhark_conversational_params: Handle<audio::FutharkSoundConfig>,
    #[asset(path = "images/backdrop.png")]
    pub backdrop: Handle<Image>,
    #[asset(path = "images/parchment_tile.png")]
    pub parchment_tile: Handle<Image>,
    #[asset(path = "images/corner_bracket.png")]
    pub corner_bracket: Handle<Image>,
    #[asset(path = "images/vignette.png")]
    pub vignette: Handle<Image>,
    #[asset(path = "images/sigils.png")]
    pub sigils: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 32, tile_size_y = 32, columns = 4, rows = 1))]
    pub sigils_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "images/goblin.png")]
    pub goblin: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 2, rows = 2))]
    pub goblin_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "images/robed.png")]
    pub robed: Handle<Image>,
    #[asset(texture_atlas_layout(tile_size_x = 64, tile_size_y = 64, columns = 2, rows = 2))]
    pub robed_layout: Handle<TextureAtlasLayout>,
    #[asset(path = "fonts/CormorantUnicase-SemiBold.ttf")]
    pub font_cormorant_unicase_semibold: Handle<Font>,
    #[asset(path = "fonts/CormorantUnicase-Bold.ttf")]
    pub font_cormorant_unicase_bold: Handle<Font>,
    #[asset(path = "fonts/CormorantGaramond-Italic-VariableFont_wght.ttf")]
    pub font_cormorant_garamond_italic: Handle<Font>,
    #[asset(path = "fonts/IMFellDWPicaSC-Regular.ttf")]
    pub font_im_fell_sc: Handle<Font>,
    #[asset(path = "fonts/UnifrakturMaguntia-Regular.ttf")]
    pub font_unifraktur: Handle<Font>,
    #[asset(path = "npcs/goblin.npc.json")]
    pub goblin_spec: Handle<npcs::NpcSpec>,
    #[asset(path = "npcs/robed.npc.json")]
    pub robed_spec: Handle<npcs::NpcSpec>,
    #[asset(path = "spellbook.book.json")]
    pub spellbook: Handle<spellbook::Book>,
}

pub fn configure_app(app: &mut App) {
    app.insert_resource(ClearColor(ui::palette::NIGHT));
    app.add_plugins(AspectRatioPlugin {
        resolution: Resolution {
            width: 1280.0,
            height: 720.0,
        },
        ..default()
    })
    .add_plugins(EguiPlugin::default())
    .add_plugins(WorldInspectorPlugin::default().run_if(input_toggle_active(false, KeyCode::F7)));
    futhark::configure_futhark_keyboard(app);
    rune_words::rune_slots::configure_rune_slots(app);
    ui::configure_ui(app);
    combat::configure_combat(app);
    tutorial::configure_tutorial(app);

    app.init_resource::<PlayerCombatState>();
    app.init_resource::<RunStats>();

    app.add_systems(
        Update,
        (futhark::sync_keyboard_zoom,)
            .chain()
            .run_if(in_state(GameState::Adventure)),
    );
    app.add_systems(
        Update,
        (
            futhark::emit_futhark_keyboard_command_from_clicks,
            futhark::emit_typed_futhark_input_from_keyboard,
            futhark::emit_typed_futhark_input_from_keyboard_clicks,
            futhark::sync_futhark_key_hover,
            futhark::animate_futhark_keyboard_colors,
            futhark::sync_eliminated_futhark_keys,
            rune_words::rune_slots::activate_rune_slot_on_click,
            rune_words::rune_slots::update_active_rune_slot_from_typed_input,
            futhark::play_futhark_key_sound,
            rune_words::rune_slots::handle_backspace_in_rune_slots,
            rune_words::rune_slots::emit_play_active_rune_word_audio_on_enter,
            rune_words::rune_slots::play_active_rune_word_audio,
            rune_words::rune_slots::play_futhark_letters_audio,
            rune_words::rune_slots::sync_rune_slot_visuals,
        )
            .chain()
            .run_if(in_state(GameState::Adventure)),
    );
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, listen_for_fullscreen);

    app.add_systems(
        Update,
        rune_words::rune_slots::tick_word_audio_queue.run_if(in_state(GameState::Adventure)),
    );
}

pub fn configure_loading(app: &mut App) {
    audio::configure_audio(app);
    npcs::configure_npcs(app);
    spellbook::configure_book_asset(app);
    loading::configure_loading(app);
}

#[wasm_bindgen(start)]
pub fn wasm_main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Runic Ascendancy".into(),
            resolution: bevy::window::WindowResolution::new(1280_u32, 960_u32),
            canvas: Some("#bevy".into()),
            ..default()
        }),
        ..default()
    }));
    configure_app(&mut app);
    rune_words::battle::configure_battle(&mut app);
    configure_loading(&mut app);
    app.run();
}

#[cfg(target_arch = "wasm32")]
fn listen_for_fullscreen(mut key_input: Res<ButtonInput<KeyCode>>) {
    if key_input.just_pressed(KeyCode::F11) {
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.document_element())
            .and_then(|elem| elem.request_fullscreen().ok());
        web_sys::console::log_1(&"Toggled fullscreen".into());
    }
}
