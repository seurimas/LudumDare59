use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    prelude::{Camera2d, Commands, Image, On},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
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

/// Spawns an off-screen camera, captures one frame, and attaches the snapshot observer.
///
/// When the `update` feature is set, saves the captured image as the new baseline.
/// Otherwise, compares against the saved baseline and panics if they differ or none exists.
/// The captured image is always written to `tests/snapshots/<name>_last.png`.
pub fn take(commands: &mut Commands, name: &'static str) {
    commands.queue(move |world: &mut bevy::prelude::World| {
        let mut images = world.resource_mut::<bevy::prelude::Assets<Image>>();
        let size = Extent3d {
            width: 320,
            height: 180,
            depth_or_array_layers: 1,
        };

        let mut render_target = Image::new_fill(
            size,
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        render_target.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
        let handle = images.add(render_target);
        drop(images);

        world.spawn((Camera2d, RenderTarget::Image(handle.clone().into())));

        world
            .spawn(Screenshot::image(handle))
            .observe(snapshot_observer(name));
    });
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
