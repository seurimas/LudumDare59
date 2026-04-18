use LudumDare59::{
    GameAssets, GameState, acceptance, configure_app, configure_loading,
    futhark::{self, FutharkKeyboardAnimationSpeed},
    rune_slots::{
        self, ActiveRuneSlot, RuneSlotConfig, RuneSlotForegroundSet, activate_rune_slot_on_click,
        handle_backspace_in_rune_slots, spawn_rune_word, sync_rune_slot_visuals,
        update_active_rune_slot_from_typed_input,
    },
};
use bevy::prelude::*;

const TEST_ID: u8 = 6;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    futhark::configure_futhark_keyboard(&mut app);
    rune_slots::configure_rune_slots(&mut app);

    app.add_systems(OnEnter(GameState::Ready), futhark::spawn_futhark_keyboard);
    app.add_systems(OnEnter(GameState::Ready), spawn_word_demo);
    app.add_systems(
        Update,
        (
            futhark::toggle_futhark_keyboard_legend_mode,
            futhark::sync_futhark_keyboard_labels,
            futhark::emit_typed_futhark_input_from_keyboard,
            futhark::emit_typed_futhark_input_from_keyboard_clicks,
            futhark::sync_futhark_key_hover,
            futhark::animate_futhark_keyboard_colors,
            futhark::play_futhark_key_sound,
            activate_rune_slot_on_click,
            update_active_rune_slot_from_typed_input,
            handle_backspace_in_rune_slots,
            sync_rune_slot_visuals,
        )
            .chain()
            .run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Type futhark letters to fill the word slots left-to-right. Backspace clears the previous slot.",
    );

    app.run();
}

fn spawn_word_demo(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    speed.hue_degrees_per_second = 45.0;

    let configs = vec![
        RuneSlotConfig {
            left: Val::Px(48.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.4, 0.95),
            foreground_set: RuneSlotForegroundSet::Primary,
            ..default()
        },
        RuneSlotConfig {
            left: Val::Px(116.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.4, 0.95),
            foreground_set: RuneSlotForegroundSet::Primary,
            ..default()
        },
        RuneSlotConfig {
            left: Val::Px(184.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.4, 0.95),
            foreground_set: RuneSlotForegroundSet::Primary,
            ..default()
        },
        RuneSlotConfig {
            left: Val::Px(252.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.4, 0.95),
            foreground_set: RuneSlotForegroundSet::Primary,
            ..default()
        },
    ];

    let slots = spawn_rune_word(&mut commands, &game_assets, configs);

    // Start with the first slot active
    active_slot.entity = Some(slots[0]);

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(48.0),
            top: Val::Px(110.0),
            ..default()
        },
        Text::new("Type futhark letters to fill the word. Backspace clears the previous slot."),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}
