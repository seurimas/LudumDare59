use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::rune_slots::{
    RuneSlot, RuneSlotBackground, RuneSlotConfig, RuneSlotForegroundSet, RuneSlotLinks,
    spawn_rune_slot_flex, spawn_rune_word,
};
use crate::{GameAssets, futhark};

/// Legacy absolute-pixel constants — kept for backward-compat; will be removed once UATs pass.
pub const LEGACY_ACTIVE_ROW_TOP: f32 = 236.0;
pub const ROW_RISE: f32 = 72.0;
pub const LEGACY_ROW_LEFT: f32 = 36.0;
pub const LEGACY_SLOT_SPACING: f32 = 68.0;
pub const LEGACY_SLOT_SIZE: f32 = 48.0;
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

impl BattlePhase {
    pub fn phase_index(&self) -> usize {
        match self {
            Self::Idle => 0,
            Self::Binding => 0,
            Self::Acting => 1,
            Self::Reacting => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcType {
    Goblin,
    Robed,
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
    pub npc_type: Option<NpcType>,
    pub active_row_slots: Vec<Entity>,
    pub pending_resolved_row: Option<u32>,
    pub pending_settle_frames: u8,
    pub next_row_id: u32,
    pub resolved_rows: usize,
}

#[derive(Clone)]
struct QueuedLetterPlaybackStep {
    slot_entity: Entity,
    letter: char,
    match_state: RuneMatchState,
    handle: Option<Handle<AudioSource>>,
    duration_seconds: f32,
}

#[derive(Clone)]
struct QueuedRowGrading {
    row_id: u32,
    row_slots: Vec<Entity>,
    slot_results: Vec<(Entity, RuneMatchState)>,
    steps: Vec<QueuedLetterPlaybackStep>,
    current_step: usize,
    elapsed_seconds: f32,
    current_step_duration_seconds: f32,
    started: bool,
}

#[derive(Resource, Default)]
pub struct PendingRowGrading {
    current: Option<QueuedRowGrading>,
}

impl PendingRowGrading {
    pub fn is_active(&self) -> bool {
        self.current.is_some()
    }
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

#[derive(bevy::ecs::message::Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowLetterGraded {
    pub row_id: u32,
    pub letter: char,
    pub match_state: RuneMatchState,
}

pub fn configure_battle(app: &mut App) {
    app.init_resource::<BattleState>();
    app.init_resource::<PendingRowGrading>();
    app.add_message::<RowResolved>();
    app.add_message::<RowLetterGraded>();
    app.configure_sets(
        Update,
        BattleSet::CheckAnimations.before(BattleSet::PostAnimation),
    );
    app.add_systems(
        Update,
        (
            tick_pending_row_grading,
            animate_resolved_rows,
            check_row_animation_done,
        )
            .chain()
            .in_set(BattleSet::CheckAnimations),
    );
    super::battle_states::configure_battle_states(app);
}

fn clip_duration(p: &crate::audio::ProcessedAudio) -> f32 {
    if p.channels == 0 || p.sample_rate == 0 {
        return 0.0;
    }
    p.samples.len() as f32 / (p.channels as f32 * p.sample_rate as f32)
}

pub fn queue_row_grading_playback(
    row_id: u32,
    row_slots: &[Entity],
    guess: &[Option<char>],
    results: &[RuneMatchState],
    pending: &mut PendingRowGrading,
    prebaked_audio: Option<&crate::futhark::PrebakedFutharkConversationalAudio>,
    baked_samples: Option<&crate::futhark::BakedAudioSamples>,
) {
    let slot_results: Vec<(Entity, RuneMatchState)> = row_slots
        .iter()
        .copied()
        .zip(results.iter().copied())
        .map(|(entity, result)| (entity, result))
        .collect();

    let mut steps = Vec::new();
    for ((entity, guess_char), result) in row_slots
        .iter()
        .copied()
        .zip(guess.iter().copied())
        .zip(results.iter().copied())
    {
        let Some(letter) = guess_char else {
            continue;
        };

        let Some(rune_index) = futhark::letter_to_index(letter) else {
            continue;
        };

        let handle = prebaked_audio
            .and_then(|audio| audio.handles_by_index.get(rune_index))
            .and_then(|handles| handles.first())
            .cloned();
        let duration_seconds = baked_samples
            .and_then(|samples| samples.conversational.get(rune_index))
            .and_then(|clips| clips.first())
            .map(clip_duration)
            .unwrap_or(0.0);

        steps.push(QueuedLetterPlaybackStep {
            slot_entity: entity,
            letter,
            match_state: result,
            handle,
            duration_seconds,
        });
    }

    pending.current = Some(QueuedRowGrading {
        row_id,
        row_slots: row_slots.to_vec(),
        slot_results,
        steps,
        current_step: 0,
        elapsed_seconds: 0.0,
        current_step_duration_seconds: 0.0,
        started: false,
    });
}

fn tint_slot(
    slot_entity: Entity,
    color: Color,
    slot_children: &Query<&Children>,
    backgrounds: &mut Query<&mut RuneSlotBackground>,
) {
    if let Ok(children) = slot_children.get(slot_entity) {
        for child in children.iter() {
            if let Ok(mut bg) = backgrounds.get_mut(child) {
                bg.base_color = color;
            }
        }
    }
}

fn begin_row_resolution_animation(
    _commands: &mut Commands,
    battle_state: &mut BattleState,
    row_id: u32,
    _row_slots: &[Entity],
) {
    // No row-rise animation in the flex layout: just mark the row for resolution.
    // `check_row_animation_done` will fire `RowResolved` immediately (no BattleRowMotion set).
    battle_state.pending_resolved_row = Some(row_id);
    battle_state.pending_settle_frames = 1;
}

fn tick_pending_row_grading(
    mut commands: Commands,
    time: Res<Time>,
    mut battle_state: ResMut<BattleState>,
    mut pending: ResMut<PendingRowGrading>,
    mut row_letter_graded: MessageWriter<RowLetterGraded>,
    slot_children: Query<&Children>,
    mut backgrounds: Query<&mut RuneSlotBackground>,
) {
    if battle_state.pending_resolved_row.is_some() {
        return;
    }

    let should_finalize = {
        let Some(queued) = pending.current.as_mut() else {
            return;
        };

        if queued.steps.is_empty() {
            true
        } else {
            if !queued.started {
                if let Some(step) = queued.steps.first() {
                    tint_slot(
                        step.slot_entity,
                        step.match_state.background_color(),
                        &slot_children,
                        &mut backgrounds,
                    );
                    row_letter_graded.write(RowLetterGraded {
                        row_id: queued.row_id,
                        letter: step.letter,
                        match_state: step.match_state,
                    });
                    if let Some(handle) = step.handle.clone() {
                        commands.spawn((
                            AudioPlayer::<AudioSource>(handle),
                            PlaybackSettings::DESPAWN,
                        ));
                    }
                    queued.current_step_duration_seconds = step.duration_seconds;
                    queued.elapsed_seconds = 0.0;
                    queued.started = true;
                }
                return;
            }

            queued.elapsed_seconds += time.delta_secs();
            if queued.elapsed_seconds < queued.current_step_duration_seconds {
                return;
            }

            queued.current_step += 1;
            if queued.current_step >= queued.steps.len() {
                true
            } else {
                let step = &queued.steps[queued.current_step];
                tint_slot(
                    step.slot_entity,
                    step.match_state.background_color(),
                    &slot_children,
                    &mut backgrounds,
                );
                row_letter_graded.write(RowLetterGraded {
                    row_id: queued.row_id,
                    letter: step.letter,
                    match_state: step.match_state,
                });
                if let Some(handle) = step.handle.clone() {
                    commands.spawn((
                        AudioPlayer::<AudioSource>(handle),
                        PlaybackSettings::DESPAWN,
                    ));
                }
                queued.current_step_duration_seconds = step.duration_seconds;
                queued.elapsed_seconds = 0.0;
                return;
            }
        }
    };

    if !should_finalize {
        return;
    }

    let Some(queued) = pending.current.take() else {
        return;
    };

    for (entity, match_state) in queued.slot_results {
        tint_slot(
            entity,
            match_state.background_color(),
            &slot_children,
            &mut backgrounds,
        );
    }

    begin_row_resolution_animation(
        &mut commands,
        &mut battle_state,
        queued.row_id,
        &queued.row_slots,
    );
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
    let start_left = LEGACY_ROW_LEFT;
    let configs = (0..rune_count)
        .map(|index| RuneSlotConfig {
            left: Val::Px(start_left + index as f32 * LEGACY_SLOT_SPACING),
            top: Val::Px(top),
            size: LEGACY_SLOT_SIZE,
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

/// Spawn a battle row as flex children inside `container` (e.g. the `RuneSlotRow` UI node).
/// Uses `LEGACY_SLOT_SIZE` pixel dimensions but without absolute top/left positioning.
pub fn spawn_battle_row_in_container(
    commands: &mut Commands,
    game_assets: &GameAssets,
    row_id: u32,
    rune_count: usize,
    container: Entity,
) -> Vec<Entity> {
    let entities: Vec<Entity> = (0..rune_count)
        .map(|_| {
            spawn_rune_slot_flex(
                commands,
                game_assets,
                RuneSlotConfig {
                    size: LEGACY_SLOT_SIZE,
                    background_color: idle_row_color(),
                    foreground_set: RuneSlotForegroundSet::Primary,
                    initial_rune: None,
                    ..default()
                },
            )
        })
        .collect();

    let len = entities.len();
    for i in 0..len {
        let prev = if i > 0 { Some(entities[i - 1]) } else { None };
        let next = if i + 1 < len {
            Some(entities[i + 1])
        } else {
            None
        };
        commands
            .entity(entities[i])
            .insert(RuneSlotLinks { prev, next })
            .insert(BattleRuneSlot { row_id });
    }

    for &entity in &entities {
        commands.entity(container).add_child(entity);
    }

    entities
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
            _ => LEGACY_ACTIVE_ROW_TOP,
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
