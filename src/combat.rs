use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::GameAssets;
use crate::GameState;
use crate::RunStats;
use crate::dictionary;
use crate::health::{NpcAttack, NpcAttackState, NpcCombatState, PlayerCombatState};
use crate::npcs::NpcSpec;
use crate::rune_words::battle::{BattlePhase, BattleState, PendingRowGrading};
use crate::rune_words::battle_states::acting::StartActing;
use crate::rune_words::battle_states::binding::{BindingData, BindingSucceeded, StartBinding};
use crate::rune_words::battle_states::{LastGradedWord, WordBook};
use crate::spellbook::{Book, LearnedSpells};
use crate::tutorial::TutorialState;
use crate::ui::effects::EffectsQueue;
use crate::ui::spell_selection::SpellSelection;

const NPC_SPAWN_DELAY: f32 = 5.0;

/// Timer that counts down before a new NPC is spawned.
#[derive(Resource)]
pub struct NpcSpawnTimer {
    pub remaining: f32,
    pub active: bool,
}

/// Raised to signal the start of a fresh combat. Consumers reset per-combat
/// state (deck/hand/discard) when this fires.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug, Default)]
pub struct BattleStart;

pub fn configure_combat(app: &mut App) {
    app.add_message::<NpcAttack>();
    app.add_message::<BattleStart>();
    app.insert_resource(NpcSpawnTimer {
        remaining: NPC_SPAWN_DELAY,
        active: false,
    });
    app.add_systems(
        OnEnter(GameState::Adventure),
        (reset_adventure_state, reset_learned_spells_to_starters).chain(),
    );
    app.add_systems(
        Update,
        (
            tick_npc_attacks,
            apply_npc_damage_to_player,
            reset_player_deck_on_battle_start,
            setup_binding_target_on_battle_start,
            trigger_binding_on_npc_death,
            track_enemies_defeated,
            trigger_victory_on_binding_success,
            resume_combat_after_effects,
            tick_npc_spawn_timer,
            check_player_death,
            debug_kill_player,
        )
            .run_if(in_state(GameState::Adventure)),
    );
}

/// Reset all game state to a clean slate when (re-)entering Adventure.
fn reset_adventure_state(
    mut commands: Commands,
    mut player: ResMut<PlayerCombatState>,
    mut battle_state: ResMut<BattleState>,
    mut effects: ResMut<EffectsQueue>,
    mut binding_data: ResMut<BindingData>,
    mut spell_selection: ResMut<SpellSelection>,
    mut spawn_timer: ResMut<NpcSpawnTimer>,
    mut run_stats: ResMut<RunStats>,
    mut pending_grading: ResMut<PendingRowGrading>,
    mut last_graded: ResMut<LastGradedWord>,
    mut word_book: ResMut<WordBook>,
    npc_query: Query<Entity, With<NpcCombatState>>,
) {
    // Player: full health, no shields/buffs, empty deck
    *player = PlayerCombatState::default();
    player.hp = player.max;

    // Battle: idle phase, no NPC
    battle_state.phase = BattlePhase::Idle;
    battle_state.npc = None;
    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = None;
    battle_state.pending_settle_frames = 0;
    battle_state.next_row_id = 0;
    battle_state.resolved_rows = 0;

    // Clear effects, binding, and spell selection
    *effects = EffectsQueue::default();
    *binding_data = BindingData::default();
    *pending_grading = PendingRowGrading::default();
    *last_graded = LastGradedWord::default();
    *word_book = WordBook::default();
    spell_selection.close();

    // Reset NPC spawn timer
    spawn_timer.remaining = NPC_SPAWN_DELAY;
    spawn_timer.active = false;

    // Reset run stats
    run_stats.enemies_defeated = 0;

    // Despawn any leftover NPC entities
    for entity in &npc_query {
        commands.entity(entity).despawn();
    }
}

fn reset_learned_spells_to_starters(
    mut learned: ResMut<LearnedSpells>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
    tutorial: Option<Res<TutorialState>>,
) {
    if tutorial.map_or(false, |t| t.active) {
        return;
    }
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };
    learned.reset_to_starters(book.spells());
}

fn reset_player_deck_on_battle_start(
    mut events: MessageReader<BattleStart>,
    mut player: ResMut<PlayerCombatState>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
    tutorial: Option<Res<TutorialState>>,
    learned: Res<LearnedSpells>,
) {
    if events.read().count() == 0 {
        return;
    }
    if tutorial.map_or(false, |t| t.active) {
        return;
    }
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };
    let known: Vec<_> = learned
        .filter_spells(book.spells())
        .into_iter()
        .cloned()
        .collect();
    let mut rng = rand::thread_rng();
    player.reset_for_new_combat(&known, &mut rng);
}

