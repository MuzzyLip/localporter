use std::time::Duration;

use eframe::egui::{self, Align, Color32, Layout, Rect, RichText, Sense, UiBuilder};
use localporter_core::{BoundPort, PortProtocol, ProcessSummary};

#[derive(Default)]
pub struct PortRow;

impl PortRow {
    const PORT_COLUMN_WIDTH: f32 = 80.0;
    const ROW_MIN_HEIGHT: f32 = 50.0;
    const PORT_LABEL_FONT_SIZE: f32 = 13.0;
    const PORT_CENTER_SPACING: f32 = 12.0;
    const TITLE_HEIGHT: f32 = 22.0;
    const META_HEIGHT: f32 = 18.0;

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        process: &ProcessSummary,
        port: Option<BoundPort>,
        uptime_offset: Duration,
    ) {
        ui.set_min_height(Self::ROW_MIN_HEIGHT);

        let row_rect = ui.max_rect();
        let (_, response) = ui.allocate_exact_size(
            egui::vec2(row_rect.width(), Self::ROW_MIN_HEIGHT),
            Sense::hover(),
        );
        let row_rect = response.rect;

        let port_rect = Rect::from_min_size(
            row_rect.min,
            egui::vec2(Self::PORT_COLUMN_WIDTH, Self::ROW_MIN_HEIGHT),
        );
        let center_rect = Rect::from_min_max(
            egui::pos2(
                port_rect.right() + Self::PORT_CENTER_SPACING,
                row_rect.top(),
            ),
            row_rect.right_bottom(),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(port_rect)
                .layout(Layout::centered_and_justified(egui::Direction::TopDown)),
            |ui| self.port_column(ui, port),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(center_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| self.center_column(ui, process, uptime_offset),
        );
    }

    fn port_column(&self, ui: &mut egui::Ui, port: Option<BoundPort>) {
        self.port_label(ui, port);
    }

    fn center_column(&self, ui: &mut egui::Ui, process: &ProcessSummary, uptime_offset: Duration) {
        let rect = ui.max_rect();
        let title_rect =
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), Self::TITLE_HEIGHT));
        let meta_top = (rect.bottom() - Self::META_HEIGHT).max(title_rect.bottom());
        let meta_rect = Rect::from_min_max(
            egui::pos2(rect.left(), meta_top),
            egui::pos2(rect.right(), rect.bottom()),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(title_rect)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.label(
                    RichText::new(process.name_or_unknown())
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(32, 37, 43)),
                );
            },
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(meta_rect)
                .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

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
            },
        );
    }

    fn meta_text(&self, ui: &mut egui::Ui, text: String) {
        ui.label(
            RichText::new(text)
                .size(13.0)
                .color(Color32::from_rgb(112, 118, 126)),
        );
    }

    fn port_label(&self, ui: &mut egui::Ui, port: Option<BoundPort>) {
        let text = match port {
            Some(port) => format!(":{}", port.port),
            None => "Unknown".to_owned(),
        };
        let color = self.port_color(port.map(|value| value.protocol));

        ui.label(
            RichText::new(text)
                .monospace()
                .size(Self::PORT_LABEL_FONT_SIZE)
                .color(color),
        );
    }

    fn port_color(&self, protocol: Option<PortProtocol>) -> Color32 {
        match protocol {
            Some(PortProtocol::Tcp) => Color32::from_rgb(120, 170, 255),
            Some(PortProtocol::Udp) => Color32::from_rgb(104, 200, 156),
            None => Color32::from_rgb(148, 154, 163),
        }
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
