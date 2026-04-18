use bevy::{
    prelude::{Commands, On},
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
};
use std::path::PathBuf;

fn snapshot_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}.png"))
}

fn snapshot_last_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{name}_last.png"))
}

/// Spawns a screenshot of the primary window and attaches the snapshot observer.
///
/// When the `update` feature is set, saves the captured image as the new baseline.
/// Otherwise, compares against the saved baseline and panics if they differ or none exists.
/// The captured image is always written to `tests/snapshots/<name>_last.png`.
pub fn take(commands: &mut Commands, name: &'static str) {
    commands
        .spawn(Screenshot::primary_window())
        .observe(snapshot_observer(name));
}

fn snapshot_observer(
    name: &'static str,
) -> impl FnMut(On<ScreenshotCaptured>) + Send + Sync + 'static {
    move |trigger: On<ScreenshotCaptured>| {
        let img = trigger
            .image
            .clone()
            .try_into_dynamic()
            .unwrap_or_else(|_| panic!("Failed to convert screenshot '{name}' to image"));

        let last_path = snapshot_last_path(name);
        std::fs::create_dir_all(last_path.parent().unwrap())
            .expect("Failed to create snapshots directory");
        img.save(&last_path)
            .unwrap_or_else(|e| panic!("Failed to save _last snapshot '{name}': {e}"));

        let path = snapshot_path(name);

        #[cfg(feature = "update")]
        {
            img.save(&path)
                .unwrap_or_else(|e| panic!("Failed to save snapshot '{name}': {e}"));
            println!("Snapshot updated: {path:?}");
        }

        #[cfg(not(feature = "update"))]
        {
            assert!(
                path.exists(),
                "No snapshot found at {path:?}. Run with `--features update` to create one."
            );
            let expected = image::open(&path)
                .unwrap_or_else(|e| panic!("Failed to load snapshot '{name}': {e}"));
            assert_eq!(
                img.to_rgba8().into_raw(),
                expected.to_rgba8().into_raw(),
                "Screenshot does not match saved snapshot '{name}'"
            );
        }
    }
}
