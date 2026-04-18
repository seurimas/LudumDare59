use bevy::ecs::message::{MessageReader, MessageWriter};
use bevy::prelude::*;

use crate::dictionary::Futharkation;
use crate::rune_words::battle_states::acting::{StartActing};
use crate::rune_words::battle_states::binding::StartBinding;
use crate::rune_words::battle_states::reacting::{ReactingFailed, ReactingSucceeded, StartReacting};

#[derive(Resource, Default)]
pub struct WordBook {
    pub words: Vec<Futharkation>,
}

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct QuickTime(pub Futharkation);

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub struct FinalChallenge(pub Futharkation);

pub fn configure_battle_stages(app: &mut App) {
    app.init_resource::<WordBook>();
    app.add_message::<QuickTime>();
    app.add_message::<FinalChallenge>();
    app.add_systems(
        Update,
        (on_quicktime, on_reacting_resolved, on_final_challenge).chain(),
    );
}

fn on_quicktime(
    mut quicktime: MessageReader<QuickTime>,
    mut start_reacting: MessageWriter<StartReacting>,
) {
    for QuickTime(word) in quicktime.read() {
        start_reacting.write(StartReacting {
            target: word.clone(),
            time_limit: 10.0,
        });
    }
}

fn on_reacting_resolved(
    mut succeeded: MessageReader<ReactingSucceeded>,
    mut failed: MessageReader<ReactingFailed>,
    mut start_acting: MessageWriter<StartActing>,
    book: Res<WordBook>,
) {
    let any = !succeeded.is_empty() || !failed.is_empty();
    succeeded.clear();
    failed.clear();
    if any && !book.words.is_empty() {
        start_acting.write(StartActing {
            targets: book.words.clone(),
        });
    }
}

fn on_final_challenge(
    mut challenge: MessageReader<FinalChallenge>,
    mut start_binding: MessageWriter<StartBinding>,
) {
    for FinalChallenge(word) in challenge.read() {
        start_binding.write(StartBinding(word.clone()));
    }
}
