use std::f32::consts::TAU;

use bevy::prelude::*;

use crate::GameState;

#[derive(Resource, Default)]
pub struct BattleUiClock {
    pub elapsed: f32,
}

pub fn configure_clock(app: &mut App) {
    app.init_resource::<BattleUiClock>();
    app.add_systems(Update, tick_clock.run_if(in_state(GameState::Ready)));
}

fn tick_clock(mut clock: ResMut<BattleUiClock>, time: Res<Time>) {
    clock.elapsed += time.delta_secs();
}

pub fn wave(clock: f32, period: f32, lo: f32, hi: f32) -> f32 {
    let t = (clock % period) / period;
    lo + (hi - lo) * (0.5 - 0.5 * (t * TAU).cos())
}
