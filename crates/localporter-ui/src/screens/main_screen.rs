use crate::{components::PortRow, state::AppState};

#[derive(Default)]
pub struct MainScreen {
    port_row: PortRow,
}

impl MainScreen {
    pub fn ui(&mut self, ui: &mut eframe::egui::Ui, state: &AppState) {
        let Some(snapshot) = &state.snapshot else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("Waiting for first snapshot...");
            });
            return;
        };
        let uptime_offset = state.elapsed_since_collection();

        eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            for item in &snapshot.items {
                self.port_row.ui(ui, item, uptime_offset);
            }
        });
    }
}
