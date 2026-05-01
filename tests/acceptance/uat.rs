#![allow(unused_imports)]

use LudumDare59::{
    GameAssets, GameState, acceptance,
    acceptance::AcceptanceTest,
    combat::BattleStart,
    configure_app, configure_loading, dictionary,
    futhark::{self, FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    health::{NpcCombatState, PlayerCombatState},
    npcs::NpcSpec,
    rune_words::{
        battle::{BattlePhase, BattleState, NpcType, configure_battle},
        battle_states::{
            acting::{ActingSucceeded, StartActing},
            binding::{BindingFailed, BindingSucceeded, StartBinding},
        },
        rune_slots::{
            ActiveRuneSlot, RuneSlotConfig, RuneSlotForegroundSet, spawn_rune_slot, spawn_rune_word,
        },
    },
    spellbook::{LearnedSpells, SpellDef},
    ui::{arena::NpcSprite, hud_root::spawn_battle_hud_root, spell_selection::SpellSelection},
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let test_name = match args.next() {
        Some(name) => name,
        None => {
            eprintln!("Usage: uat <test_name> [args...]");
            eprintln!("Available tests:");
            eprintln!("  shows_window");
            eprintln!("  shows_futhark_rune");
            eprintln!("  shows_loading_rune_reveal");
            eprintln!("  shows_typed_futhark_rune");
            eprintln!("  shows_rune_slots");
            eprintln!("  shows_rune_word_navigation");
            eprintln!("  shows_binding_battle_state");
            eprintln!("  shows_acting_battle_state");
            eprintln!("  battle_stages");
            eprintln!("  battle_against_npc");
            eprintln!("  shows_spell_selection");
            std::process::exit(1);
        }
    };
    let extra_args: Vec<String> = args.collect();

    match test_name.as_str() {
        "shows_window" => shows_window::run(&extra_args),
        "shows_futhark_rune" => shows_futhark_rune::run(&extra_args),
        "shows_loading_rune_reveal" => shows_loading_rune_reveal::run(&extra_args),
        "shows_typed_futhark_rune" => shows_typed_futhark_rune::run(&extra_args),
        "shows_rune_slots" => shows_rune_slots::run(&extra_args),
        "shows_rune_word_navigation" => shows_rune_word_navigation::run(&extra_args),
        "shows_binding_battle_state" => shows_binding_battle_state::run(&extra_args),
        "shows_acting_battle_state" => shows_acting_battle_state::run(&extra_args),
        "battle_stages" => battle_stages::run(&extra_args),
        "battle_against_npc" => battle_against_npc::run(&extra_args),
        "shows_spell_selection" => shows_spell_selection::run(&extra_args),
        other => {
            eprintln!("Unknown test: {:?}", other);
            eprintln!("Run without arguments to see available tests.");
            std::process::exit(1);
        }
    }
}

// ── Test 1 ────────────────────────────────────────────────────────────────────

mod shows_window {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        acceptance::initialize_app(&mut app, 1u8.into(), "Shows window");
        app.run();
    }
}

// ── Test 2 ────────────────────────────────────────────────────────────────────

mod shows_futhark_rune {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        app.add_systems(OnEnter(GameState::Adventure), spawn_futhark_rune);
        acceptance::initialize_app(&mut app, 2u8.into(), "Displays a single futhark rune");
        app.run();
    }

    fn spawn_futhark_rune(mut commands: Commands, game_assets: Res<GameAssets>) {
        commands.spawn(Sprite {
            image: game_assets.futhark.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: game_assets.futhark_layout.clone(),
                index: 0,
            }),
            ..default()
        });
    }
}

// ── Test 3 ────────────────────────────────────────────────────────────────────

