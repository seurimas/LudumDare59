use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::rune_words::battle::{
    BattlePhase, BattleRuneSlot, BattleSet, BattleState, LEGACY_ACTIVE_ROW_TOP, LEGACY_ROW_LEFT,
    PendingRowGrading, RowResolved, RuneMatchState, collect_guess_submission,
    queue_row_grading_playback, reset_battle_state, score_guess_submission, spawn_battle_row,
    spawn_battle_row_in_container,
};
use crate::rune_words::battle_states::LastGradedWord;
use crate::rune_words::rune_slots::{ActiveRuneSlot, EnterActiveRuneWord, RuneSlot};
use crate::ui::inscribed::RuneSlotRow;
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
    pub pending_success: bool,
    pub timer_display: Option<Entity>,
    pub target_word_display: Option<Entity>,
}

#[derive(Component)]
pub struct ReactingTimerDisplay;

#[derive(Component)]
pub struct ReactingTargetWordDisplay;

pub fn configure_reacting(app: &mut App) {
    app.init_resource::<ReactingData>();
    app.add_message::<StartReacting>();
    app.add_message::<ReactingSucceeded>();
    app.add_message::<ReactingFailed>();
    app.add_systems(Update, cleanup_reacting_overlays_outside_phase);
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
    row_slot_container: Query<Entity, With<RuneSlotRow>>,
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
    if let Some(display) = reacting_data.target_word_display.take() {
        commands.entity(display).despawn();
    }

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Reacting;

    let target_word = target.word.clone();
    let rune_count = target.letters.chars().count();
    reacting_data.target = Some(target);
    reacting_data.time_limit = time_limit;
    reacting_data.elapsed = 0.0;
    reacting_data.active = true;
    reacting_data.pending_success = false;

    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };
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
                left: Val::Px(LEGACY_ROW_LEFT),
                top: Val::Px(LEGACY_ACTIVE_ROW_TOP + 64.0),
                ..default()
            },
        ))
        .id();
    reacting_data.timer_display = Some(timer_entity);

    let target_word_entity = commands
        .spawn((
            ReactingTargetWordDisplay,
            Text::new(target_word),
            TextFont {
                font_size: 64.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(24.0),
                bottom: Val::Px(24.0),
                ..default()
            },
        ))
        .id();
    reacting_data.target_word_display = Some(target_word_entity);
}

