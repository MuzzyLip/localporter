use std::time::Duration;

use eframe::egui;

use crate::{screens::MainScreen, state::AppState};

pub struct StandaloneApp {
    state: AppState,
    main_screen: MainScreen,
}

impl StandaloneApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        cc.egui_ctx.style_mut_of(egui::Theme::Light, |style| {
            style.visuals = egui::Visuals::light();
            style.visuals.panel_fill = egui::Color32::from_rgb(251, 251, 251);
        });

        Self {
            state: AppState::new(cc.egui_ctx.clone()),
            main_screen: MainScreen::default(),
        }
    }
}

impl eframe::App for StandaloneApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.state.drain_updates();
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        self.main_screen.ui(ui, &self.state);
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(251, 251, 251).to_normalized_gamma_f32()
    }
}
