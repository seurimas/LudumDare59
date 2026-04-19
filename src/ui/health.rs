use bevy::prelude::*;

#[derive(Resource)]
pub struct PlayerHealthState {
    pub hp: u32,
    pub max: u32,
}

impl Default for PlayerHealthState {
    fn default() -> Self {
        Self { hp: 78, max: 100 }
    }
}

#[derive(Component)]
pub struct NpcHealthState {
    pub hp: u32,
    pub max: u32,
}

impl Default for NpcHealthState {
    fn default() -> Self {
        Self { hp: 60, max: 100 }
    }
}

pub fn configure_health(app: &mut App) {
    app.init_resource::<PlayerHealthState>();
}
