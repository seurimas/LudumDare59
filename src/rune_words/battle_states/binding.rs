use bevy::ecs::message::MessageReader;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::rune_words::battle::{
    ACTIVE_ROW_TOP, BattlePhase, BattleRowMotion, BattleRuneSlot, BattleSet, BattleState, ROW_RISE,
    RowResolved, collect_guess_submission, push_all_non_active_slots_up, reset_battle_state,
    score_guess_submission, spawn_battle_row,
};
use crate::rune_words::rune_slots::{
    ActiveRuneSlot, EnterActiveRuneWord, RuneSlot, RuneSlotBackground,
};
use crate::{GameAssets, dictionary};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct StartBinding(pub dictionary::Futharkation);

#[derive(Resource, Default)]
pub struct BindingData {
    pub target: Option<dictionary::Futharkation>,
}

pub fn configure_binding(app: &mut App) {
    app.init_resource::<BindingData>();
    app.add_message::<StartBinding>();
    app.add_systems(
        Update,
        (start_binding, score_binding_row_on_enter.run_if(is_binding)).chain(),
    );
    app.add_systems(
        Update,
        on_binding_row_resolved
            .run_if(is_binding)
            .in_set(BattleSet::PostAnimation),
    );
}

fn is_binding(state: Res<BattleState>) -> bool {
    matches!(state.phase, BattlePhase::Binding)
}

fn start_binding(
    mut commands: Commands,
    mut start_events: MessageReader<StartBinding>,
    game_assets: Option<Res<GameAssets>>,
    existing_rows: Query<Entity, With<BattleRuneSlot>>,
    mut battle_state: ResMut<BattleState>,
    mut binding_data: ResMut<BindingData>,
    mut active_slot: ResMut<ActiveRuneSlot>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    let Some(StartBinding(target)) = start_events.read().last().cloned() else {
        return;
    };

    reset_battle_state(&mut commands, &mut battle_state, existing_rows.iter());
    battle_state.phase = BattlePhase::Binding;
    binding_data.target = Some(target.clone());

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

fn score_binding_row_on_enter(
    mut commands: Commands,
    mut enter_events: MessageReader<EnterActiveRuneWord>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    binding_data: Res<BindingData>,
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

    let Some(target) = binding_data.target.clone() else {
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

    battle_state.active_row_slots.clear();
    battle_state.pending_resolved_row = Some(row_id);
    battle_state.pending_settle_frames = 1;
}

fn on_binding_row_resolved(
    mut commands: Commands,
    game_assets: Option<Res<GameAssets>>,
    mut battle_state: ResMut<BattleState>,
    mut active_slot: ResMut<ActiveRuneSlot>,
    mut row_resolved: MessageReader<RowResolved>,
    binding_data: Res<BindingData>,
) {
    let Some(game_assets) = game_assets else {
        return;
    };
    if row_resolved.is_empty() {
        return;
    }
    row_resolved.clear();

    let Some(target) = binding_data.target.clone() else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::futhark;
    use crate::rune_words::battle::{ACTIVE_ROW_TOP, ROW_RISE, RuneMatchState, configure_battle};
    use crate::rune_words::rune_slots::configure_rune_slots;
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
        let active_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        for (entity, letter) in active_row_slots.into_iter().zip(guess.chars()) {
            app.world_mut()
                .entity_mut(entity)
                .get_mut::<RuneSlot>()
                .expect("slot")
                .rune_index = futhark::letter_to_index(letter);
        }
    }

    #[test]
    fn start_binding_spawns_one_slot_per_target_rune() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();
        let battle_state = app.world().resource::<BattleState>();
        assert_eq!(battle_state.active_row_slots.len(), 5);
        assert!(battle_state.pending_resolved_row.is_none());
    }

    #[test]
    fn submit_guess_colors_row_and_spawns_fresh_row_after_animation() {
        let mut app = make_test_app();
        let target = dictionary::Futharkation {
            word: "fable".to_string(),
            letters: "futar".to_string(),
        };
        app.world_mut().write_message(StartBinding(target));
        app.update();

        let submitted_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        fill_active_row(&mut app, "fukkk");
        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();

        assert!(
            app.world()
                .resource::<BattleState>()
                .pending_resolved_row
                .is_some(),
            "row should be animating after Enter"
        );

        let first_bg_color = {
            let world = app.world();
            let children = world
                .entity(*submitted_row_slots.first().unwrap())
                .get::<Children>()
                .unwrap();
            let mut color = None;
            for child in children.iter() {
                if let Some(bg) = world.entity(child).get::<RuneSlotBackground>() {
                    color = Some(bg.base_color);
                    break;
                }
            }
            color.unwrap()
        };
        assert_eq!(first_bg_color, RuneMatchState::Correct.background_color());

        for _ in 0..5 {
            app.update();
        }
        let battle_state = app.world().resource::<BattleState>();
        assert!(battle_state.pending_resolved_row.is_none());
        assert_eq!(battle_state.active_row_slots.len(), 5);
    }

    #[test]
    fn incomplete_submission_pushes_previous_rows_up() {
        let mut app = make_test_app();
        app.world_mut()
            .write_message(StartBinding(dictionary::Futharkation {
                word: "fable".to_string(),
                letters: "futar".to_string(),
            }));
        app.update();

        let first_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        app.world_mut()
            .entity_mut(first_row_slots[0])
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = futhark::letter_to_index('f');

        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..5 {
            app.update();
        }

        let second_row_slots = app
            .world()
            .resource::<BattleState>()
            .active_row_slots
            .clone();
        app.world_mut()
            .entity_mut(second_row_slots[0])
            .get_mut::<RuneSlot>()
            .unwrap()
            .rune_index = futhark::letter_to_index('u');

        app.world_mut().write_message(EnterActiveRuneWord);
        app.update();
        for _ in 0..5 {
            app.update();
        }

        let first_top = app
            .world()
            .entity(first_row_slots[0])
            .get::<Node>()
            .unwrap()
            .top;
        let second_top = app
            .world()
            .entity(second_row_slots[0])
            .get::<Node>()
            .unwrap()
            .top;
        assert_eq!(first_top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE * 2.0));
        assert_eq!(second_top, Val::Px(ACTIVE_ROW_TOP - ROW_RISE));
    }
}
