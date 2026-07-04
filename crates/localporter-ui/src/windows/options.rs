use eframe::{
    NativeOptions,
    egui::{Vec2, ViewportBuilder},
};

use crate::windows::constants::{
    APP_NAME, DEFAULT_HEIGHT, DEFAULT_WIDTH, MIN_HEIGHT, MIN_WIDTH, WINDOWS_DECORATED,
    WINDOWS_RESIZEABLE,
};

pub fn standalone_native_options() -> NativeOptions {
    NativeOptions {
        viewport: ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_inner_size(Vec2::new(DEFAULT_WIDTH, DEFAULT_HEIGHT))
            .with_min_inner_size(Vec2::new(MIN_WIDTH, MIN_HEIGHT))
            .with_resizable(WINDOWS_RESIZEABLE)
            .with_decorations(WINDOWS_DECORATED),
        ..Default::default()
    }
}