fn tick_npc_attacks(
    time: Res<Time>,
    battle_state: Option<Res<BattleState>>,
    tutorial: Option<Res<TutorialState>>,
    mut npcs: Query<&mut NpcCombatState>,
    mut npc_attack: MessageWriter<NpcAttack>,
) {
    let in_binding = battle_state
        .as_ref()
        .is_some_and(|s| matches!(s.phase, BattlePhase::Binding));

    let in_tutorial = tutorial.as_ref().is_some_and(|t| t.active);

    let dt = time.delta_secs();

    for mut npc in &mut npcs {
        if in_binding {
            continue;
        }

        // During tutorial, timers tick but attacks never fire
        if in_tutorial {
            continue;
        }

        match npc.attack_state {
            NpcAttackState::Stunned(t) => {
                let remaining = t - dt;
                if remaining <= 0.0 {
                    npc.chosen_attack = None;
                    npc.attack_state = NpcAttackState::Cooldown(0.0);
                } else {
                    npc.attack_state = NpcAttackState::Stunned(remaining);
                }
            }
            NpcAttackState::WaitingFor(t) => {
                let remaining = t - dt;
                if remaining <= 0.0 {
                    let attack_time = npc.chosen_attack.map(|a| a.attack_time).unwrap_or(0.0);
                    npc.attack_state = NpcAttackState::AttackingIn(attack_time);
                } else {
                    npc.attack_state = NpcAttackState::WaitingFor(remaining);
                }
            }
            NpcAttackState::AttackingIn(t) => {
                let remaining = t - dt;
                if remaining <= 0.0 {
                    if let Some(attack) = npc.chosen_attack {
                        npc_attack.write(NpcAttack(attack.damage));
                        npc.attack_state = NpcAttackState::Cooldown(attack.cooldown_time);
                    } else {
                        npc.attack_state = NpcAttackState::Cooldown(0.0);
                    }
                } else {
                    npc.attack_state = NpcAttackState::AttackingIn(remaining);
                }
            }
            NpcAttackState::Cooldown(t) => {
                let remaining = t - dt;
                if remaining <= 0.0 {
                    npc.chosen_attack = None;
                    npc.attack_state = NpcAttackState::Cooldown(0.0);
                } else {
                    npc.attack_state = NpcAttackState::Cooldown(remaining);
                }
            }
        }

        let needs_attack = npc.chosen_attack.is_none()
            && !matches!(npc.attack_state, NpcAttackState::Stunned(_))
            && !npc.attacks.is_empty();

        if needs_attack {
            let idx = rand::thread_rng().gen_range(0..npc.attacks.len());
            let attack = npc.attacks[idx];
            npc.chosen_attack = Some(attack);
            npc.attack_state = NpcAttackState::WaitingFor(attack.thinking_time);
        }
    }
}

/// When a battle starts and the NPC spec is known, pick a random binding word
/// from the NPC's spec and store it in BindingData so it's ready when binding begins.
fn setup_binding_target_on_battle_start(
    mut events: MessageReader<BattleStart>,
    battle_state: Res<BattleState>,
    tutorial: Option<Res<TutorialState>>,
    mut binding_data: ResMut<BindingData>,
) {
    if events.read().count() == 0 {
        return;
    }

    // During tutorial, use a fixed binding word
    if tutorial.as_ref().is_some_and(|t| t.active) {
        match dictionary::futharkation_from_word(crate::tutorial::TUTORIAL_BINDING_WORD) {
            Ok(futharkation) => {
                binding_data.target = Some(futharkation);
                binding_data.attempts_remaining = 0; // unlimited attempts
            }
            Err(e) => {
                bevy::log::warn!(
                    "Could not futharkate tutorial binding word '{}': {}",
                    crate::tutorial::TUTORIAL_BINDING_WORD,
                    e
                );
            }
        }
        return;
    }

    let Some(spec) = battle_state.npc.as_ref() else {
        return;
    };

    let mut rng = rand::thread_rng();
    let Some(word) = spec.binding_words.choose(&mut rng) else {
        return;
    };

    match dictionary::futharkation_from_word(word) {
        Ok(futharkation) => {
            binding_data.target = Some(futharkation);
            binding_data.attempts_remaining = spec.minimum_bindings;
        }
        Err(e) => {
            bevy::log::warn!("Could not futharkate binding word '{}': {}", word, e);
        }
    }
}

