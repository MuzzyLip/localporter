use crate::state::AppState;

#[derive(Default)]
pub struct MainScreen;

impl MainScreen {
    pub fn ui(&mut self, ui: &mut eframe::egui::Ui, state: &mut AppState) {
        eframe::egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Hello LocalPorter");
                ui.label(&state.status_text)
            })
        });
    }
}
