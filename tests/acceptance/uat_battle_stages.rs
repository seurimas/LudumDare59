use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    health::PlayerCombatState,
    rune_words::{
        battle::{BattleState, NpcType, configure_battle},
        battle_states::{
            acting::{ActingSucceeded, StartActing},
            binding::{BindingSucceeded, StartBinding},
        },
    },
    spellbook::SpellDef,
    ui::hud_root::spawn_battle_hud_root,
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

    app.init_resource::<StageFlow>();
    app.add_message::<QuickTime>();

    app.add_systems(
        OnEnter(GameState::Ready),
        spawn_futhark_keyboard.after(spawn_battle_hud_root),
    );
    app.add_systems(OnEnter(GameState::Ready), setup_demo);
    app.add_systems(Update, demo_controller.run_if(in_state(GameState::Ready)));
    app.add_systems(
        Update,
        (on_quicktime, on_acting_succeeded, on_binding_succeeded)
            .chain()
            .run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Battle stages UAT: acting loops by default. After QuickTime (20s), next acting success transitions to binding.",
    );

    app.run();
}

#[derive(Resource, Default)]
struct StageFlow {
    binding_unlocked_by_quicktime: bool,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
struct QuickTime;

#[derive(Resource)]
struct DemoState {
    elapsed: f32,
    quicktime_sent: bool,
}

fn setup_demo(
    mut commands: Commands,
    mut start_acting: MessageWriter<StartActing>,
    mut player: ResMut<PlayerCombatState>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
    mut battle_state: ResMut<BattleState>,
) {
    battle_state.npc_type = Some(NpcType::Goblin);
    let mut rng = rand::thread_rng();

    let words: Vec<dictionary::Futharkation> = [3usize, 4, 5, 3, 4]
        .iter()
        .filter_map(|&len| dictionary::random_futharkation_with_rune_length(len, &mut rng).ok())
        .collect();

    speed.hue_degrees_per_second = 45.0;
    set_player_hand_from_words(&mut player, &words);

    let book_label = words
        .iter()
        .map(|w| format!("{} ({})", w.word, w.letters))
        .collect::<Vec<_>>()
        .join(", ");

    commands.insert_resource(DemoState {
        elapsed: 0.0,
        quicktime_sent: false,
    });

    start_acting.write(StartActing);

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
                Text::new("Flow: acting loops. QuickTime at 20s unlocks binding. Next acting success -> binding."),
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
        quicktime.write(QuickTime);
        state.quicktime_sent = true;
    }
}

fn on_quicktime(mut quicktime: MessageReader<QuickTime>, mut flow: ResMut<StageFlow>) {
    if quicktime.read().count() > 0 {
        flow.binding_unlocked_by_quicktime = true;
    }
}

fn on_acting_succeeded(
    mut succeeded: MessageReader<ActingSucceeded>,
    mut flow: ResMut<StageFlow>,
    mut start_binding: MessageWriter<StartBinding>,
    mut start_acting: MessageWriter<StartActing>,
) {
    let Some(_matched) = succeeded.read().last().map(|ev| ev.matched.clone()) else {
        return;
    };

    if flow.binding_unlocked_by_quicktime {
        flow.binding_unlocked_by_quicktime = false;
        start_binding.write(StartBinding(Some(
            dictionary::random_futharkation_with_rune_length(5, &mut thread_rng()).unwrap(),
        )));
        return;
    }

    start_acting.write(StartActing);
}

fn on_binding_succeeded(
    mut succeeded: MessageReader<BindingSucceeded>,
    mut start_acting: MessageWriter<StartActing>,
) {
    if !succeeded.is_empty() {
        succeeded.clear();
        start_acting.write(StartActing);
    }
}

fn set_player_hand_from_words(player: &mut PlayerCombatState, words: &[dictionary::Futharkation]) {
    player.hand = words
        .iter()
        .map(|word| SpellDef {
            word: word.word.clone(),
            effects: Vec::new(),
            futharkation: word.letters.clone(),
        })
        .collect();
}
