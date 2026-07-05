use std::time::Duration;

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Frame, Layout, Margin, RichText, Sense, Stroke,
};
use localporter_core::ProcessSummary;

#[derive(Default)]
pub struct PortRow;

impl PortRow {
    const PORT_COLUMN_WIDTH: f32 = 120.0;
    const PORT_LABEL_WIDTH: f32 = 88.0;
    const ROW_MIN_HEIGHT: f32 = 50.0;
    const PORT_LABEL_FONT_SIZE: f32 = 16.0;
    const PORT_LABEL_HEIGHT: f32 = 50.0;
    const PORT_CENTER_SPACING: f32 = 12.0;
    const RIGHT_ICON_WIDTH: f32 = 20.0;

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        process: &ProcessSummary,
        uptime_offset: Duration,
    ) {
        Frame::group(ui.style())
            .fill(Color32::from_rgb(251, 251, 251))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_premultiplied(0, 0, 0, 13),
            ))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.set_min_height(Self::ROW_MIN_HEIGHT);

                ui.horizontal(|ui| {
                    ui.set_min_height(Self::ROW_MIN_HEIGHT);

                    ui.allocate_ui_with_layout(
                        egui::vec2(Self::PORT_COLUMN_WIDTH, Self::ROW_MIN_HEIGHT),
                        Layout::centered_and_justified(egui::Direction::TopDown),
                        |ui| self.port_column(ui, process),
                    );

                    ui.add_space(Self::PORT_CENTER_SPACING);

                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            (ui.available_width() - Self::RIGHT_ICON_WIDTH).max(0.0),
                            Self::ROW_MIN_HEIGHT,
                        ),
                        Layout::top_down(Align::Min),
                        |ui| self.center_column(ui, process, uptime_offset),
                    );

                    ui.allocate_ui_with_layout(
                        egui::vec2(Self::RIGHT_ICON_WIDTH, Self::ROW_MIN_HEIGHT),
                        Layout::right_to_left(Align::Center),
                        |ui| {
                            ui.label(
                                RichText::new(">")
                                    .size(18.0)
                                    .strong()
                                    .color(Color32::from_gray(150)),
                            );
                        },
                    );
                });
            });
    }

    fn port_column(&self, ui: &mut egui::Ui, process: &ProcessSummary) {
        let color = match process.primary_port().map(|port| port.protocol) {
            Some(localporter_core::PortProtocol::Tcp) => Color32::from_rgb(120, 170, 255),
            Some(localporter_core::PortProtocol::Udp) => Color32::from_rgb(104, 200, 156),
            None => Color32::from_rgb(120, 170, 255),
        };

        self.port_label(ui, process.primary_port_text(), color, true);
    }

    fn center_column(&self, ui: &mut egui::Ui, process: &ProcessSummary, uptime_offset: Duration) {
        ui.label(
            RichText::new(process.name_or_unknown())
                .size(18.0)
                .strong()
                .color(Color32::from_rgb(32, 37, 43)),
        );
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            self.meta_text(
                ui,
                format!("Launcher {}", Self::value_or_unknown(&process.launcher)),
            );
            self.meta_text(
                ui,
                format!(
                    "Uptime {}",
                    Self::format_uptime(process.uptime.saturating_add(uptime_offset))
                ),
            );
            self.meta_text(ui, format!("CPU {:.1}%", process.cpu_percent));
            self.meta_text(
                ui,
                format!("Memory {}", Self::format_memory(process.memory_usage)),
            );
        });
    }

    fn meta_text(&self, ui: &mut egui::Ui, text: String) {
        ui.label(
            RichText::new(text)
                .size(13.0)
                .color(Color32::from_rgb(112, 118, 126)),
        );
    }

    fn port_label(&self, ui: &mut egui::Ui, text: String, color: Color32, _strong: bool) {
        let font_id = FontId::monospace(Self::PORT_LABEL_FONT_SIZE);
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(Self::PORT_LABEL_WIDTH, Self::PORT_LABEL_HEIGHT),
            Sense::hover(),
        );

        ui.painter()
            .text(rect.center(), Align2::CENTER_CENTER, text, font_id, color);
    }

    fn value_or_unknown(value: &str) -> &str {
        if value.trim().is_empty() {
            "Unknown"
        } else {
            value
        }
    }

    fn format_uptime(uptime: Duration) -> String {
        let total_secs = uptime.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else {
            format!("{seconds}s")
        }
    }

    fn format_memory(memory_bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        let bytes = memory_bytes as f64;

        if bytes >= GB {
            format!("{:.1} GB", bytes / GB)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes / MB)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes / KB)
        } else {
            format!("{memory_bytes} B")
        }
    }
}
