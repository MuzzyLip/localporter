use std::time::Duration;

use eframe::egui;

use crate::{
    components::{TitleBar, ToastOverlay},
    screens::MainScreen,
    state::AppState,
    windows::constants::WINDOW_CORNER_RADIUS,
};

pub struct StandaloneApp {
    state: AppState,
    title_bar: TitleBar,
    main_screen: MainScreen,
    toast_overlay: ToastOverlay,
}

const WINDOW_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(251, 251, 251);

impl StandaloneApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);
        cc.egui_ctx.style_mut_of(egui::Theme::Light, |style| {
            style.visuals = egui::Visuals::light();
            style.visuals.panel_fill = egui::Color32::TRANSPARENT;
        });

        Self {
            state: AppState::new(cc.egui_ctx.clone()),
            title_bar: TitleBar,
            main_screen: MainScreen::default(),
            toast_overlay: ToastOverlay,
        }
    }
}

impl eframe::App for StandaloneApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.state.drain_updates();
        let toasts = self.state.toast_views();
        ui.ctx().request_repaint_after(Duration::from_secs(1));
        let maximized = ui
            .ctx()
            .input(|input| input.viewport().maximized.unwrap_or(false));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::ZERO))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                let panel_rect = ui.max_rect().shrink(0.5);
                ui.painter().rect_filled(
                    panel_rect,
                    full_corner_radius(maximized),
                    WINDOW_BACKGROUND,
                );

                let mut show_all_enabled = self.state.show_all_enabled;
                if self.title_bar.show(ui, &mut show_all_enabled) {
                    self.state.set_show_all_enabled(show_all_enabled);
                }

                let content_rect = ui.available_rect_before_wrap();

                ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                    ui.set_min_size(content_rect.size());
                    self.main_screen.ui(ui, &mut self.state);
                });

                ui.painter().rect_stroke(
                    panel_rect,
                    full_corner_radius(maximized),
                    egui::Stroke::new(1.0, window_border()),
                    egui::StrokeKind::Middle,
                );
            });

        self.toast_overlay.show(ui.ctx(), &toasts);
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

fn full_corner_radius(maximized: bool) -> egui::CornerRadius {
    if maximized {
        egui::CornerRadius::ZERO
    } else {
        egui::CornerRadius::same(WINDOW_CORNER_RADIUS)
    }
}

fn window_border() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 13)
}
