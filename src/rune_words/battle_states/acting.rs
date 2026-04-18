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

pub(crate) fn find_best_match(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::rune_words::battle::{ACTIVE_ROW_TOP, ROW_RISE, configure_battle};
    use crate::rune_words::rune_slots::{RuneSlot, configure_rune_slots};
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
        });
        app
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

    fn futha(word: &str, letters: &str) -> dictionary::Futharkation {
        dictionary::Futharkation {
            word: word.to_string(),
            letters: letters.to_string(),
        }
    }

    // --- unit tests for find_best_match ---

    #[test]
    fn find_best_match_returns_none_for_empty_targets() {
        let guess: Vec<Option<char>> = vec![Some('f'), Some('u')];
        assert!(find_best_match(&guess, &[]).is_none());
    }

    #[test]
    fn find_best_match_picks_target_with_most_correct() {
        let guess: Vec<Option<char>> = vec![Some('f'), Some('u'), Some('t')];
        let targets = vec![futha("a", "fxx"), futha("b", "fut")];
        let (matched, results) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "b");
        assert!(results.iter().all(|r| matches!(r, RuneMatchState::Correct)));
    }

    #[test]
    fn find_best_match_breaks_ties_by_present_count() {
        // "fax" vs "fut": guess "fut" → "fax" gives 1 correct (f) + 0 present,
        // "fut" gives 3 correct. Let's test present tiebreaker instead:
        // guess "fab", targets "fax" (2 correct: f,a) vs "fbx" (1 correct: f, 1 present: b)
        let guess: Vec<Option<char>> = vec![Some('f'), Some('a'), Some('b')];
        let targets = vec![futha("fax", "fax"), futha("fbx", "fbx")];
        let (matched, _) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "fax");
    }

    #[test]
    fn find_best_match_prefers_correct_over_present() {
        // guess "abc", target1 "abc" (3 correct), target2 "bca" (0 correct, 3 present)
        let guess: Vec<Option<char>> = vec![Some('a'), Some('b'), Some('c')];
        let targets = vec![futha("bca", "bca"), futha("abc", "abc")];
        let (matched, _) = find_best_match(&guess, &targets).unwrap();
        assert_eq!(matched.word, "abc");
    }

    // --- app tests ---

    #[test]
    fn start_acting_spawns_slots_for_longest_target() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![
                futha("aa", "fu"),
                futha("bbbbb", "futar"),
                futha("ccc", "fut"),
            ],
        });
        app.update();
        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.active_row_slots.len(),
            5,
            "slots = longest target length"
        );
        assert_eq!(state.phase, BattlePhase::Acting);
    }

    #[test]
    fn start_acting_ignores_empty_targets() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartActing { targets: vec![] });
        app.update();
        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "empty targets should not start acting"
        );
    }

    #[test]
    fn acting_two_correct_marks_pending_success() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(
            acting_data.pending_success,
            "3 correct runes should set pending_success"
        );
    }

    #[test]
    fn acting_fewer_than_two_correct_does_not_set_pending_success() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        // only 1 correct ('f')
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        let acting_data = app.world().resource::<ActingData>();
        assert!(!acting_data.pending_success);
    }

    #[test]
    fn acting_success_sends_succeeded_and_transitions_to_idle() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        fill_active_row(&mut app, "futxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        // advance past animation
        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Idle,
            "phase should return to Idle after success"
        );
    }

    #[test]
    fn acting_failure_spawns_new_row_and_stays_acting() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        // only 1 correct → failure
        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        for _ in 0..10 {
            app.update();
        }

        let state = app.world().resource::<BattleState>();
        assert_eq!(
            state.phase,
            BattlePhase::Acting,
            "phase should stay Acting after failure"
        );
        assert_eq!(state.active_row_slots.len(), 5, "new row should be spawned");
    }

    #[test]
    fn acting_failure_pushes_previous_row_up() {
        let mut app = make_test_app();
        app.world_mut().write_message(StartActing {
            targets: vec![futha("fable", "futar")],
        });
        app.update();

        let first_row = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();

        fill_active_row(&mut app, "fxxxx");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..10 {
            app.update();
        }

        let top = app.world().entity(first_row[0]).get::<Node>().unwrap().top;
        assert_eq!(top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE));
    }
}
