use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::rune_slots::{RuneSlot, RuneSlotConfig, RuneSlotForegroundSet, spawn_rune_word};
use crate::{GameAssets, futhark};

pub const ACTIVE_ROW_TOP: f32 = 236.0;
pub const ROW_RISE: f32 = 72.0;
pub const ROW_LEFT: f32 = 36.0;
pub const SLOT_SPACING: f32 = 68.0;
pub const SLOT_SIZE: f32 = 48.0;
pub const ROW_RISE_DURATION_SECONDS: f32 = 0.5;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BattleSet {
    CheckAnimations,
    PostAnimation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BattlePhase {
    #[default]
    Idle,
    Binding,
    Acting,
    Reacting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuneMatchState {
    Missing,
    Present,
    Correct,
}

impl RuneMatchState {
    pub fn background_color(self) -> Color {
        match self {
            Self::Missing => Color::srgb(0.78, 0.2, 0.2),
            Self::Present => Color::srgb(0.85, 0.72, 0.16),
            Self::Correct => Color::srgb(0.24, 0.68, 0.32),
        }
    }
}

#[derive(Resource, Default)]
pub struct BattleState {
    pub phase: BattlePhase,
    pub active_row_slots: Vec<Entity>,
    pub pending_resolved_row: Option<u32>,
    pub pending_settle_frames: u8,
    pub next_row_id: u32,
    pub resolved_rows: usize,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BattleRuneSlot {
    pub row_id: u32,
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct BattleRowMotion {
    pub start_top: f32,
    pub end_top: f32,
    pub elapsed_seconds: f32,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct RowResolved(pub u32);

pub fn configure_battle(app: &mut App) {
    app.init_resource::<BattleState>();
    app.add_message::<RowResolved>();
    app.configure_sets(
        Update,
        BattleSet::CheckAnimations.before(BattleSet::PostAnimation),
    );
    app.add_systems(
        Update,
        (animate_resolved_rows, check_row_animation_done)
            .chain()
            .in_set(BattleSet::CheckAnimations),
    );
    super::battle_states::configure_battle_states(app);
}

fn animate_resolved_rows(
    mut commands: Commands,
    time: Res<Time>,
    mut moving_rows: Query<(Entity, &mut Node, &mut BattleRowMotion)>,
) {
    for (entity, mut node, mut motion) in &mut moving_rows {
        motion.elapsed_seconds += time.delta_secs();
        let progress = (motion.elapsed_seconds / ROW_RISE_DURATION_SECONDS).clamp(0.0, 1.0);
        let top = motion.start_top + (motion.end_top - motion.start_top) * progress;
        node.top = Val::Px(top);
        if progress >= 1.0 {
            commands.entity(entity).remove::<BattleRowMotion>();
        }
    }
}

fn check_row_animation_done(
    mut commands: Commands,
    mut battle_state: ResMut<BattleState>,
    mut row_resolved: MessageWriter<RowResolved>,
    row_slots: Query<(Entity, &BattleRuneSlot)>,
    moving_rows: Query<(&BattleRuneSlot, Option<&BattleRowMotion>)>,
) {
    let Some(row_id) = battle_state.pending_resolved_row else {
        return;
    };

    if battle_state.pending_settle_frames > 0 {
        battle_state.pending_settle_frames -= 1;
        return;
    }

    let still_animating = moving_rows.iter().any(|(slot, motion)| {
        slot.row_id == row_id
            && motion.map_or(false, |m| m.elapsed_seconds < ROW_RISE_DURATION_SECONDS)
    });
    if still_animating {
        return;
    }

    for (entity, slot) in &row_slots {
        if slot.row_id == row_id {
            commands.entity(entity).remove::<(Button, Interaction)>();
        }
    }

    battle_state.pending_resolved_row = None;
    battle_state.resolved_rows += 1;
    row_resolved.write(RowResolved(row_id));
}

pub fn reset_battle_state(
    commands: &mut Commands,
    battle_state: &mut BattleState,
    existing_row_entities: impl IntoIterator<Item = Entity>,
) {
    for entity in existing_row_entities {
        commands.entity(entity).despawn();
    }
    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = None;
    battle_state.pending_settle_frames = 0;
    battle_state.next_row_id = 0;
    battle_state.resolved_rows = 0;
}

pub fn spawn_battle_row(
    commands: &mut Commands,
    game_assets: &GameAssets,
    row_id: u32,
    rune_count: usize,
    top: f32,
) -> Vec<Entity> {
    let start_left = ROW_LEFT;
    let configs = (0..rune_count)
        .map(|index| RuneSlotConfig {
            left: Val::Px(start_left + index as f32 * SLOT_SPACING),
            top: Val::Px(top),
            size: SLOT_SIZE,
            background_color: idle_row_color(),
            foreground_set: RuneSlotForegroundSet::Primary,
            initial_rune: None,
        })
        .collect();

    let slots = spawn_rune_word(commands, game_assets, configs);
    for entity in &slots {
        commands.entity(*entity).insert(BattleRuneSlot { row_id });
    }
    slots
}

pub fn collect_guess_submission(
    slot_entities: &[Entity],
    slots: &Query<&RuneSlot>,
) -> Option<Vec<Option<char>>> {
    let mut guess = Vec::with_capacity(slot_entities.len());
    let mut has_any_rune = false;

    for entity in slot_entities {
        let slot = slots.get(*entity).ok()?;
        let guess_letter = slot
            .rune_index
            .and_then(|rune_index| futhark::LETTERS.get(rune_index).copied());
        has_any_rune |= guess_letter.is_some();
        guess.push(guess_letter);
    }

    has_any_rune.then_some(guess)
}

/// Inserts `BattleRowMotion` (rising) on all `BattleRuneSlot` entities not in `active_entities`.
pub fn push_all_non_active_slots_up(
    commands: &mut Commands,
    active_entities: &HashSet<Entity>,
    slots: &Query<(Entity, &Node), With<BattleRuneSlot>>,
) {
    for (entity, node) in slots.iter() {
        if active_entities.contains(&entity) {
            continue;
        }
        let start_top = match node.top {
            Val::Px(top) => top,
            _ => ACTIVE_ROW_TOP,
        };
        commands.entity(entity).insert(BattleRowMotion {
            start_top,
            end_top: start_top - ROW_RISE,
            elapsed_seconds: 0.0,
        });
    }
}

pub fn idle_row_color() -> Color {
    Color::srgb(0.16, 0.28, 0.76)
}

pub fn score_guess(guess: &str, target: &str) -> Vec<RuneMatchState> {
    let guess_chars: Vec<Option<char>> = guess.chars().map(Some).collect();
    score_guess_submission(&guess_chars, target)
}

pub fn score_guess_submission(guess: &[Option<char>], target: &str) -> Vec<RuneMatchState> {
    let target_chars: Vec<char> = target.chars().collect();
    let mut results = vec![RuneMatchState::Missing; guess.len()];
    let mut remaining_target_counts = HashMap::<char, usize>::new();

    for (index, target_char) in target_chars.iter().copied().enumerate() {
        if guess.get(index) == Some(&Some(target_char)) {
            results[index] = RuneMatchState::Correct;
        } else {
            *remaining_target_counts.entry(target_char).or_default() += 1;
        }
    }

    for (index, guess_char) in guess.iter().copied().enumerate() {
        if results[index] == RuneMatchState::Correct {
            continue;
        }
        let Some(guess_char) = guess_char else {
            continue;
        };
        let Some(count) = remaining_target_counts.get_mut(&guess_char) else {
            continue;
        };
        if *count == 0 {
            continue;
        }
        results[index] = RuneMatchState::Present;
        *count -= 1;
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_guess_handles_duplicate_letters() {
        assert_eq!(
            score_guess("aaccc", "abaca"),
            vec![
                RuneMatchState::Correct,
                RuneMatchState::Present,
                RuneMatchState::Missing,
                RuneMatchState::Correct,
                RuneMatchState::Missing,
            ]
        );
    }
}
