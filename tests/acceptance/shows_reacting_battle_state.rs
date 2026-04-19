use LudumDare59::{
    GameState, acceptance, configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    rune_words::battle::configure_battle,
    rune_words::battle_states::reacting::{ReactingFailed, ReactingSucceeded, StartReacting},
    ui::hud_root::spawn_battle_hud_root,
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

const TEST_ID: u8 = 9;

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
    app.add_systems(
        Update,
        (show_outcome, reset_on_f3).run_if(in_state(GameState::Ready)),
    );

    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Reacting battle state: enter the correct 5-rune word within 10 seconds. Timer counts down. Correct entry = success; timeout = failure. Press F3 to reset with a new word.",
    );

    app.run();
}

fn start_demo(
    mut commands: Commands,
    mut start_reacting: MessageWriter<StartReacting>,
    mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
) {
    let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::thread_rng())
        .expect("default dictionary should contain a five-rune futharkation");

    speed.hue_degrees_per_second = 45.0;
    start_reacting.write(StartReacting {
        target: selected.clone(),
        time_limit: 10.0,
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
                Text::new(format!("Target: {} ({})", selected.word, selected.letters)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("Reacting state: enter the correct word before the timer runs out.",),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
            (
                Text::new("F1 pass | F2 fail | F3 new word"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ),
        ],
    ));
}

#[derive(Component)]
struct OutcomeLabel;

fn show_outcome(
    mut commands: Commands,
    mut succeeded: MessageReader<ReactingSucceeded>,
    mut failed: MessageReader<ReactingFailed>,
    existing: Query<Entity, With<OutcomeLabel>>,
) {
    let outcome = if !succeeded.is_empty() {
        succeeded.clear();
        Some(("SUCCESS", Color::srgb(0.24, 0.68, 0.32)))
    } else if !failed.is_empty() {
        failed.clear();
        Some(("FAILED", Color::srgb(0.78, 0.2, 0.2)))
    } else {
        return;
    };

    let (text, color) = outcome.unwrap();

    for entity in &existing {
        commands.entity(entity).despawn();
    }

    commands.spawn((
        OutcomeLabel,
        Text::new(text),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(color),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(180.0),
            top: Val::Px(320.0),
            ..default()
        },
    ));
}

fn reset_on_f3(keys: Res<ButtonInput<KeyCode>>, mut start_reacting: MessageWriter<StartReacting>) {
    if keys.just_pressed(KeyCode::F3) {
        let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::thread_rng())
            .expect("default dictionary should contain a five-rune futharkation");
        start_reacting.write(StartReacting {
            target: selected,
            time_limit: 10.0,
        });
    }
}