mod shows_loading_rune_reveal {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        app.add_systems(OnEnter(GameState::Adventure), spawn_ready_confirmation);
        acceptance::initialize_app(
            &mut app,
            AcceptanceTest::from(3u8).with_grid(),
            "Processes audio one rune per frame with fade-in animation, then enters Ready",
        );
        app.run();
    }

    fn spawn_ready_confirmation(mut commands: Commands) {
        commands.spawn((
            Text::new("Processing complete — press F1 to pass, F2 to fail"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(16.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                ..default()
            },
        ));
    }
}

// ── Test 4 ────────────────────────────────────────────────────────────────────

mod shows_typed_futhark_rune {
    use super::*;

    const SPEED_MIN: f32 = 30.0;
    const SPEED_MAX: f32 = 60.0;
    const SPEED_STEP: f32 = 5.0;

    #[derive(Component)]
    struct TypedRuneDisplay;

    #[derive(Component)]
    struct SpeedLabel;

    #[derive(Component)]
    struct SpeedButton {
        delta: f32,
    }

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        app.add_systems(
            OnEnter(GameState::Adventure),
            spawn_futhark_keyboard.after(spawn_battle_hud_root),
        );
        app.add_systems(OnEnter(GameState::Adventure), spawn_typed_rune_display);
        app.add_systems(OnEnter(GameState::Adventure), spawn_speed_controls);
        app.add_systems(
            Update,
            (update_typed_rune, handle_speed_buttons, sync_speed_label)
                .chain()
                .run_if(in_state(GameState::Adventure)),
        );
        acceptance::initialize_app(
            &mut app,
            4u8.into(),
            "Displays only the rune that matches the most recently typed character",
        );
        app.run();
    }

    fn spawn_typed_rune_display(mut commands: Commands, game_assets: Res<GameAssets>) {
        commands.spawn((
            Sprite {
                image: game_assets.futhark.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: game_assets.futhark_layout.clone(),
                    index: 0,
                }),
                ..default()
            },
            Transform::from_xyz(0.0, 120.0, 0.0),
            Visibility::Hidden,
            TypedRuneDisplay,
        ));
    }

    fn spawn_speed_controls(mut commands: Commands, speed: Res<FutharkKeyboardAnimationSpeed>) {
        commands
            .spawn(Node {
                position_type: PositionType::Absolute,
                top: Val::Px(16.0),
                right: Val::Px(16.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                        SpeedButton { delta: -SPEED_STEP },
                    ))
                    .with_child((
                        Text::new("-"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                parent.spawn((
                    Text::new(format!("{:.0} °/s", speed.hue_degrees_per_second)),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    SpeedLabel,
                ));

                parent
                    .spawn((
                        Button,
                        Node {
                            width: Val::Px(32.0),
                            height: Val::Px(32.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                        SpeedButton { delta: SPEED_STEP },
                    ))
                    .with_child((
                        Text::new("+"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
            });
    }

    fn handle_speed_buttons(
        buttons: Query<(&Interaction, &SpeedButton), (Changed<Interaction>, With<Button>)>,
        mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
    ) {
        for (interaction, btn) in &buttons {
            if *interaction == Interaction::Pressed {
                speed.hue_degrees_per_second =
                    (speed.hue_degrees_per_second + btn.delta).clamp(SPEED_MIN, SPEED_MAX);
            }
        }
    }

    fn sync_speed_label(
        speed: Res<FutharkKeyboardAnimationSpeed>,
        mut labels: Query<&mut Text, With<SpeedLabel>>,
    ) {
        if !speed.is_changed() {
            return;
        }
        for mut text in &mut labels {
            *text = Text::new(format!("{:.0} °/s", speed.hue_degrees_per_second));
        }
    }

    fn update_typed_rune(
        mut typed_rune_input: MessageReader<futhark::TypedFutharkInput>,
        mut display: Query<(&mut Sprite, &mut Visibility), With<TypedRuneDisplay>>,
    ) {
        let Some(last_typed) = futhark::last_typed_futhark_character(&mut typed_rune_input) else {
            return;
        };

        let Ok((mut sprite, mut visibility)) = display.single_mut() else {
            return;
        };

        if let Some(index) = futhark::letter_to_index(last_typed) {
            if let Some(texture_atlas) = &mut sprite.texture_atlas {
                texture_atlas.index = index;
                *visibility = Visibility::Visible;
            }
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

// ── Test 5 ────────────────────────────────────────────────────────────────────

mod shows_rune_slots {
    use super::*;

    pub fn run(_args: &[String]) {
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
            5u8.into(),
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
}

// ── Test 6 ────────────────────────────────────────────────────────────────────

mod shows_rune_word_navigation {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        app.add_systems(
            OnEnter(GameState::Adventure),
            spawn_futhark_keyboard.after(spawn_battle_hud_root),
        );
        app.add_systems(OnEnter(GameState::Adventure), spawn_word_demo);
        acceptance::initialize_app(
            &mut app,
            6u8.into(),
            "Type futhark letters to fill the word slots left-to-right. Backspace clears the previous slot. Press Enter to play the word using conversational rune samples.",
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
        active_slot.entity = Some(slots[0]);

        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(48.0),
                top: Val::Px(110.0),
                ..default()
            },
            Text::new(
                "Type futhark letters to fill the word. Backspace clears the previous slot. Press Enter to play it.",
            ),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    }
}

// ── Test 7 ────────────────────────────────────────────────────────────────────

mod shows_binding_battle_state {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        configure_battle(&mut app);
        app.add_systems(
            OnEnter(GameState::Adventure),
            spawn_futhark_keyboard.after(spawn_battle_hud_root),
        );
        app.add_systems(OnEnter(GameState::Adventure), start_demo);
        app.add_systems(Update, reset_on_f3.run_if(in_state(GameState::Adventure)));
        acceptance::initialize_app(
            &mut app,
            7u8.into(),
            "Starts a random five-rune binding battle. Type guesses and press Enter to score rows. Correct=green, misplaced=yellow, wrong=red. Each scored row rises and a fresh row appears. Press F3 to reset with a new word.",
        );
        app.run();
    }

    fn start_demo(
        mut commands: Commands,
        mut start_binding: MessageWriter<StartBinding>,
        mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
    ) {
        let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::rng())
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
                    TextFont {
                        font_size: 24.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Text::new(
                        "Binding state: guess an unknown word. Correct=green, misplaced=yellow, wrong=red. Scored row rises; fresh row spawns below.",
                    ),
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

    fn reset_on_f3(
        keys: Res<ButtonInput<KeyCode>>,
        mut start_binding: MessageWriter<StartBinding>,
    ) {
        if keys.just_pressed(KeyCode::F3) {
            let selected = dictionary::random_futharkation_with_rune_length(5, &mut rand::rng())
                .expect("default dictionary should contain a five-rune futharkation");
            start_binding.write(StartBinding(Some(selected)));
        }
    }
}

// ── Test 8 ────────────────────────────────────────────────────────────────────

mod shows_acting_battle_state {
    use super::*;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        configure_battle(&mut app);
        app.add_systems(
            OnEnter(GameState::Adventure),
            spawn_futhark_keyboard.after(spawn_battle_hud_root),
        );
        app.add_systems(OnEnter(GameState::Adventure), start_demo);
        acceptance::initialize_app(
            &mut app,
            8u8.into(),
            "Acting battle state: guess target words. On success, acting ends.",
        );
        app.run();
    }

    fn start_demo(
        mut commands: Commands,
        mut start_acting: MessageWriter<StartActing>,
        mut player: ResMut<PlayerCombatState>,
        mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
    ) {
        let mut rng = rand::rng();
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
        player.hand = targets
            .iter()
            .map(|target| SpellDef {
                word: target.word.clone(),
                effects: Vec::new(),
                futharkation: target.letters.clone(),
                starter: true,
            })
            .collect();
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
                    Text::new(format!("StartActing: {}", label)),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Text::new("Acting: guess the full target word. Full match ends acting."),
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
}

// ── Test 10 ───────────────────────────────────────────────────────────────────

mod battle_stages {
    use super::*;

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

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        configure_battle(&mut app);

        app.init_resource::<StageFlow>();
        app.add_message::<QuickTime>();

        app.add_systems(
            OnEnter(GameState::Adventure),
            spawn_futhark_keyboard.after(spawn_battle_hud_root),
        );
        app.add_systems(OnEnter(GameState::Adventure), setup_demo);
        app.add_systems(
            Update,
            demo_controller.run_if(in_state(GameState::Adventure)),
        );
        app.add_systems(
            Update,
            (on_quicktime, on_acting_succeeded, on_binding_succeeded)
                .chain()
                .run_if(in_state(GameState::Adventure)),
        );

        acceptance::initialize_app(
            &mut app,
            10u8.into(),
            "Battle stages UAT: acting loops by default. After QuickTime (20s), next acting success transitions to binding.",
        );

        app.run();
    }

    fn setup_demo(
        mut commands: Commands,
        game_assets: Res<GameAssets>,
        specs: Res<Assets<NpcSpec>>,
        mut start_acting: MessageWriter<StartActing>,
        mut player: ResMut<PlayerCombatState>,
        mut speed: ResMut<FutharkKeyboardAnimationSpeed>,
        mut battle_state: ResMut<BattleState>,
    ) {
        let Some(spec) = specs.get(&game_assets.goblin_spec) else {
            return;
        };
        battle_state.npc = Some(spec.clone());
        let mut rng = rand::rng();

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
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Text::new(
                        "Flow: acting loops. QuickTime at 20s unlocks binding. Next acting success -> binding.",
                    ),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ),
                (
                    Text::new("F1 = pass, F2 = fail"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
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
                dictionary::random_futharkation_with_rune_length(5, &mut rand::rng()).unwrap(),
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

    fn set_player_hand_from_words(
        player: &mut PlayerCombatState,
        words: &[dictionary::Futharkation],
    ) {
        player.hand = words
            .iter()
            .map(|word| SpellDef {
                word: word.word.clone(),
                effects: Vec::new(),
                futharkation: word.letters.clone(),
                starter: true,
            })
            .collect();
    }
}

// ── Test 11 ───────────────────────────────────────────────────────────────────

mod battle_against_npc {
    use super::*;

    #[derive(Resource, Default)]
    struct ActiveFight {
        npc_type: Option<NpcType>,
        spec_applied: bool,
        max_health: Option<u32>,
        attack_count: usize,
    }

    #[derive(Component)]
    struct InstructionsPanel;

    #[derive(Component)]
    struct StatusLabel;

    #[derive(Component)]
    struct DeckLabel;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        configure_battle(&mut app);

        app.init_resource::<ActiveFight>();

        app.add_systems(
            OnEnter(GameState::Adventure),
            (
                spawn_futhark_keyboard.after(spawn_battle_hud_root),
                spawn_instructions,
                configure_keyboard_speed,
            ),
        );
        app.add_systems(
            Update,
            (
                pick_npc_on_function_keys,
                set_npc_hp_zero_on_f5,
                apply_spec_to_spawned_npc,
                loop_acting_on_success,
                on_binding_succeeded,
                on_binding_failed,
                update_status_label,
                update_deck_label,
            )
                .chain()
                .run_if(in_state(GameState::Adventure)),
        );

        acceptance::initialize_app(
            &mut app,
            11u8.into(),
            "Battle against NPC: F3 = Goblin, F4 = Robed cultist. F5 = set NPC HP to 0 (trigger binding).",
        );

        app.run();
    }

    fn configure_keyboard_speed(mut speed: ResMut<FutharkKeyboardAnimationSpeed>) {
        speed.hue_degrees_per_second = 45.0;
    }

    fn spawn_instructions(mut commands: Commands) {
        commands
            .spawn((
                InstructionsPanel,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(48.0),
                    top: Val::Px(40.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Choose your foe:"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("F3 - Goblin   |   F4 - Robed cultist   |   F5 - set HP to 0"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("F1 = pass, F2 = fail"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    StatusLabel,
                    Text::new("No battle active."),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.85, 0.55)),
                ));
                panel.spawn((
                    DeckLabel,
                    Text::new("Hand: -   Deck: -   Discard: -"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.75, 0.85, 0.95)),
                ));
            });
    }

    fn pick_npc_on_function_keys(
        input: Res<ButtonInput<KeyCode>>,
        game_assets: Res<GameAssets>,
        specs: Res<Assets<NpcSpec>>,
        mut battle_state: ResMut<BattleState>,
        mut fight: ResMut<ActiveFight>,
        mut player: ResMut<PlayerCombatState>,
        mut start_acting: MessageWriter<StartActing>,
        mut battle_start: MessageWriter<BattleStart>,
    ) {
        let picked_spec = if input.just_pressed(KeyCode::F3) {
            specs.get(&game_assets.goblin_spec)
        } else if input.just_pressed(KeyCode::F4) {
            specs.get(&game_assets.robed_spec)
        } else {
            None
        };

        let Some(spec) = picked_spec else {
            return;
        };

        battle_state.npc = Some(spec.clone());

        let mut rng = rand::rng();
        let words: Vec<dictionary::Futharkation> = [3usize, 4, 5, 3, 4]
            .iter()
            .filter_map(|&len| dictionary::random_futharkation_with_rune_length(len, &mut rng).ok())
            .collect();

        fight.npc_type = Some(spec.npc_type);
        fight.spec_applied = false;
        fight.max_health = Some(spec.max_health);
        fight.attack_count = spec.attacks.len();

        battle_start.write(BattleStart);
        set_player_hand_from_words(&mut player, &words);
        start_acting.write(StartActing);
    }

    fn apply_spec_to_spawned_npc(
        mut fight: ResMut<ActiveFight>,
        mut npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
    ) {
        if fight.spec_applied {
            return;
        }
        let Some(max_health) = fight.max_health else {
            return;
        };
        let Ok(mut npc) = npcs.single_mut() else {
            return;
        };
        npc.max = max_health;
        npc.hp = max_health;
        fight.spec_applied = true;
    }

    fn loop_acting_on_success(
        mut succeeded: MessageReader<ActingSucceeded>,
        mut start_acting: MessageWriter<StartActing>,
    ) {
        if succeeded.read().last().is_none() {
            return;
        }
        start_acting.write(StartActing);
    }

    fn set_npc_hp_zero_on_f5(
        input: Res<ButtonInput<KeyCode>>,
        mut npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
    ) {
        if !input.just_pressed(KeyCode::F5) {
            return;
        }
        for mut npc in &mut npcs {
            npc.hp = 0;
        }
    }

    fn on_binding_succeeded(
        mut succeeded: MessageReader<BindingSucceeded>,
        mut battle_state: ResMut<BattleState>,
    ) {
        if succeeded.read().last().is_none() {
            return;
        }
        battle_state.phase = BattlePhase::Victory;
    }

    fn on_binding_failed(
        mut failed: MessageReader<BindingFailed>,
        battle_state: Res<BattleState>,
        fight: Res<ActiveFight>,
        mut npcs: Query<&mut NpcCombatState, With<NpcSprite>>,
        mut start_acting: MessageWriter<StartActing>,
    ) {
        if failed.read().last().is_none() {
            return;
        }
        let half_health = fight.max_health.unwrap_or(0) / 2;
        let minimum_bindings = battle_state
            .npc
            .as_ref()
            .map(|spec| spec.minimum_bindings)
            .unwrap_or(0);
        for mut npc in &mut npcs {
            npc.hp = half_health.max(1);
            npc.bindings = minimum_bindings;
        }
        start_acting.write(StartActing);
    }

    fn set_player_hand_from_words(
        player: &mut PlayerCombatState,
        words: &[dictionary::Futharkation],
    ) {
        player.hand = words
            .iter()
            .map(|word| SpellDef {
                word: word.word.clone(),
                effects: Vec::new(),
                futharkation: word.letters.clone(),
                starter: true,
            })
            .collect();
    }

    fn update_status_label(
        fight: Res<ActiveFight>,
        mut labels: Query<&mut Text, With<StatusLabel>>,
    ) {
        let text = match fight.npc_type {
            None => "No battle active. Press F3 or F4.".to_string(),
            Some(npc_type) => {
                let name = match npc_type {
                    NpcType::Goblin => "Goblin",
                    NpcType::Robed => "Robed cultist",
                };
                let hp = fight.max_health.unwrap_or(0);
                format!(
                    "Fighting {name}: {hp} HP, {} attacks in spec.",
                    fight.attack_count
                )
            }
        };
        for mut label in &mut labels {
            if label.0 != text {
                label.0 = text.clone();
            }
        }
    }

    fn update_deck_label(
        player: Res<PlayerCombatState>,
        mut labels: Query<&mut Text, With<DeckLabel>>,
    ) {
        let hand_words: Vec<&str> = player.hand.iter().map(|c| c.word.as_str()).collect();
        let text = format!(
            "Hand [{}]: {}   |   Deck: {}   Discard: {}",
            hand_words.len(),
            hand_words.join(", "),
            player.deck.len(),
            player.discard.len(),
        );
        for mut label in &mut labels {
            if label.0 != text {
                label.0 = text.clone();
            }
        }
    }
}

// ── Test 12 ───────────────────────────────────────────────────────────────────

mod shows_spell_selection {
    use super::*;

    #[derive(Component)]
    struct StatusLabel;

    pub fn run(_args: &[String]) {
        let mut app = App::new();
        app.add_plugins(DefaultPlugins);
        configure_app(&mut app);
        configure_loading(&mut app);
        configure_battle(&mut app);

        app.add_systems(OnEnter(GameState::Adventure), spawn_instructions);
        app.add_systems(
            Update,
            (fire_binding_success_on_f3, update_status_label)
                .run_if(in_state(GameState::Adventure)),
        );

        acceptance::initialize_app(
            &mut app,
            12u8.into(),
            "Spell selection: F3 triggers BindingSucceeded. A modal shows two un-learned spells; clicking one learns it.",
        );

        app.run();
    }

    fn spawn_instructions(mut commands: Commands) {
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(48.0),
                    top: Val::Px(40.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                },
                ZIndex(200),
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new(
                        "Press F3 to trigger a binding success and open the spell selection window.",
                    ),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    Text::new("F1 = pass, F2 = fail"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel.spawn((
                    StatusLabel,
                    Text::new("Learned: -"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.75, 0.85, 0.95)),
                ));
            });
    }

    fn fire_binding_success_on_f3(
        input: Res<ButtonInput<KeyCode>>,
        mut succeeded: MessageWriter<BindingSucceeded>,
    ) {
        if input.just_pressed(KeyCode::F3) {
            succeeded.write(BindingSucceeded);
        }
    }

    fn update_status_label(
        learned: Res<LearnedSpells>,
        selection: Res<SpellSelection>,
        mut labels: Query<&mut Text, With<StatusLabel>>,
    ) {
        let open = selection.is_open();
        let text = format!(
            "Learned [{}]: {}   |   Selection open: {}",
            learned.words.len(),
            learned.words.join(", "),
            open,
        );
        for mut label in &mut labels {
            if label.0 != text {
                label.0 = text.clone();
            }
        }
    }
}
