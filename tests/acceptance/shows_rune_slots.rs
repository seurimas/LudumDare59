use LudumDare59::{
    GameAssets, GameState, acceptance, configure_app, configure_loading,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::rune_slots::{RuneSlotConfig, RuneSlotForegroundSet, spawn_rune_slot},
    ui::hud_root::spawn_battle_hud_root,
};
use bevy::prelude::*;

const TEST_ID: u8 = 5;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);

    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_futhark_keyboard.after(spawn_battle_hud_root),
    );
    app.add_systems(OnEnter(GameState::Adventure), spawn_demo_rune_slots);

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
