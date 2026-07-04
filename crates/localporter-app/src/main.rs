use eframe::egui;

#[derive(Default)]
struct LocalPorterApp;

impl eframe::App for LocalPorterApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading("Hello, LocalPorter!");
                ui.label("egui hello-world is running.");
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions::default();

    eframe::run_native(
        "LocalPorter",
        options,
        Box::new(|_cc| Ok(Box::<LocalPorterApp>::default())),
    )
}
