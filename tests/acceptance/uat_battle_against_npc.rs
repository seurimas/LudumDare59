use LudumDare59::{
    GameAssets, GameState, acceptance,
    combat::BattleStart,
    configure_app, configure_loading, dictionary,
    futhark::{FutharkKeyboardAnimationSpeed, spawn_futhark_keyboard},
    health::{NpcCombatState, PlayerCombatState},
    npcs::NpcSpec,
    rune_words::{
        battle::{BattlePhase, BattleState, NpcType, configure_battle},
        battle_states::{
            acting::{ActingSucceeded, StartActing},
            binding::{BindingFailed, BindingSucceeded},
        },
    },
    spellbook::SpellDef,
    ui::{arena::NpcSprite, hud_root::spawn_battle_hud_root},
};
use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

const TEST_ID: u8 = 11;

fn main() {
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
        TEST_ID.into(),
        "Battle against NPC: F3 = Goblin, F4 = Robed cultist. F5 = set NPC HP to 0 (trigger binding).",
    );

    app.run();
}

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

    let mut rng = rand::thread_rng();
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

fn update_status_label(fight: Res<ActiveFight>, mut labels: Query<&mut Text, With<StatusLabel>>) {
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
