use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use rand::Rng;

use crate::GameAssets;
use crate::GameState;
use crate::health::{NpcAttack, NpcAttackState, NpcCombatState, PlayerCombatState};
use crate::rune_words::battle::{BattlePhase, BattleState};
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
        (tick_npc_attacks, reset_player_deck_on_battle_start)
            .run_if(in_state(GameState::Ready)),
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
