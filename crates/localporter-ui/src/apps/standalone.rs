use std::time::Duration;

use eframe::egui;

use crate::{
    components::{BottomBar, ConfirmDialog, FilterBar, SettingsModal, TitleBar, ToastOverlay},
    screens::{MainScreen, MainScreenAction},
    state::{AppSettings, AppState},
    windows::constants::WINDOW_CORNER_RADIUS,
};

pub struct StandaloneApp {
    state: AppState,
    title_bar: TitleBar,
    filter_bar: FilterBar,
    main_screen: MainScreen,
    bottom_bar: BottomBar,
    settings_modal: SettingsModal,
    confirm_dialog: ConfirmDialog,
    toast_overlay: ToastOverlay,
    settings_open: bool,
    search_query: String,
    filtered_killable_pids: Vec<u32>,
    pending_settings: Option<AppSettings>,
    pending_kill_action: Option<PendingKillAction>,
}

const WINDOW_BACKGROUND: egui::Color32 = egui::Color32::from_rgb(251, 251, 251);
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
            filter_bar: FilterBar,
            main_screen: MainScreen::default(),
            bottom_bar: BottomBar,
            settings_modal: SettingsModal,
            confirm_dialog: ConfirmDialog,
            toast_overlay: ToastOverlay,
            settings_open: false,
            search_query: String::new(),
            filtered_killable_pids: Vec::new(),
            pending_settings: None,
            pending_kill_action: None,
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
                self.filter_bar.show(ui, &mut self.search_query);

                let remaining_rect = ui.available_rect_before_wrap();
                let bottom_bar_rect = egui::Rect::from_min_max(
                    egui::pos2(
                        remaining_rect.left(),
                        (remaining_rect.bottom() - BottomBar::HEIGHT).max(remaining_rect.top()),
                    ),
                    remaining_rect.right_bottom(),
                );
                let content_rect = egui::Rect::from_min_max(
                    remaining_rect.min,
                    egui::pos2(remaining_rect.max.x, bottom_bar_rect.min.y),
                );

                ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                    ui.set_min_size(content_rect.size());
                    let output = self.main_screen.ui(ui, &mut self.state, &self.search_query);
                    self.filtered_killable_pids = output.killable_pids;
                    if let Some(action) = output.action {
                        self.handle_main_screen_action(action);
                    }
                });

                ui.scope_builder(egui::UiBuilder::new().max_rect(bottom_bar_rect), |ui| {
                    ui.set_min_size(bottom_bar_rect.size());
                    let bottom_bar_response =
                        self.bottom_bar.show(ui, self.filtered_killable_pids.len());
                    if bottom_bar_response.kill_all_clicked {
                        self.request_kill_action(PendingKillAction::KillAllKillable(
                            self.filtered_killable_pids.clone(),
                        ));
                    }
                    if bottom_bar_response.settings_clicked {
                        if !self.settings_open {
                            self.pending_settings = Some(self.state.settings().clone());
                        }
                        self.settings_open = true;
                    }
                });

                ui.painter().rect_stroke(
                    panel_rect,
                    full_corner_radius(maximized),
                    egui::Stroke::new(1.0, window_border()),
                    egui::StrokeKind::Middle,
                );
            });

        self.show_settings_modal(ui.ctx());
        self.show_confirm_dialog(ui.ctx());
        self.toast_overlay.show(ui.ctx(), &toasts);
    }

    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        egui::Color32::TRANSPARENT.to_normalized_gamma_f32()
    }
}

impl StandaloneApp {
    fn handle_main_screen_action(&mut self, action: MainScreenAction) {
        match action {
            MainScreenAction::KillProcess(pid) => {
                self.request_kill_action(PendingKillAction::Single(pid));
            }
        }
    }

    fn request_kill_action(&mut self, action: PendingKillAction) {
        match action {
            PendingKillAction::Single(pid) if self.state.is_kill_pending(pid) => {}
            PendingKillAction::KillAllKillable(ref pids) if pids.is_empty() => {}
            _ if self.state.kill_requires_confirmation() => {
                self.pending_kill_action = Some(action);
            }
            PendingKillAction::Single(pid) => self.state.kill_process(pid),
            PendingKillAction::KillAllKillable(pids) => self.state.kill_processes(pids),
        }
    }

    fn show_settings_modal(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }

        if self.pending_settings.is_none() {
            self.pending_settings = Some(self.state.settings().clone());
        }

        let response = self.settings_modal.show(
            ctx,
            self.pending_settings
                .as_mut()
                .expect("pending settings should exist when modal is open"),
            self.state.launch_at_startup_supported(),
            APP_VERSION,
        );

        if response.save_requested {
            if let Some(settings) = self.pending_settings.take() {
                self.state.apply_settings(settings);
            }
            self.settings_open = false;
        } else if response.close_requested {
            self.pending_settings = None;
            self.settings_open = false;
        }
    }

    fn show_confirm_dialog(&mut self, ctx: &egui::Context) {
        let Some(action) = self.pending_kill_action.clone() else {
            return;
        };

        let (title, message, confirm_label) = match action {
            PendingKillAction::Single(pid) => (
                "Confirm Kill",
                format!("Kill PID {pid} now?"),
                "Kill process",
            ),
            PendingKillAction::KillAllKillable(ref pids) => (
                "Confirm Kill killable",
                format!("Kill {} killable process(es) now?", pids.len()),
                "Kill killable",
            ),
        };

        let response = self
            .confirm_dialog
            .show(ctx, title, &message, confirm_label);

        if response.confirmed {
            self.pending_kill_action = None;
            match action {
                PendingKillAction::Single(pid) => self.state.kill_process(pid),
                PendingKillAction::KillAllKillable(pids) => self.state.kill_processes(pids),
            }
        } else if response.canceled {
            self.pending_kill_action = None;
        }
    }
}

#[derive(Clone)]
enum PendingKillAction {
    Single(u32),
    KillAllKillable(Vec<u32>),
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
