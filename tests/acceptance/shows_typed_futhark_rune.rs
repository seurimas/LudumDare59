use LudumDare59::{GameAssets, GameState, acceptance, configure_app, configure_loading, futhark};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

const TEST_ID: u8 = 4;

#[derive(Component)]
struct TypedRuneDisplay;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    configure_loading(&mut app);
    app.add_systems(OnEnter(GameState::Ready), spawn_typed_rune_display);
    app.add_systems(Update, update_typed_rune.run_if(in_state(GameState::Ready)));
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
        Visibility::Hidden,
        TypedRuneDisplay,
    ));
}

fn update_typed_rune(
    mut keyboard_input: MessageReader<KeyboardInput>,
    mut display: Query<(&mut Sprite, &mut Visibility), With<TypedRuneDisplay>>,
) {
    let Some(last_typed) = last_typed_character(&mut keyboard_input) else {
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

fn last_typed_character(keyboard_input: &mut MessageReader<KeyboardInput>) -> Option<char> {
    let mut typed = None;

    for event in keyboard_input.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        let Some(text) = &event.text else {
            continue;
        };

        for c in text.chars() {
            typed = Some(c);
        }
    }

    typed
}
