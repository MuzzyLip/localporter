use eframe::egui;

use crate::{screens::MainScreen, state::AppState};

#[derive(Default)]
pub struct StandaloneApp {
    state: AppState,
    main_screen: MainScreen,
}

impl eframe::App for StandaloneApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            self.main_screen.ui(ui, &mut self.state);
        });
    }
}
