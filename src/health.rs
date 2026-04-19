use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource)]
pub struct PlayerCombatState {
    pub hp: u32,
    pub max: u32,
}

impl Default for PlayerCombatState {
    fn default() -> Self {
        Self { hp: 78, max: 100 }
    }
}

#[derive(Component)]
pub struct NpcCombatState {
    pub hp: u32,
    pub max: u32,
    pub bindings: u32,
    pub attack_state: NpcAttackState,
    pub chosen_attack: Option<NpcAttack>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum NpcAttackState {
    Stunned(f32),
    WaitingFor(f32),
    AttackingIn(f32),
    Cooldown(f32),
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NpcAttack {
    pub thinking_time: f32,
    pub attack_time: f32,
    pub damage: u32,
    pub cooldown_time: f32,
}

impl Default for NpcCombatState {
    fn default() -> Self {
        Self {
            hp: 100,
            max: 100,
            bindings: 0,
            attack_state: NpcAttackState::Cooldown(3.0),
            chosen_attack: None,
        }
    }
}
