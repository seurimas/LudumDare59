use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::battle::configure_battle,
    rune_words::battle_states::acting::StartActing,
};
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

const TEST_ID: u8 = 8;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    configure_battle(&mut app);

    app.add_systems(OnEnter(GameState::Ready), spawn_futhark_keyboard);
    app.add_systems(OnEnter(GameState::Ready), start_demo);

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Acting battle state: enter three specific words of varying lengths. Each scored row shows correct/misplaced/wrong counts and rises. After all targets, the phase ends.",
    );

    app.run();
}

fn start_demo(
    mut commands: Commands,
    mut start_acting: MessageWriter<StartActing>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    let mut rng = rand::thread_rng();
    let targets: Vec<_> = [3, 4, 5]
        .iter()
        .filter_map(|&len| dictionary::random_futharkation_with_rune_length(len, &mut rng).ok())
        .collect();

    let label = targets
        .iter()
        .map(|t| format!("{} ({})", t.word, t.letters))
        .collect::<Vec<_>>()
        .join(" → ");

    speed.hue_degrees_per_second = 45.0;
    start_acting.write(StartActing { targets });

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
                Text::new(format!("StartActing: {}", label)),
                TextFont { font_size: 22.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new(
                    "Acting state: enter each target word. Each scored row shows ✓~/✗ counts and rises. After all words, the phase ends.",
                ),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("Press F1 to pass or F2 to fail."),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}
