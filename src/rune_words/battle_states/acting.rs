use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use std::collections::HashSet;

use crate::rune_words::battle::{
    ACTIVE_ROW_TOP, BattlePhase, BattleRowMotion, BattleRuneSlot, BattleSet, BattleState,
    ROW_CENTER_LEFT, ROW_RISE, RowResolved, RuneMatchState, collect_guess_submission,
    push_all_non_active_slots_up, reset_battle_state, score_guess_submission, spawn_battle_row,
};
use crate::rune_words::rune_slots::{
    ActiveRuneSlot, EnterActiveRuneWord, RuneSlot, RuneSlotBackground,
};
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartActing {
    pub targets: Vec<dictionary::Futharkation>,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct ActingSucceeded {
    pub matched: dictionary::Futharkation,
    pub results: Vec<RuneMatchState>,
}

#[derive(Resource, Default)]
pub struct ActingData {
    pub targets: Vec<dictionary::Futharkation>,
    pub max_rune_count: usize,
    pub pending_success: bool,
}

#[derive(Component)]
pub struct ActingCountLabel;

pub fn configure_acting(app: &mut App) {
    app.init_resource::<ActingData>();
    app.add_message::<StartActing>();
    app.add_message::<ActingSucceeded>();
    app.add_systems(
        Update,
        (start_acting, score_acting_row_on_enter.run_if(is_acting)).chain(),
    );
    app.add_systems(
        Update,
        on_acting_row_resolved
            .run_if(is_acting)
            .in_set(BattleSet::PostAnimation),
    );
}

fn is_acting(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Acting)
}

fn start_acting(
    mut commands: Commands,
    mut start_events: MessageReader<StartActing>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut acting_data: ResMut<ActingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(StartActing { targets }) = start_events.read().last().cloned() else {
        return;
    };
    if targets.is_empty() {
        return;
    }

    let max_rune_count = targets
        .iter()
        .map(|t| t.letters.chars().count())
        .max()
        .unwrap_or(1);

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Acting;

    acting_data.targets = targets;
    acting_data.max_rune_count = max_rune_count;
    acting_data.pending_success = false;

    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        max_rune_count,
        ACTIVE_ROW_TOP,
    );
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

fn score_acting_row_on_enter(
    mut commands: Commands,
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut acting_data: ResMut<ActingData>,
    rune_slots: Query<&RuneSlot>,
    battle_slots: Query<&BattleRuneSlot>,
    slot_nodes: Query<(Entity, &Node), With<BattleRuneSlot>>,
    slot_children: Query<&Children>,
    mut backgrounds: Query<&mut RuneSlotBackground>,
) {
    if enter_events.is_empty() {
        return;
    }
    enter_events.clear();

    if battle_state.pending_resolved_row.is_some() {
        return;
    }

    let Some(guess) = collect_guess_submission(&battle_state.active_row_slots, &rune_slots) else {
        return;
    };
    let Some(first_entity) = battle_state.active_row_slots.first().copied() else {
        return;
    };
    let Ok(battle_slot) = battle_slots.get(first_entity) else {
        return;
    };
    let row_id = battle_slot.row_id;

    let Some((best_target, best_results)) = find_best_match(&guess, &acting_data.targets) else {
        return;
    };

    let correct = best_results
        .iter()
        .filter(|r| matches!(r, RuneMatchState::Correct))
        .count();
    let present = best_results
        .iter()
        .filter(|r| matches!(r, RuneMatchState::Present))
        .count();
    let missing = best_results
        .iter()
        .filter(|r| matches!(r, RuneMatchState::Missing))
        .count();

    let active_set: HashSet<Entity> = battle_state.active_row_slots.iter().copied().collect();
    active_slot.entity = None;

    let row_top = slot_nodes
        .get(first_entity)
        .map(|(_, n)| match n.top {
            Val::Px(t) => t,
            _ => ACTIVE_ROW_TOP,
        })
        .unwrap_or(ACTIVE_ROW_TOP);

    for (entity, result) in battle_state
        .active_row_slots
        .iter()
        .copied()
        .zip(best_results.iter().copied())
    {
        if let Ok(children) = slot_children.get(entity) {
            for child in children.iter() {
                if let Ok(mut bg) = backgrounds.get_mut(child) {
                    bg.base_color = result.background_color();
                }
            }
        }
        let start_top = match slot_nodes.get(entity).map(|(_, n)| n.top) {
            Ok(Val::Px(t)) => t,
            _ => ACTIVE_ROW_TOP,
        };
        commands.entity(entity).insert(BattleRowMotion {
            start_top,
            end_top: start_top - ROW_RISE,
            elapsed_seconds: 0.0,
        });
    }

    push_all_non_active_slots_up(&mut commands, &active_set, &slot_nodes);

    let label_top = row_top + 52.0;
    let label_entity = commands
        .spawn((
            ActingCountLabel,
            BattleRuneSlot { row_id },
            BattleRowMotion {
                start_top: label_top,
                end_top: label_top - ROW_RISE,
                elapsed_seconds: 0.0,
            },
            Text::new(format!("{}✓  {}~  {}✗", correct, present, missing)),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(ROW_CENTER_LEFT - 60.0),
                top: Val::Px(label_top),
                ..default()
            },
        ))
        .id();
    let _ = label_entity;

    acting_data.pending_success = correct >= 2;
    if acting_data.pending_success {
        acting_data.targets = vec![best_target]; // keep only the matched target for post-resolve
    }

    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = Some(row_id);
    battle_state.pending_settle_frames = 1;
}

fn on_acting_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    mut acting_data: ResMut<ActingData>,
    mut succeeded: MessageWriter<ActingSucceeded>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.is_empty() {
        return;
    }
    row_resolved.clear();

    if acting_data.pending_success {
        let matched =
            acting_data
                .targets
                .first()
                .cloned()
                .unwrap_or_else(|| dictionary::Futharkation {
                    word: String::new(),
                    letters: String::new(),
                });
        battle_state.phase = BattlePhase::Idle;
        succeeded.write(ActingSucceeded {
            matched,
            results: Vec::new(),
        });
        active_slot.entity = None;
        return;
    }

    // Failure: spawn another attempt with the same slot count.
    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        acting_data.max_rune_count,
        ACTIVE_ROW_TOP,
    );
    battle_state.phase = BattlePhase::Acting;
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

fn find_best_match(
    guess: &[Option<char>],
    targets: &[dictionary::Futharkation],
) -> Option<(dictionary::Futharkation, Vec<RuneMatchState>)> {
    targets
        .iter()
        .map(|target| {
            let results = score_guess_submission(guess, &target.letters);
            let correct = results
                .iter()
                .filter(|r| matches!(r, RuneMatchState::Correct))
                .count();
            let present = results
                .iter()
                .filter(|r| matches!(r, RuneMatchState::Present))
                .count();
            (target.clone(), results, correct, present)
        })
        .max_by_key(|(_, _, correct, present)| (*correct, *present))
        .map(|(target, results, _, _)| (target, results))
}
