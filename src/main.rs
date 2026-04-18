use bevy::prelude::*;

#[cfg(test)]
use bevy::time::TimeUpdateStrategy;
#[cfg(test)]
use std::time::Duration;

#[cfg(test)]
mod snapshot;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    app.run();
}

fn configure_app(app: &mut App) {
    let _ = app
        // Set the clear color to blue
        .insert_resource(ClearColor(Color::linear_rgb(0.0, 0.0, 1.0)))
        // Spawn a red square in the center of the screen
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Sprite {
                color: Color::linear_rgb(1.0, 0.0, 0.0),
                custom_size: Some(Vec2::new(100.0, 100.0)),
                ..default()
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        configure_app(&mut app);

        // Use a fixed timestep so elapsed time progression is deterministic.
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )));

        app
    }

    #[test]
    fn time_advances_between_frames() {
        let mut app = create_test_app();

        app.update();
        let elapsed_after_first_update = app.world().resource::<Time>().elapsed();
        app.update();
        let elapsed_after_second_update = app.world().resource::<Time>().elapsed();
        assert!(elapsed_after_second_update > elapsed_after_first_update);
    }

    /// Screenshot test for the default (empty) app window.
    ///
    /// Run with `--features update` to capture or refresh the baseline:
    ///   cargo test snapshot_default_window --features update
    ///
    /// Run without the flag to verify against the saved baseline:
    ///   cargo test snapshot_default_window
    #[test]
    fn snapshot_default_window() {
        use bevy::winit::WinitPlugin;
        use std::path::PathBuf;

        let mut app = App::new();
        app.add_plugins(DefaultPlugins.build().disable::<WinitPlugin>());
        configure_app(&mut app);

        let last_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("snapshots")
            .join("default_window_last.png");
        let _ = std::fs::remove_file(&last_path);

        app.add_systems(Update, |mut commands: Commands, mut done: Local<bool>| {
            if !*done {
                *done = true;
                snapshot::take(&mut commands, "default_window");
            }
        });

        app.finish();
        app.cleanup();

        for _ in 0..40 {
            app.update();
            if last_path.exists() {
                break;
            }
        }

        assert!(
            last_path.exists(),
            "Expected snapshot output at {:?}",
            last_path
        );
    }
}
