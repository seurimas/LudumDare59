use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use rand::Rng;
use rand::seq::SliceRandom;

use crate::GameAssets;
use crate::GameState;
use crate::RunStats;
use crate::dictionary;
use crate::health::{NpcAttack, NpcAttackState, NpcCombatState, PlayerCombatState};
use crate::rune_words::battle::{BattlePhase, BattleState};
use crate::rune_words::battle_states::binding::{BindingData, BindingSucceeded, StartBinding};
use crate::spellbook::Book;

/// Raised to signal the start of a fresh combat. Consumers reset per-combat
/// state (deck/hand/discard) when this fires.
#[derive(bevy::ecs::message::Message, Clone, Copy, Debug, Default)]
pub struct BattleStart;

pub fn configure_combat(app: &mut App) {
    app.add_message::<NpcAttack>();
    app.add_message::<BattleStart>();
    app.add_systems(
        Update,
        (
            tick_npc_attacks,
            apply_npc_damage_to_player,
            reset_player_deck_on_battle_start,
            setup_binding_target_on_battle_start,
            trigger_binding_on_npc_death,
            track_enemies_defeated,
            check_player_death,
        )
            .run_if(in_state(GameState::Adventure)),
    );
}

fn reset_player_deck_on_battle_start(
    mut events: MessageReader<BattleStart>,
    mut player: ResMut<PlayerCombatState>,
    game_assets: Option<Res<GameAssets>>,
    books: Res<Assets<Book>>,
) {
    if events.read().count() == 0 {
        return;
    }
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(book) = books.get(&game_assets.spellbook) else {
        return;
    };
    let mut rng = rand::thread_rng();
    player.reset_for_new_combat(book.spells(), &mut rng);
}

fn tick_npc_attacks(
    time: Res<Time>,
    battle_state: Option<Res<BattleState>>,
    mut npcs: Query<&mut NpcCombatState>,
    mut npc_attack: MessageWriter<NpcAttack>,
) {
    let in_binding = battle_state
        .as_ref()
        .is_some_and(|s| matches!(s.phase, BattlePhase::Binding));

    let dt = time.delta_secs();

    for mut npc in &mut npcs {
        if in_binding {
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
    mut binding_data: ResMut<BindingData>,
) {
    if events.read().count() == 0 {
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
