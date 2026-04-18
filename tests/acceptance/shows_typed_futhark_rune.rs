use LudumDare59::{GameAssets, GameState, acceptance, configure_app, configure_loading, futhark};
use bevy::ecs::message::MessageReader;
use bevy::prelude::*;

const TEST_ID: u8 = 4;

#[derive(Component)]
struct TypedRuneDisplay;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    futhark::configure_futhark_keyboard(&mut app);
    app.add_systems(OnEnter(GameState::Ready), spawn_typed_rune_display);
    app.add_systems(OnEnter(GameState::Ready), futhark::spawn_futhark_keyboard);
    app.add_systems(
        Update,
        (
            futhark::toggle_futhark_keyboard_legend_mode,
            futhark::sync_futhark_keyboard_labels,
            futhark::emit_typed_futhark_input_from_keyboard,
            futhark::emit_typed_futhark_input_from_keyboard_clicks,
            update_typed_rune,
        )
            .chain()
            .run_if(in_state(GameState::Ready)),
    );
    acceptance::initialize_app(
        &mut app,
        TEST_ID.into(),
        "Displays only the rune that matches the most recently typed character",
    );
    app.run();
}

fn spawn_typed_rune_display(mut commands: Commands, game_assets: Res<GameAssets>) {
    commands.spawn((
        Sprite {
            image: game_assets.futhark.clone(),
            texture_atlas: Some(TextureAtlas {
                layout: game_assets.futhark_layout.clone(),
                index: 0,
            }),
            ..default()
        },
        Transform::from_xyz(0.0, 120.0, 0.0),
        Visibility::Hidden,
        TypedRuneDisplay,
    ));
}

fn update_typed_rune(
    mut typed_rune_input: MessageReader<futhark::TypedFutharkInput>,
    mut display: Query<(&mut Sprite, &mut Visibility), With<TypedRuneDisplay>>,
) {
    let Some(last_typed) = futhark::last_typed_futhark_character(&mut typed_rune_input) else {
        return;
    };

    let Ok((mut sprite, mut visibility)) = display.single_mut() else {
        return;
    };

    if let Some(index) = futhark::letter_to_index(last_typed) {
        if let Some(texture_atlas) = &mut sprite.texture_atlas {
            texture_atlas.index = index;
            *visibility = Visibility::Visible;
        }
    } else {
        *visibility = Visibility::Hidden;
    }
}
