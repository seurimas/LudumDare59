use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::battle::configure_battle,
    rune_words::battle_states::binding::StartBinding,
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
        OnEnter(GameState::Ready),
        spawn_futhark_keyboard.after(spawn_battle_hud_root),
    );
    app.add_systems(OnEnter(GameState::Ready), start_demo);
    app.add_systems(Update, reset_on_f3.run_if(in_state(GameState::Ready)));

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Starts a random five-rune binding battle. Type guesses and press Enter to score rows. Correct=green, misplaced=yellow, wrong=red. Each scored row rises and a fresh row appears. Press F3 to reset with a new word.",
    );

    app.run();
}

fn start_demo(
    mut commands: Commands,
    mut start_binding: MessageWriter<StartBinding>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::thread_rng())
        .expect("default dictionary should contain a five-rune futharkation");

    speed.hue_degrees_per_second = 45.0;
    start_binding.write(StartBinding(Some(selected.clone())));

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
                Text::new(format!("StartBinding({}: {})", selected.word, selected.letters)),
                TextFont { font_size: 24.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new(
                    "Binding state: guess an unknown word. Correct=green, misplaced=yellow, wrong=red. Scored row rises; fresh row spawns below.",
                ),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("F1 pass | F2 fail | F3 new word"),
                TextFont { font_size: 18.0, ..default() },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}

fn reset_on_f3(keys: Res<ButtonInput<KeyCode>>, mut start_binding: MessageWriter<StartBinding>) {
    if keys.just_pressed(KeyCode::F3) {
        let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::thread_rng())
            .expect("default dictionary should contain a five-rune futharkation");
        start_binding.write(StartBinding(Some(selected)));
    }
}