fn cleanup_reacting_overlays_outside_phase(
    mut commands: Commands,
    battle_state: Res<BattleState>,
    overlays: Query<Entity, Or<(With<ReactingTimerDisplay>, With<ReactingTargetWordDisplay>)>>,
) {
    if matches!(battle_state.phase, BattlePhase::Reacting) {
        return;
    }

    for entity in overlays.iter() {
        commands.entity(entity).despawn();
    }
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
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut reacting_data: ResMut<ReactingData>,
    mut pending_grading: ResMut<PendingRowGrading>,
    rune_slots: Query<&RuneSlot>,
    battle_slots: Query<&BattleRuneSlot>,
    prebaked_audio: Option<Res<crate::futhark::PrebakedFutharkConversationalAudio>>,
    baked_samples: Option<Res<crate::futhark::BakedAudioSamples>>,
) {
    if enter_events.is_empty() {
        return;
    }
    enter_events.clear();

    if battle_state.pending_resolved_row.is_some()
        || pending_grading.is_active()
        || !reacting_data.active
    {
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

    let results = score_guess_submission(&guess, &target.letters);

    let all_correct = results.iter().all(|r| matches!(r, RuneMatchState::Correct));

    queue_row_grading_playback(
        row_id,
        &battle_state.active_row_slots,
        &guess,
        &results,
        &mut pending_grading,
        prebaked_audio.as_deref(),
        baked_samples.as_deref(),
    );

    active_slot.entity = None;
    reacting_data.pending_success = all_correct;
    if all_correct {
        reacting_data.active = false;
    }

    battle_state.active_row_slots.clear();
}

fn on_reacting_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    mut reacting_data: ResMut<ReactingData>,
    mut succeeded: MessageWriter<ReactingSucceeded>,
    existing_battle_slots: Query<Entity, With<BattleRuneSlot>>,
    row_slot_container: Query<Entity, With<RuneSlotRow>>,
    mut last_graded_word: ResMut<LastGradedWord>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.read().count() == 0 {
        return;
    }

    // Record word for the ledger.
    last_graded_word.word = reacting_data.target.as_ref().map(|t| t.word.clone());

    if reacting_data.pending_success {
        reacting_data.pending_success = false;
        battle_state.phase = BattlePhase::Idle;
        active_slot.entity = None;
        succeeded.write(ReactingSucceeded);
        return;
    }

    // Only spawn another row if still active (not yet succeeded or timed out).
    if !reacting_data.active {
        active_slot.entity = None;
        return;
    }

    let Some(target) = reacting_data.target.clone() else {
        return;
    };

    // Despawn old slots, spawn fresh row.
    for entity in existing_battle_slots.iter() {
        commands.entity(entity).despawn();
    }

    let rune_count = target.letters.chars().count();
    let row = if let Some(container) = row_slot_container.iter().next() {
        spawn_battle_row_in_container(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            container,
        )
    } else {
        spawn_battle_row(
            &mut commands,
            &game_assets,
            battle_state.next_row_id,
            rune_count,
            LEGACY_ACTIVE_ROW_TOP,
        )
    };
    battle_state.next_row_id += 1;
    battle_state.active_row_slots = row.clone();
    active_slot.entity = row.first().copied();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::rune_words::battle::configure_battle;
    use crate::rune_words::rune_slots::{EnterActiveRuneWord, RuneSlot, configure_rune_slots};
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure_rune_slots(&mut app);
        configure_battle(&mut app);
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            100,
        )));
        app.insert_resource(crate::GameAssets {
            futhark: Handle::default(),
            futhark_layout: Handle::default(),
            futhark_sounds: Vec::new(),
            futhark_sound_params: Handle::default(),
            futhark_conversational_params: Handle::default(),
            backdrop: Handle::default(),
            parchment_tile: Handle::default(),
            corner_bracket: Handle::default(),
            vignette: Handle::default(),
            sigils: Handle::default(),
            sigils_layout: Handle::default(),
            goblin: Handle::default(),
            goblin_layout: Handle::default(),
            robed: Handle::default(),
            robed_layout: Handle::default(),
            font_cormorant_unicase_semibold: Handle::default(),
            font_cormorant_unicase_bold: Handle::default(),
            font_cormorant_garamond_italic: Handle::default(),
            font_im_fell_sc: Handle::default(),
            font_unifraktur: Handle::default(),
        });
        app
    }

    fn futha(word: &str, letters: &str) -> dictionary::Futharkation {
        dictionary::Futharkation {
            word: word.to_string(),
            letters: letters.to_string(),
        }
    }

    fn fill_active_row(app: &mut App, guess: &str) {
        let slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        for (entity, letter) in slots.into_iter().zip(guess.chars()) {
            app.world_mut()
                .entity_mut(entity)
                .get_mut::<RuneSlot>()
                .expect("slot")
                .rune_index = futhark::letter_to_index(letter);
        }
    }

    #[test]
    fn start_reacting_sets_phase_and_spawns_slots() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartReacting {
            target: futha("fable", "fut"),
            time_limit: 10.0,
        });
        app.update();

        let state = app.world().resource::<BattleState>();
        assert_eq!(state.phase, BattlePhase::Reacting);
        assert_eq!(
            state.active_row_slots.len(),
            3,
            "one slot per rune in target"
        );

        let data = app.world().resource::<ReactingData>();
        assert!(data.active);
        assert_eq!(data.time_limit, 10.0);
        assert_eq!(data.elapsed, 0.0);
    }

    #[test]
    fn reacting_timer_advances_each_frame() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartReacting {
            target: futha("fable", "fut"),
            time_limit: 10.0,
        });
        app.update();
        app.update(); // one 100ms tick

        let data = app.world().resource::<ReactingData>();
        assert!(data.elapsed > 0.0, "elapsed should increase after a frame");
    }

    #[test]
    fn reacting_timeout_sends_failed_and_transitions_to_idle() {
        let mut app = make_test_app();
        // 100ms time step × 5 updates = 500ms; set limit to 0.3s so it expires
        app.world_mut().write_message(StartReacting {
            target: futha("fa", "fu"),
            time_limit: 0.3,
        });
        app.update();

        // Run enough frames to exceed the 0.3s limit (each frame = 100ms)
        for _ in 0..5 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "phase should be Idle after timeout"
        );

        let data = app.world().resource::<ReactingData>();
        assert!(!data.active, "timer should be inactive after timeout");
    }

    #[test]
    fn reacting_correct_guess_sends_succeeded_and_transitions_to_idle() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartReacting {
            target: futha("fable", "fut"),
            time_limit: 10.0,
        });
        app.update();

        fill_active_row(&mut app, "fut");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        // Let animation complete
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "phase should be Idle after correct guess"
        );

        let data = app.world().resource::<ReactingData>();
        assert!(!data.active, "should be inactive after success");
    }

    #[test]
    fn reacting_wrong_guess_stays_active_and_spawns_new_row() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartReacting {
            target: futha("fable", "fut"),
            time_limit: 10.0,
        });
        app.update();

        let first_row = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        fill_active_row(&mut app, "fxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Reacting,
            "should remain Reacting after wrong guess"
        );
        assert_eq!(state.active_row_slots.len(), 3, "new row should be spawned");
        assert_ne!(
            state.active_row_slots, first_row,
            "new row entities should differ from first row"
        );
    }

    #[test]
    fn reacting_wrong_guess_increments_resolved_count() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartReacting {
            target: futha("fable", "fut"),
            time_limit: 10.0,
        });
        app.update();

        let first_row = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        fill_active_row(&mut app, "fxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(state.resolved_rows, 1, "resolved row count should be 1");
        assert_ne!(
            state.active_row_slots, first_row,
            "new row entities should differ from first row"
        );
    }
}
