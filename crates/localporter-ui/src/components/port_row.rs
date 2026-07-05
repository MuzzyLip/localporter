use std::time::Duration;

use eframe::egui::{
    self, Align, Color32, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke, UiBuilder,
};
use localporter_core::ProcessSummary;

#[derive(Default)]
pub struct PortRow;

impl PortRow {
    const PORT_COLUMN_WIDTH: f32 = 80.0;
    const PORT_LABEL_WIDTH: f32 = 32.0;
    const ROW_MIN_HEIGHT: f32 = 50.0;
    const PORT_LABEL_FONT_SIZE: f32 = 16.0;
    const PORT_SUFFIX_FONT_SIZE: f32 = 12.0;
    const PORT_LABEL_HEIGHT: f32 = 50.0;
    const PORT_CENTER_SPACING: f32 = 12.0;
    const CENTER_RIGHT_SPACING: f32 = 12.0;
    const RIGHT_ICON_WIDTH: f32 = 20.0;
    const TITLE_HEIGHT: f32 = 22.0;
    const META_HEIGHT: f32 = 18.0;

    pub fn ui(&mut self, ui: &mut egui::Ui, process: &ProcessSummary, uptime_offset: Duration) {
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

                let (row_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), Self::ROW_MIN_HEIGHT),
                    Sense::hover(),
                );

                let port_rect = Rect::from_min_size(
                    row_rect.min,
                    egui::vec2(Self::PORT_COLUMN_WIDTH, Self::ROW_MIN_HEIGHT),
                );
                let icon_rect = Rect::from_min_size(
                    egui::pos2(row_rect.right() - Self::RIGHT_ICON_WIDTH, row_rect.top()),
                    egui::vec2(Self::RIGHT_ICON_WIDTH, Self::ROW_MIN_HEIGHT),
                );
                let center_left = port_rect.right() + Self::PORT_CENTER_SPACING;
                let center_right = (icon_rect.left() - Self::CENTER_RIGHT_SPACING).max(center_left);
                let center_rect = Rect::from_min_max(
                    egui::pos2(center_left, row_rect.top()),
                    egui::pos2(center_right, row_rect.bottom()),
                );

                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(port_rect)
                        .layout(Layout::centered_and_justified(egui::Direction::TopDown)),
                    |ui| self.port_column(ui, process),
                );

                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(center_rect)
                        .layout(Layout::top_down(Align::Min)),
                    |ui| self.center_column(ui, process, uptime_offset),
                );

                ui.scope_builder(
                    UiBuilder::new()
                        .max_rect(icon_rect)
                        .layout(Layout::right_to_left(Align::Center)),
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
    }

    fn port_column(&self, ui: &mut egui::Ui, process: &ProcessSummary) {
        let color = match process.primary_port().map(|port| port.protocol) {
            Some(localporter_core::PortProtocol::Tcp) => Color32::from_rgb(120, 170, 255),
            Some(localporter_core::PortProtocol::Udp) => Color32::from_rgb(104, 200, 156),
            None => Color32::from_rgb(120, 170, 255),
        };

        self.port_label(ui, process, color);
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

    fn port_label(&self, ui: &mut egui::Ui, process: &ProcessSummary, color: Color32) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(Self::PORT_LABEL_WIDTH, Self::PORT_LABEL_HEIGHT),
            Sense::hover(),
        );

        let primary_text = process
            .primary_port()
            .map(|port| format!(":{}", port.port))
            .unwrap_or_else(|| "Unknown".to_owned());
        let remaining_count = process.remaining_port_count();

        let primary_font = FontId::monospace(Self::PORT_LABEL_FONT_SIZE);
        let suffix_font = FontId::monospace(Self::PORT_SUFFIX_FONT_SIZE);
        let suffix_text = if remaining_count > 0 {
            format!(" +{remaining_count}")
        } else {
            String::new()
        };

        let primary_galley = ui
            .painter()
            .layout_no_wrap(primary_text, primary_font.clone(), color);
        let suffix_galley = if suffix_text.is_empty() {
            None
        } else {
            Some(ui.painter().layout_no_wrap(
                suffix_text,
                suffix_font.clone(),
                Color32::from_rgb(148, 154, 163),
            ))
        };

        let total_width = primary_galley.size().x
            + suffix_galley
                .as_ref()
                .map(|galley| galley.size().x)
                .unwrap_or_default();
        let start_x = rect.center().x - total_width * 0.5;
        let primary_pos = egui::pos2(start_x, rect.center().y - primary_galley.size().y * 0.5);

        ui.painter().galley(primary_pos, primary_galley, color);

        if let Some(suffix_galley) = suffix_galley {
            let suffix_pos = egui::pos2(
                start_x + total_width - suffix_galley.size().x,
                rect.center().y - suffix_galley.size().y * 0.5,
            );
            ui.painter()
                .galley(suffix_pos, suffix_galley, Color32::from_rgb(148, 154, 163));
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
