use LudumDare59::{
    GameAssets, GameState, acceptance, configure_app, configure_loading,
    futhark::{self, FutharkKeyboardAnimationSpeed},
    rune_slots::{
        self, RuneSlotConfig, RuneSlotForegroundSet, activate_rune_slot_on_click, spawn_rune_slot,
        sync_rune_slot_visuals, update_active_rune_slot_from_typed_input,
    },
};
use bevy::prelude::*;

const TEST_ID: u8 = 5;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    futhark::configure_futhark_keyboard(&mut app);
    rune_slots::configure_rune_slots(&mut app);

    app.add_systems(OnEnter(GameState::Ready), futhark::spawn_futhark_keyboard);
    app.add_systems(OnEnter(GameState::Ready), spawn_demo_rune_slots);
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
            sync_rune_slot_visuals,
        )
            .chain()
            .run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Click a rune slot, then type from keyboard/mapped keys to update that active slot",
    );

    app.run();
}

fn spawn_demo_rune_slots(
    mut commands: Commands,
    game_assets: Res<GameAssets>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    speed.hue_degrees_per_second = 45.0;

    spawn_rune_slot(
        &mut commands,
        &game_assets,
        RuneSlotConfig {
            left: Val::Px(48.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.4, 0.95),
            foreground_set: RuneSlotForegroundSet::Primary,
            initial_rune: Some('f'),
            ..default()
        },
    );

    spawn_rune_slot(
        &mut commands,
        &game_assets,
        RuneSlotConfig {
            left: Val::Px(116.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.2, 0.75, 0.35),
            foreground_set: RuneSlotForegroundSet::Alternate { page: 0 },
            initial_rune: Some('u'),
            ..default()
        },
    );

    spawn_rune_slot(
        &mut commands,
        &game_assets,
        RuneSlotConfig {
            left: Val::Px(184.0),
            top: Val::Px(48.0),
            background_color: Color::srgb(0.95, 0.45, 0.2),
            foreground_set: RuneSlotForegroundSet::Alternate { page: 1 },
            initial_rune: None,
            ..default()
        },
    );

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(48.0),
            top: Val::Px(110.0),
            ..default()
        },
        Text::new("Click a slot to activate it, then type a futhark key to change its rune."),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
    ));
}
