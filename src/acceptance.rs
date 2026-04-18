use bevy::prelude::*;

#[derive(Resource)]
struct AcceptanceTest {
    test_id: u8,
}

/// Configures an app for user acceptance testing.
///
/// Sets the window title to include the test description and adds keybindings:
/// - F1: exit with status code 0 (pass)
/// - F2: exit with the `test_id` as the status code (fail)
pub fn initialize_app(app: &mut App, test_id: u8, description: &str) {
    assert!(test_id != 0, "test_id must be nonzero");

    app.insert_resource(AcceptanceTest { test_id });

    // Update the window title to show the test purpose.
    let title = format!("UAT #{test_id}: {description}");
    app.add_systems(Startup, move |mut windows: Query<&mut Window>| {
        for mut window in &mut windows {
            window.title = title.clone();
        }
    });

    app.add_systems(Update, handle_acceptance_keys);
}

fn handle_acceptance_keys(input: Res<ButtonInput<KeyCode>>, test: Res<AcceptanceTest>) {
    if input.just_pressed(KeyCode::F1) {
        std::process::exit(0);
    }
    if input.just_pressed(KeyCode::F2) {
        std::process::exit(test.test_id as i32);
    }
}
