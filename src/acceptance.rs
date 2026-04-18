use bevy::prelude::*;

#[derive(Resource)]
pub struct AcceptanceTest {
    pub test_id: u8,
    pub show_grid: bool,
}

impl From<u8> for AcceptanceTest {
    fn from(test_id: u8) -> Self {
        Self {
            test_id,
            show_grid: false,
        }
    }
}

impl AcceptanceTest {
    pub fn with_grid(mut self) -> Self {
        self.show_grid = true;
        self
    }
}

/// Configures an app for user acceptance testing.
///
/// Sets the window title to include the test description and adds keybindings:
/// - F1: exit with status code 0 (pass)
/// - F2: exit with the `test_id` as the status code (fail)
pub fn initialize_app(app: &mut App, test: AcceptanceTest, description: &str) {
    assert!(test.test_id != 0, "test_id must be nonzero");

    let test_id = test.test_id;
    app.insert_resource(test);

    // Update the window title to show the test purpose.
    let title = format!("UAT #{test_id}: {description}");
    app.add_systems(Startup, move |mut windows: Query<&mut Window>| {
        for mut window in &mut windows {
            window.title = title.clone();
        }
    });

    app.add_systems(Update, handle_acceptance_keys);
    app.add_systems(Update, draw_grid);
}

fn handle_acceptance_keys(input: Res<ButtonInput<KeyCode>>, test: Res<AcceptanceTest>) {
    if input.just_pressed(KeyCode::F1) {
        std::process::exit(0);
    }
    if input.just_pressed(KeyCode::F2) {
        std::process::exit(test.test_id as i32);
    }
}

fn draw_grid(mut gizmos: Gizmos, test: Res<AcceptanceTest>) {
    if !test.show_grid {
        return;
    }
    let half_extent = 800.0_f32;
    let step = 50.0_f32;
    let color = Color::srgba(1.0, 1.0, 1.0, 0.25);
    let mut x = -half_extent;
    while x <= half_extent {
        gizmos.line_2d(Vec2::new(x, -half_extent), Vec2::new(x, half_extent), color);
        x += step;
    }
    let mut y = -half_extent;
    while y <= half_extent {
        gizmos.line_2d(Vec2::new(-half_extent, y), Vec2::new(half_extent, y), color);
        y += step;
    }
}
