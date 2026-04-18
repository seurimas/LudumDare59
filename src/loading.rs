use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use rand::seq::SliceRandom;
use rand::thread_rng;

use crate::GameAssets;
use crate::GameState;

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct RuneRevealScreen;

#[derive(Component)]
struct RevealedRune {
    reveal_order: usize,
}

#[derive(Resource, Default)]
struct RuneRevealProgress {
    elapsed_seconds: f32,
}

const RUNE_COUNT: usize = 5;
const RUNE_VARIANTS: usize = 24;
const REVEAL_DURATION_SECONDS: f32 = 0.2;
const RUNE_SPACING: f32 = 48.0;

pub fn configure_loading(app: &mut App) {
    app.init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::RuneReveal)
                .load_collection::<GameAssets>(),
        )
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::Loading), spawn_loading_screen)
        .add_systems(OnExit(GameState::Loading), despawn_loading_screen)
        .add_systems(OnEnter(GameState::RuneReveal), spawn_rune_reveal_screen)
        .add_systems(
            Update,
            animate_rune_reveal.run_if(in_state(GameState::RuneReveal)),
        )
        .add_systems(OnExit(GameState::RuneReveal), despawn_rune_reveal_screen);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn spawn_loading_screen(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            LoadingScreen,
        ))
        .with_child((
            Text::new("Loading..."),
            TextFont {
                font_size: 50.0,
                ..default()
            },
        ));
}

fn despawn_loading_screen(mut commands: Commands, query: Query<Entity, With<LoadingScreen>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn spawn_rune_reveal_screen(mut commands: Commands, game_assets: Res<GameAssets>) {
    let mut rune_indices: Vec<usize> = (0..RUNE_VARIANTS).collect();
    rune_indices.shuffle(&mut thread_rng());

    commands.insert_resource(RuneRevealProgress::default());

    let offset = (RUNE_COUNT as f32 - 1.0) * 0.5;
    for (i, rune_index) in rune_indices.into_iter().take(RUNE_COUNT).enumerate() {
        commands.spawn((
            Sprite {
                image: game_assets.futhark.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: game_assets.futhark_layout.clone(),
                    index: rune_index,
                }),
                color: Color::srgba(1.0, 1.0, 1.0, 0.0),
                ..default()
            },
            Transform::from_xyz((i as f32 - offset) * RUNE_SPACING, 0.0, 0.0),
            RevealedRune { reveal_order: i },
            RuneRevealScreen,
        ));
    }
}

fn animate_rune_reveal(
    mut sprites: Query<(&RevealedRune, &mut Sprite)>,
    mut progress: ResMut<RuneRevealProgress>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    progress.elapsed_seconds += time.delta_secs();

    for (revealed_rune, mut sprite) in &mut sprites {
        let alpha = rune_alpha(progress.elapsed_seconds, revealed_rune.reveal_order);
        sprite.color = sprite.color.with_alpha(alpha);
    }

    if progress.elapsed_seconds >= total_reveal_duration_seconds() {
        next_state.set(GameState::Ready);
    }
}

fn despawn_rune_reveal_screen(
    mut commands: Commands,
    query: Query<Entity, With<RuneRevealScreen>>,
) {
    for entity in &query {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<RuneRevealProgress>();
}

fn rune_alpha(elapsed_seconds: f32, reveal_order: usize) -> f32 {
    let reveal_start = reveal_order as f32 * REVEAL_DURATION_SECONDS;
    ((elapsed_seconds - reveal_start) / REVEAL_DURATION_SECONDS).clamp(0.0, 1.0)
}

fn total_reveal_duration_seconds() -> f32 {
    RUNE_COUNT as f32 * REVEAL_DURATION_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rune_alpha_respects_staggered_reveal_timing() {
        assert_eq!(rune_alpha(0.0, 0), 0.0);
        assert_eq!(rune_alpha(0.2, 0), 1.0);

        assert_eq!(rune_alpha(0.2, 1), 0.0);
        assert!((rune_alpha(0.3, 1) - 0.5).abs() < f32::EPSILON);
        assert_eq!(rune_alpha(0.4, 1), 1.0);
    }

    #[test]
    fn total_reveal_duration_matches_five_runes() {
        assert!((total_reveal_duration_seconds() - 1.0).abs() < f32::EPSILON);
    }
}
