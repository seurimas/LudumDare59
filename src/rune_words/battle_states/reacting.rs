use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;
use std::collections::HashSet;

use crate::rune_words::battle::{
    ACTIVE_ROW_TOP, BattlePhase, BattleRowMotion, BattleRuneSlot, BattleSet, BattleState, ROW_RISE,
    RowResolved, RuneMatchState, collect_guess_submission, push_all_non_active_slots_up,
    reset_battle_state, score_guess_submission, spawn_battle_row,
};
use crate::rune_words::rune_slots::{
    ActiveRuneSlot, EnterActiveRuneWord, RuneSlot, RuneSlotBackground,
};
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartReacting {
    pub target: dictionary::Futharkation,
    pub time_limit: f32,
}

#[derive(bevy::ecs::message::Message, Clone, Debug, Default)]
pub struct ReactingSucceeded;

#[derive(bevy::ecs::message::Message, Clone, Debug, Default)]
pub struct ReactingFailed;

#[derive(Resource, Default)]
pub struct ReactingData {
    pub target: Option<dictionary::Futharkation>,
    pub time_limit: f32,
    pub elapsed: f32,
    pub active: bool,
    pub timer_display: Option<Entity>,
}

#[derive(Component)]
pub struct ReactingTimerDisplay;

pub fn configure_reacting(app: &mut App) {
    app.init_resource::<ReactingData>();
    app.add_message::<StartReacting>();
    app.add_message::<ReactingSucceeded>();
    app.add_message::<ReactingFailed>();
    app.add_systems(
        Update,
        (
            start_reacting,
            tick_reacting_timer.run_if(is_reacting),
            score_reacting_row_on_enter.run_if(is_reacting),
        )
            .chain(),
    );
    app.add_systems(
        Update,
        on_reacting_row_resolved
            .run_if(is_reacting)
            .in_set(BattleSet::PostAnimation),
    );
}

fn is_reacting(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Reacting)
}

fn start_reacting(
    mut commands: Commands,
    mut start_events: MessageReader<StartReacting>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut reacting_data: ResMut<ReactingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(StartReacting { target, time_limit }) = start_events.read().last().cloned() else {
        return;
    };

    // Despawn previous timer display if any.
    if let Some(display) = reacting_data.timer_display.take() {
        commands.entity(display).despawn();
    }

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Reacting;

    let rune_count = target.letters.chars().count();
    reacting_data.target = Some(target);
    reacting_data.time_limit = time_limit;
    reacting_data.elapsed = 0.0;
    reacting_data.active = true;

    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        rune_count,
        ACTIVE_ROW_TOP,
    );
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();

    let timer_entity = commands
        .spawn((
            ReactingTimerDisplay,
            Text::new(format!("{:.1}", time_limit)),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(crate::rune_words::battle::ROW_CENTER_LEFT - 20.0),
                top: Val::Px(ACTIVE_ROW_TOP + 64.0),
                ..default()
            },
        ))
        .id();
    reacting_data.timer_display = Some(timer_entity);
}

fn tick_reacting_timer(
    time: Res<Time>,
    mut reacting_data: ResMut<ReactingData>,
    mut battle_state: ResMut<BattleState>,
    mut timer_text: Query<&mut Text, With<ReactingTimerDisplay>>,
    mut failed: MessageWriter<ReactingFailed>,
) {
    if !reacting_data.active {
        return;
    }

    reacting_data.elapsed += time.delta_secs();
    let remaining = (reacting_data.time_limit - reacting_data.elapsed).max(0.0);

    for mut text in &mut timer_text {
        **text = format!("{:.1}", remaining);
    }

    if reacting_data.elapsed >= reacting_data.time_limit {
        reacting_data.active = false;
        battle_state.phase = BattlePhase::Idle;
        failed.write(ReactingFailed);
    }
}

fn score_reacting_row_on_enter(
    mut commands: Commands,
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut reacting_data: ResMut<ReactingData>,
    rune_slots: Query<&RuneSlot>,
    battle_slots: Query<&BattleRuneSlot>,
    slot_nodes: Query<(Entity, &Node), With<BattleRuneSlot>>,
    slot_children: Query<&Children>,
    mut backgrounds: Query<&mut RuneSlotBackground>,
    mut succeeded: MessageWriter<ReactingSucceeded>,
) {
    if enter_events.is_empty() {
        return;
    }
    enter_events.clear();

    if battle_state.pending_resolved_row.is_some() || !reacting_data.active {
        return;
    }

    let Some(target) = reacting_data.target.clone() else {
        return;
    };
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

    let active_set: HashSet<Entity> = battle_state.active_row_slots.iter().copied().collect();
    let results = score_guess_submission(&guess, &target.letters);

    let all_correct = results.iter().all(|r| matches!(r, RuneMatchState::Correct));

    active_slot.entity = None;

    for (entity, result) in battle_state.active_row_slots.iter().copied().zip(results) {
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

    if all_correct {
        reacting_data.active = false;
        battle_state.phase = BattlePhase::Idle;
        succeeded.write(ReactingSucceeded);
    }

    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = Some(row_id);
    battle_state.pending_settle_frames = 1;
}

fn on_reacting_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    reacting_data: Res<ReactingData>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.is_empty() {
        return;
    }
    row_resolved.clear();

    // Only spawn another row if still active (not yet succeeded or timed out).
    if !reacting_data.active {
        active_slot.entity = None;
        return;
    }

    let Some(target) = reacting_data.target.clone() else {
        return;
    };

    let row = spawn_battle_row(
        &mut commands,
        &game_assets,
        battle_state.next_row_id,
        target.letters.chars().count(),
        ACTIVE_ROW_TOP,
    );
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}
