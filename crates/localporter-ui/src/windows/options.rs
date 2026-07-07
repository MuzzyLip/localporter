use eframe::{
    NativeOptions,
    icon_data::from_png_bytes,
    egui::{Vec2, ViewportBuilder},
};

#[cfg(target_os = "macos")]
use eframe::egui::IconData;

#[cfg(not(target_os = "macos"))]
use eframe::egui::IconData;
use crate::windows::constants::{
    APP_NAME, DEFAULT_HEIGHT, DEFAULT_WIDTH, MIN_HEIGHT, MIN_WIDTH, WINDOWS_DECORATED,
    WINDOWS_RESIZEABLE,
};

pub fn standalone_native_options() -> NativeOptions {
    let viewport = ViewportBuilder::default()
        .with_app_id(APP_NAME)
        .with_title(APP_NAME)
        .with_icon(app_icon())
        .with_inner_size(Vec2::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        .with_min_inner_size(Vec2::new(MIN_WIDTH, MIN_HEIGHT))
        .with_resizable(WINDOWS_RESIZEABLE)
        .with_decorations(WINDOWS_DECORATED)
        .with_transparent(true);

    #[cfg(target_os = "macos")]
    let viewport = viewport
        .with_visible(false)
        .with_active(false)
        .with_title_shown(false)
        .with_titlebar_buttons_shown(false)
        .with_titlebar_shown(false)
        .with_fullsize_content_view(true);

    NativeOptions {
        viewport,
        centered: true,
        ..Default::default()
    }
}

fn app_icon() -> IconData {
    from_png_bytes(include_bytes!("../../assets/app-icon.png"))
        .expect("app icon png should be valid")
}
