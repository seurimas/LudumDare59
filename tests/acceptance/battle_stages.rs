use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::battle::configure_battle,
    rune_words::battle_stages::{FinalChallenge, QuickTime, WordBook, configure_battle_stages},
    rune_words::battle_states::{acting::StartActing, configure_battle_states},
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

const TEST_ID: u8 = 10;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    configure_battle(&mut app);
    configure_battle_states(&mut app);
    configure_battle_stages(&mut app);

    app.add_systems(OnEnter(GameState::Ready), spawn_futhark_keyboard);
    app.add_systems(OnEnter(GameState::Ready), setup_demo);
    app.add_systems(
        Update,
        demo_controller.run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Battle stages: acting from word book; QuickTime (reacting) at 20s; FinalChallenge on first acting success after QuickTime; then binding.",
    );

    app.run();
}

#[derive(Resource)]
struct DemoState {
    elapsed: f32,
    quicktime_word: dictionary::Futharkation,
    quicktime_sent: bool,
    final_sent: bool,
}

fn setup_demo(
    mut commands: Commands,
    mut start_acting: MessageWriter<StartActing>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    let mut rng = rand::thread_rng();

    let words: Vec<dictionary::Futharkation> = [3usize, 4, 5, 3, 4]
        .iter()
        .filter_map(|&len| {
            dictionary::random_futharkation_with_rune_length(len, &mut rng).ok()
        })
        .collect();

    let quicktime_word =
        dictionary::random_futharkation_with_rune_length(4, &mut rng)
            .unwrap_or_else(|_| words[0].clone());

    speed.hue_degrees_per_second = 45.0;

    let book_label = words
        .iter()
        .map(|w| format!("{} ({})", w.word, w.letters))
        .collect::<Vec<_>>()
        .join(", ");

    commands.insert_resource(WordBook {
        words: words.clone(),
    });
    commands.insert_resource(DemoState {
        elapsed: 0.0,
        quicktime_word: quicktime_word.clone(),
        quicktime_sent: false,
        final_sent: false,
    });

    start_acting.write(StartActing {
        targets: words,
    });

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
                Text::new(format!("Book: {}", book_label)),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new(format!(
                    "QuickTime word at 20s: {} ({})",
                    quicktime_word.word, quicktime_word.letters
                )),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("Flow: acting → QuickTime at 20s → reacting → acting → FinalChallenge on first success → binding"),
                TextFont { font_size: 16.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("F1 = pass, F2 = fail"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}

fn demo_controller(
    time: Res<Time>,
    mut state: ResMut<DemoState>,
    mut quicktime: MessageWriter<QuickTime>,
    mut final_challenge: MessageWriter<FinalChallenge>,
    mut acting_succeeded: MessageReader<LudumDare59::rune_words::battle_states::acting::ActingSucceeded>,
) {
    state.elapsed += time.delta_secs();

    if !state.quicktime_sent && state.elapsed >= 20.0 {
        quicktime.write(QuickTime(state.quicktime_word.clone()));
        state.quicktime_sent = true;
    }

    if state.quicktime_sent && !state.final_sent {
        if let Some(ev) = acting_succeeded.read().last().cloned() {
            final_challenge.write(FinalChallenge(ev.matched));
            state.final_sent = true;
        } else {
            acting_succeeded.clear();
        }
    } else {
        acting_succeeded.clear();
    }
}
