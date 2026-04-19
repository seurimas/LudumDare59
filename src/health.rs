use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource)]
pub struct PlayerCombatState {
    pub hp: u32,
    pub max: u32,
    pub shields: Vec<ShieldState>,
    pub attack_buffs: Vec<Buff>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldState {
    pub amount: u32,
    pub expires_in: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Buff {
    pub name: String,
    pub value: i32,
    pub expires_in: f32,
}

impl Default for PlayerCombatState {
    fn default() -> Self {
        Self {
            hp: 78,
            max: 100,
            shields: Vec::new(),
            attack_buffs: Vec::new(),
        }
    }
}

impl PlayerCombatState {
    pub fn effective_hp(&self) -> f32 {
        let total_shield: u32 = self.shields.iter().map(|s| s.amount).sum();
        self.hp as f32 + total_shield as f32
    }

    pub fn effective_attack(&self, base_attack: u32) -> f32 {
        let total_buff: i32 = self.attack_buffs.iter().map(|b| b.value).sum();
        (base_attack as i32 + total_buff) as f32
    }

    pub fn apply_damage(&mut self, mut damage: u32) {
        // Shields absorb damage first
        let mut sorted_shields = self.shields.clone();
        sorted_shields.sort_by_key(|s| s.expires_in as u32);
        sorted_shields.retain_mut(|shield| {
            if damage == 0 {
                return true; // No more damage to apply
            }
            if shield.amount > damage {
                shield.amount -= damage;
                damage = 0;
                true
            } else {
                damage -= shield.amount;
                false // Shield is fully consumed
            }
        });
        self.shields = sorted_shields;

        // Apply remaining damage to HP
        if damage > 0 {
            self.hp = self.hp.saturating_sub(damage);
        }
    }
}

#[derive(Component)]
pub struct NpcCombatState {
    pub hp: u32,
    pub max: u32,
    pub bindings: u32,
    pub attack_state: NpcAttackState,
    pub chosen_attack: Option<NpcAttackSpec>,
    pub attacks: Vec<NpcAttackSpec>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum NpcAttackState {
    Stunned(f32),
    WaitingFor(f32),
    AttackingIn(f32),
    Cooldown(f32),
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct NpcAttackSpec {
    pub thinking_time: f32,
    pub attack_time: f32,
    pub damage: u32,
    pub cooldown_time: f32,
    pub flicker_rate: f32,
}

#[derive(bevy::ecs::message::Message, Clone, Copy, Debug)]
pub struct NpcAttack(pub u32);

impl Default for NpcCombatState {
    fn default() -> Self {
        Self {
            hp: 100,
            max: 100,
            bindings: 0,
            attack_state: NpcAttackState::Cooldown(3.0),
            chosen_attack: None,
            attacks: Vec::new(),
        }
    }
}
