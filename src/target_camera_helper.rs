use bevy::{
    asset::Assets,
    ecs::system::SystemParam,
    math::UVec2,
    prelude::{Camera, Entity, Image, Query, Res, With},
    render::camera::RenderTarget,
    ui::TargetCamera,
    window::{PrimaryWindow, Window, WindowRef},
};

/// helper to get window/texture info for a ui element based on [`TargetCamera`]
#[derive(SystemParam)]
pub struct TargetCameraHelper<'w, 's> {
    target_camera: Query<'w, 's, &'static TargetCamera>,
    cameras: Query<'w, 's, &'static Camera>,
    all_windows: Query<'w, 's, &'static Window>,
    primary_window: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    images: Res<'w, Assets<Image>>,
}

pub struct TargetCameraProps {
    #[allow(dead_code)]
    pub target_camera: Option<TargetCamera>,
    #[allow(dead_code)]
    pub size: UVec2,
    pub scale_factor: f32,
}

impl<'w, 's> TargetCameraHelper<'w, 's> {
    /// get info for entity with an optional [`TargetCamera`]
    pub fn get_props(&self, e: Entity) -> Option<TargetCameraProps> {
        let target_camera = self.target_camera.get(e).ok().cloned();
        let (window_ref, texture_ref) = match &target_camera {
            Some(target) => {
                let camera = self.cameras.get(target.0).ok()?;

                match &camera.target {
                    RenderTarget::Window(window_ref) => (Some(*window_ref), None),
                    RenderTarget::Image(h_image) => (None, Some(h_image)),
                    _ => return None,
                }
            }
            None => (Some(WindowRef::Primary), None),
        };

        let window = window_ref.and_then(|window_ref| match window_ref {
            WindowRef::Entity(w) => self.all_windows.get(w).ok(),
            WindowRef::Primary => self.primary_window.get_single().ok(),
        });

        let scale_factor = window.map(Window::scale_factor).unwrap_or(1.0);
        let size = if let Some(h_image) = texture_ref {
            self.images.get(h_image)?.size()
        } else {
            window?.size().as_uvec2()
        };

        Some(TargetCameraProps {
            target_camera,
            size,
            scale_factor,
        })
    }
}
