use bevy::prelude::*;
use rand::seq::SliceRandom;
use serde::Deserialize;

use crate::spellbook::SpellDef;

pub const STARTING_HAND_SIZE: usize = 4;

#[derive(Resource)]
pub struct PlayerCombatState {
    pub hp: u32,
    pub max: u32,
    pub shields: Vec<ShieldState>,
    pub attack_buffs: Vec<Buff>,
    pub deck: Vec<SpellDef>,
    pub hand: Vec<SpellDef>,
    pub discard: Vec<SpellDef>,
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
            deck: Vec::new(),
            hand: Vec::new(),
            discard: Vec::new(),
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

    /// Reset for a brand-new combat: put every spell back in the deck,
    /// clear hand and discard, shuffle, and draw a starting hand.
    pub fn reset_for_new_combat<R: rand::Rng + ?Sized>(
        &mut self,
        book: &[SpellDef],
        rng: &mut R,
    ) {
        self.deck = book.to_vec();
        self.hand.clear();
        self.discard.clear();
        self.deck.shuffle(rng);
        self.draw_up_to(STARTING_HAND_SIZE, rng);
    }

    /// Draw `count` cards, reshuffling the discard pile into the deck
    /// if the deck runs out mid-draw. Returns the number actually drawn.
    pub fn draw<R: rand::Rng + ?Sized>(&mut self, count: usize, rng: &mut R) -> usize {
        let mut drawn = 0;
        for _ in 0..count {
            if self.deck.is_empty() {
                if self.discard.is_empty() {
                    break;
                }
                std::mem::swap(&mut self.deck, &mut self.discard);
                self.deck.shuffle(rng);
            }
            if let Some(card) = self.deck.pop() {
                self.hand.push(card);
                drawn += 1;
            }
        }
        drawn
    }

    /// Draw until the hand has at least `target` cards.
    pub fn draw_up_to<R: rand::Rng + ?Sized>(&mut self, target: usize, rng: &mut R) -> usize {
        let needed = target.saturating_sub(self.hand.len());
        if needed == 0 {
            return 0;
        }
        self.draw(needed, rng)
    }

    /// Remove the first card in the hand matching `word` and move it to the
    /// discard pile. Returns `true` if such a card existed.
    pub fn cast_from_hand(&mut self, word: &str) -> bool {
        let Some(index) = self.hand.iter().position(|c| c.word == word) else {
            return false;
        };
        let card = self.hand.remove(index);
        self.discard.push(card);
        true
    }

    pub fn tick(&mut self, dt: f32) {
        // Tick down shield durations
        for shield in &mut self.shields {
            shield.expires_in -= dt;
        }
        self.shields.retain(|s| s.expires_in > 0.0);

        // Tick down buff durations
        for buff in &mut self.attack_buffs {
            buff.expires_in -= dt;
        }
        self.attack_buffs.retain(|b| b.expires_in > 0.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spellbook::{SpellDef, SpellEffect};
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    fn sample_book() -> Vec<SpellDef> {
        vec![
            SpellDef {
                word: "a".into(),
                effects: vec![SpellEffect::Damage { amount: 1 }],
            },
            SpellDef {
                word: "b".into(),
                effects: vec![SpellEffect::Damage { amount: 2 }],
            },
            SpellDef {
                word: "c".into(),
                effects: vec![SpellEffect::Damage { amount: 3 }],
            },
            SpellDef {
                word: "d".into(),
                effects: vec![SpellEffect::Damage { amount: 4 }],
            },
            SpellDef {
                word: "e".into(),
                effects: vec![SpellEffect::Damage { amount: 5 }],
            },
            SpellDef {
                word: "f".into(),
                effects: vec![SpellEffect::Damage { amount: 6 }],
            },
        ]
    }

    #[test]
    fn reset_for_new_combat_fills_starting_hand() {
        let mut rng = StdRng::seed_from_u64(42);
        let mut player = PlayerCombatState::default();
        let book = sample_book();
        player.reset_for_new_combat(&book, &mut rng);

        assert_eq!(player.hand.len(), STARTING_HAND_SIZE);
        assert_eq!(player.deck.len(), book.len() - STARTING_HAND_SIZE);
        assert!(player.discard.is_empty());
    }

    #[test]
    fn cast_from_hand_moves_card_to_discard_and_draw_refills() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut player = PlayerCombatState::default();
        player.reset_for_new_combat(&sample_book(), &mut rng);

        let cast_word = player.hand[0].word.clone();
        assert!(player.cast_from_hand(&cast_word));

        assert_eq!(player.hand.len(), STARTING_HAND_SIZE - 1);
        assert_eq!(player.discard.len(), 1);
        assert_eq!(player.discard[0].word, cast_word);

        let drawn = player.draw(1, &mut rng);
        assert_eq!(drawn, 1);
        assert_eq!(player.hand.len(), STARTING_HAND_SIZE);
    }

    #[test]
    fn cast_from_hand_returns_false_for_unknown_word() {
        let mut rng = StdRng::seed_from_u64(1);
        let mut player = PlayerCombatState::default();
        player.reset_for_new_combat(&sample_book(), &mut rng);
        assert!(!player.cast_from_hand("nonexistent"));
    }

    #[test]
    fn draw_reshuffles_discard_when_deck_empty() {
        let mut rng = StdRng::seed_from_u64(3);
        let mut player = PlayerCombatState::default();
        let book = sample_book();
        player.reset_for_new_combat(&book, &mut rng);

        // Cast every card currently in hand.
        while let Some(card) = player.hand.first().cloned() {
            player.cast_from_hand(&card.word);
        }
        assert_eq!(player.hand.len(), 0);
        assert_eq!(player.discard.len(), STARTING_HAND_SIZE);

        // Drain the deck into hand.
        let remaining_deck = player.deck.len();
        let drawn = player.draw(remaining_deck, &mut rng);
        assert_eq!(drawn, remaining_deck);
        assert!(player.deck.is_empty());

        // Next draw should reshuffle discard and pull from it.
        let drawn_more = player.draw(1, &mut rng);
        assert_eq!(drawn_more, 1);
        assert!(!player.discard.is_empty() || !player.deck.is_empty() || player.hand.len() > 0);
    }

    #[test]
    fn draw_up_to_fills_hand_without_exceeding_target() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut player = PlayerCombatState::default();
        player.reset_for_new_combat(&sample_book(), &mut rng);

        let before = player.hand.len();
        let drawn = player.draw_up_to(STARTING_HAND_SIZE, &mut rng);
        assert_eq!(drawn, 0, "hand is already at the target size");
        assert_eq!(player.hand.len(), before);
    }

    #[test]
    fn draw_stops_when_deck_and_discard_both_empty() {
        let mut rng = StdRng::seed_from_u64(99);
        let mut player = PlayerCombatState::default();
        // Only one card in the whole system.
        player.reset_for_new_combat(
            &[SpellDef {
                word: "only".into(),
                effects: Vec::new(),
            }],
            &mut rng,
        );
        assert_eq!(player.hand.len(), 1);
        assert!(player.deck.is_empty());
        assert!(player.discard.is_empty());

        let drawn = player.draw(3, &mut rng);
        assert_eq!(drawn, 0);
    }
}
