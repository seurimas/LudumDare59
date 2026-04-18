use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;

#[cfg(test)]
mod snapshot;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    configure_app(&mut app);
    app.run();
}

fn configure_app(app: &mut App) {
    let _ = app;
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

        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        visible: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(WinitPlugin {
                    run_on_any_thread: true,
                }),
        );
        configure_app(&mut app);

        app.add_systems(Update, |mut commands: Commands, mut done: Local<bool>| {
            if !*done {
                *done = true;
                snapshot::take(&mut commands, "default_window");
            }
        });

        app.finish();
        app.cleanup();

        for _ in 0..10 {
            app.update();
        }
    }
}
