use eframe::egui;

use crate::{screens::MainScreen, state::AppState};

pub struct StandaloneApp {
    state: AppState,
    main_screen: MainScreen,
}

impl StandaloneApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            state: AppState::new(cc.egui_ctx.clone()),
            main_screen: MainScreen,
        }
    }
}

impl eframe::App for StandaloneApp {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.state.drain_updates();
        self.main_screen.ui(ui, &self.state);
    }
}
