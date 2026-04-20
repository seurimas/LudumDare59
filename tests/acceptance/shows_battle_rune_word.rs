use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::battle::{StartBattle, configure_battle},
    ui::hud_root::spawn_battle_hud_root,
};
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

const TEST_ID: u8 = 7;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    configure_battle(&mut app);

    app.add_systems(
        OnEnter(GameState::Adventure),
        spawn_futhark_keyboard.after(spawn_battle_hud_root),
    );
    app.add_systems(OnEnter(GameState::Adventure), start_random_battle_demo);

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Starts a random five-rune battle word. Type a five-rune guess and press Enter to score it, lift the row, and spawn a fresh row.",
    );

    app.run();
}

fn start_random_battle_demo(
    mut commands: Commands,
    mut start_battle: MessageWriter<StartBattle>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::rng())
        .expect("default dictionary should contain a five-rune futharkation");

    speed.hue_degrees_per_second = 45.0;
    start_battle.write(StartBattle(selected.clone()));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(48.0),
            top: Val::Px(40.0),
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Text::new(format!(
                    "StartBattle({}: {})",
                    selected.word, selected.letters
                )),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                Text::new(
                    "Type a five-rune guess, then press Enter. Correct runes go green, misplaced runes go yellow, missing runes go red. The scored row should rise before a fresh row appears below it.",
                ),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("Press F1 to pass or F2 to fail."),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}