/// When an NPC's HP reaches 0 during the Acting phase, trigger the binding phase.
fn trigger_binding_on_npc_death(
    battle_state: Res<BattleState>,
    npcs: Query<&NpcCombatState>,
    mut start_binding: MessageWriter<StartBinding>,
) {
    if !matches!(battle_state.phase, BattlePhase::Acting) {
        return;
    }
    for npc in &npcs {
        if npc.hp == 0 {
            start_binding.write(StartBinding(None));
            return;
        }
    }
}

fn apply_npc_damage_to_player(
    mut attacks: MessageReader<NpcAttack>,
    mut player: ResMut<PlayerCombatState>,
) {
    for NpcAttack(damage) in attacks.read() {
        player.apply_damage(*damage);
    }
}

fn track_enemies_defeated(
    mut events: MessageReader<BindingSucceeded>,
    mut run_stats: ResMut<RunStats>,
) {
    for _ in events.read() {
        run_stats.enemies_defeated += 1;
    }
}

fn check_player_death(
    player: Res<PlayerCombatState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if player.hp == 0 {
        next_state.set(GameState::GameOver);
    }
}

fn debug_kill_player(input: Res<ButtonInput<KeyCode>>, mut player: ResMut<PlayerCombatState>) {
    if input.pressed(KeyCode::ControlLeft)
        && input.pressed(KeyCode::ShiftLeft)
        && input.just_pressed(KeyCode::Digit0)
    {
        player.hp = 0;
    }
}

/// After the effects queue drains and phase is Idle with an NPC present,
/// either restart acting (NPC alive) or trigger binding (NPC dead).
fn resume_combat_after_effects(
    effects: Res<EffectsQueue>,
    battle_state: Res<BattleState>,
    tutorial: Option<Res<TutorialState>>,
    npcs: Query<&NpcCombatState>,
    mut start_acting: MessageWriter<StartActing>,
    mut start_binding: MessageWriter<StartBinding>,
) {
    if tutorial.as_ref().is_some_and(|t| t.active) {
        return;
    }
    if effects.is_busy() {
        return;
    }
    if !matches!(battle_state.phase, BattlePhase::Idle) {
        return;
    }
    if battle_state.npc.is_none() {
        return;
    }

    let npc_dead = npcs.iter().any(|npc| npc.hp == 0);
    if npc_dead {
        start_binding.write(StartBinding(None));
    } else {
        start_acting.write(StartActing);
    }
}

/// After binding succeeds (non-tutorial), transition to Victory so the death
/// fade plays and the NPC is eventually cleared.
fn trigger_victory_on_binding_success(
    mut events: MessageReader<BindingSucceeded>,
    tutorial: Option<Res<TutorialState>>,
    mut battle_state: ResMut<BattleState>,
) {
    if tutorial.as_ref().is_some_and(|t| t.active) {
        events.clear();
        return;
    }
    if events.read().last().is_none() {
        return;
    }
    if battle_state.npc.is_some() {
        battle_state.phase = BattlePhase::Victory;
    }
}

/// When there is no NPC and phase is Idle, count down a timer and then spawn a
/// random NPC to start a new battle.
fn tick_npc_spawn_timer(
    time: Res<Time>,
    mut spawn_timer: ResMut<NpcSpawnTimer>,
    mut battle_state: ResMut<BattleState>,
    tutorial: Option<Res<TutorialState>>,
    game_assets: Option<Res<GameAssets>>,
    npc_specs: Res<Assets<NpcSpec>>,
    spell_selection: Res<SpellSelection>,
    mut battle_start: MessageWriter<BattleStart>,
    mut start_acting: MessageWriter<StartActing>,
) {
    if tutorial.as_ref().is_some_and(|t| t.active) {
        return;
    }

    if spell_selection.is_open() {
        spawn_timer.active = false;
        return;
    }

    let npc_gone = battle_state.npc.is_none() && matches!(battle_state.phase, BattlePhase::Idle);

    if !npc_gone {
        spawn_timer.active = false;
        return;
    }

    if !spawn_timer.active {
        spawn_timer.remaining = NPC_SPAWN_DELAY;
        spawn_timer.active = true;
    }

    spawn_timer.remaining -= time.delta_secs();
    if spawn_timer.remaining > 0.0 {
        return;
    }

    spawn_timer.active = false;

    let Some(game_assets) = game_assets else {
        return;
    };

    let candidates: Vec<&Handle<NpcSpec>> = vec![&game_assets.goblin_spec, &game_assets.robed_spec];
    let mut rng = rand::thread_rng();
    let Some(&chosen_handle) = candidates.choose(&mut rng) else {
        return;
    };
    let Some(spec) = npc_specs.get(chosen_handle) else {
        return;
    };

    battle_state.npc = Some(spec.clone());
    battle_start.write(BattleStart);
    start_acting.write(StartActing);
}
