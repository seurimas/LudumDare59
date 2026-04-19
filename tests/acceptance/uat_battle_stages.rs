use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::{
        battle::{BattleState, NpcType, configure_battle},
        battle_states::{
            acting::{ActingSucceeded, StartActing},
            binding::{BindingSucceeded, StartBinding},
            reacting::{ReactingFailed, ReactingSucceeded, StartReacting},
        },
    },
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use rand::thread_rng;

const TEST_ID: u8 = 10;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    configure_battle(&mut app);

    app.init_resource::<WordBook>();
    app.init_resource::<StageFlow>();
    app.add_message::<QuickTime>();

    app.add_systems(OnEnter(GameState::Ready), spawn_futhark_keyboard);
    app.add_systems(OnEnter(GameState::Ready), setup_demo);
    app.add_systems(Update, demo_controller.run_if(in_state(GameState::Ready)));
    app.add_systems(
        Update,
        (
            on_quicktime,
            on_reacting_resolved,
            on_acting_succeeded,
            on_binding_succeeded,
        )
            .chain()
            .run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Battle stages UAT: acting loops by default. After QuickTime (reacting), the next acting success transitions to binding.",
    );

    app.run();
}

#[derive(Resource, Default)]
struct WordBook {
    words: Vec<dictionary::Futharkation>,
}

#[derive(Resource, Default)]
struct StageFlow {
    binding_unlocked_by_quicktime: bool,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
struct QuickTime(dictionary::Futharkation);

#[derive(Resource)]
struct DemoState {
    elapsed: f32,
    quicktime_word: dictionary::Futharkation,
    quicktime_sent: bool,
}

fn setup_demo(
    mut commands: Commands,
    mut start_acting: MessageWriter<StartActing>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
    mut book: ResMut<WordBook>,
    mut battle_state: ResMut<BattleState>,
) {
    battle_state.npc_type = Some(NpcType::Goblin);
    let mut rng = rand::thread_rng();

    let words: Vec<dictionary::Futharkation> = [3usize, 4, 5, 3, 4]
        .iter()
        .filter_map(|&len| dictionary::random_futharkation_with_rune_length(len, &mut rng).ok())
        .collect();

    let quicktime_word = dictionary::random_futharkation_with_rune_length(4, &mut rng)
        .unwrap_or_else(|_| words[0].clone());

    speed.hue_degrees_per_second = 45.0;
    book.words = words.clone();

    let book_label = words
        .iter()
        .map(|w| format!("{} ({})", w.word, w.letters))
        .collect::<Vec<_>>()
        .join(", ");

    commands.insert_resource(DemoState {
        elapsed: 0.0,
        quicktime_word: quicktime_word.clone(),
        quicktime_sent: false,
    });

    start_acting.write(StartActing { targets: words });

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
                Text::new(
                    "Flow: acting loops. QuickTime at 20s -> reacting. After that, next acting success -> binding.",
                ),
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
) {
    state.elapsed += time.delta_secs();

    if !state.quicktime_sent && state.elapsed >= 20.0 {
        quicktime.write(QuickTime(state.quicktime_word.clone()));
        state.quicktime_sent = true;
    }
}

fn on_quicktime(
    mut quicktime: MessageReader<QuickTime>,
    mut flow: ResMut<StageFlow>,
    mut start_reacting: MessageWriter<StartReacting>,
) {
    for QuickTime(word) in quicktime.read() {
        flow.binding_unlocked_by_quicktime = true;
        start_reacting.write(StartReacting {
            target: word.clone(),
            time_limit: 10.0,
        });
    }
}

fn on_reacting_resolved(
    mut succeeded: MessageReader<ReactingSucceeded>,
    mut failed: MessageReader<ReactingFailed>,
    mut start_acting: MessageWriter<StartActing>,
    book: Res<WordBook>,
) {
    let any = !succeeded.is_empty() || !failed.is_empty();
    succeeded.clear();
    failed.clear();
    if any && !book.words.is_empty() {
        start_acting.write(StartActing {
            targets: book.words.clone(),
        });
    }
}

fn on_acting_succeeded(
    mut succeeded: MessageReader<ActingSucceeded>,
    mut flow: ResMut<StageFlow>,
    mut start_binding: MessageWriter<StartBinding>,
    mut start_acting: MessageWriter<StartActing>,
    book: Res<WordBook>,
) {
    let Some(matched) = succeeded.read().last().map(|ev| ev.matched.clone()) else {
        return;
    };

    if flow.binding_unlocked_by_quicktime {
        flow.binding_unlocked_by_quicktime = false;
        start_binding.write(StartBinding(
            dictionary::random_futharkation_with_rune_length(5, &mut thread_rng()).unwrap(),
        ));
        return;
    }

    if !book.words.is_empty() {
        start_acting.write(StartActing {
            targets: book.words.clone(),
        });
    }
}

fn on_binding_succeeded(
    mut succeeded: MessageReader<BindingSucceeded>,
    mut start_acting: MessageWriter<StartActing>,
    book: Res<WordBook>,
) {
    if !succeeded.is_empty() {
        succeeded.clear();
        if !book.words.is_empty() {
            start_acting.write(StartActing {
                targets: book.words.clone(),
            });
        }
    }
}
