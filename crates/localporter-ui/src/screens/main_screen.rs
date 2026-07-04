use crate::state::AppState;

#[derive(Default)]
pub struct MainScreen;

impl MainScreen {
    pub fn ui(&mut self, ui: &mut eframe::egui::Ui, state: &AppState) {
        ui.heading("LocalPorter");
        ui.label(&state.status_text);
        ui.separator();

        let Some(snapshot) = &state.snapshot else {
            ui.label("Waiting for first snapshot...");
            return;
        };

        ui.label(format!("Processes: {}", snapshot.items.len()));
        ui.label(format!("Warnings: {}", snapshot.warnings.len()));
        ui.separator();

        eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &snapshot.items {
                let tcp_ports = item.tcp_ports();
                let udp_ports = item.udp_ports();

                ui.group(|ui| {
                    ui.label(format!(
                        "{}  pid-group launcher={}",
                        item.name_or_unknown(),
                        item.launcher
                    ));

                    if tcp_ports.is_empty() {
                        ui.label("TCP LISTEN: -");
                    } else {
                        ui.label(format!("TCP LISTEN: {:?}", tcp_ports));
                    }

                    if udp_ports.is_empty() {
                        ui.label("UDP Bound: -");
                    } else {
                        ui.label(format!("UDP Bound: {:?}", udp_ports));
                    }

                    ui.label(format!("Uptime: {}s", item.uptime.as_secs()));
                    ui.label(format!("CPU: {:.2}%", item.cpu_percent));
                    ui.label(format!("Memory: {} bytes", item.memory_usage));
                });

                ui.add_space(8.0);
            }
        });
    }
}
