use eframe::egui::{self, Align, Color32, CornerRadius, Frame, Layout, Margin, RichText};
use localporter_core::ProcessSummary;

pub enum PortRowDetailsAction {
    KillProcess(u32),
}

#[derive(Default)]
pub struct PortRowDetails;

impl PortRowDetails {
    const DETAIL_LABEL_WIDTH: f32 = 72.0;
    const DETAIL_ROW_HEIGHT: f32 = 20.0;
    const KILL_BUTTON_WIDTH: f32 = 60.0;
    const KILL_BUTTON_HEIGHT: f32 = 30.0;
    const COLUMN_SPACING: f32 = 16.0;
    const ROW_SPACING: f32 = 8.0;
    const COMMAND_BOX_HEIGHT: f32 = 42.0;
    const COMMAND_BOX_FILL: Color32 = Color32::from_rgb(247, 247, 247);

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        process: &ProcessSummary,
        kill_pending: bool,
    ) -> Option<PortRowDetailsAction> {
        let mut action = None;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = Self::COLUMN_SPACING;

            let details_width =
                (ui.available_width() - Self::KILL_BUTTON_WIDTH - Self::COLUMN_SPACING).max(0.0);

            let details = ui.allocate_ui_with_layout(
                egui::vec2(details_width, 0.0),
                Layout::top_down(Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = Self::ROW_SPACING;
                    self.detail_row(ui, "PID", process.pid.to_string());
                    self.command_row(ui, process);
                },
            );
            let details_height = details.response.rect.height();

            ui.allocate_ui_with_layout(
                egui::vec2(Self::KILL_BUTTON_WIDTH, details_height),
                Layout::top_down(Align::TOP),
                |ui| {
                    if self.kill_button(ui, kill_pending).clicked() {
                        action = Some(PortRowDetailsAction::KillProcess(process.pid));
                    }
                },
            );
        });

        action
    }

    fn detail_row(&self, ui: &mut egui::Ui, label: &str, value: String) {
        ui.horizontal(|ui| {
            self.detail_label(ui, label);
            ui.label(
                RichText::new(value)
                    .size(12.0)
                    .monospace()
                    .color(Color32::from_rgb(32, 37, 43)),
            );
        });
    }

    fn command_row(&self, ui: &mut egui::Ui, process: &ProcessSummary) {
        ui.vertical(|ui| {
            self.detail_label(ui, "Command");
            ui.add_space(4.0);

            Frame::new()
                .fill(Self::COMMAND_BOX_FILL)
                .stroke(Self::command_box_stroke())
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("command", process.pid))
                        .max_height(Self::COMMAND_BOX_HEIGHT)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.add(
                                egui::Label::new(
                                    RichText::new(Self::value_or_unknown(&process.command))
                                        .size(12.0)
                                        .monospace()
                                        .color(Color32::from_rgb(32, 37, 43)),
                                )
                                .wrap(),
                            );
                        });
                });
        });
    }

    fn detail_label(&self, ui: &mut egui::Ui, label: &str) {
        ui.allocate_ui_with_layout(
            egui::vec2(Self::DETAIL_LABEL_WIDTH, Self::DETAIL_ROW_HEIGHT),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(label)
                        .size(12.0)
                        .strong()
                        .color(Color32::from_rgb(112, 118, 126)),
                );
            },
        );
    }

    fn kill_button(&self, ui: &mut egui::Ui, kill_pending: bool) -> egui::Response {
        ui.scope(|ui| {
            ui.spacing_mut().button_padding = egui::vec2(0.0, 0.0);
            ui.add_sized(
                [Self::KILL_BUTTON_WIDTH, Self::KILL_BUTTON_HEIGHT],
                egui::Button::new(
                    RichText::new(if kill_pending { "Killing" } else { "Kill" })
                        .size(12.0)
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(if kill_pending {
                    Color32::from_rgb(181, 115, 115)
                } else {
                    Color32::from_rgb(217, 61, 61)
                })
                .corner_radius(CornerRadius::same(8))
                .sense(if kill_pending {
                    egui::Sense::hover()
                } else {
                    egui::Sense::click()
                }),
            )
        })
        .inner
    }

    fn value_or_unknown(value: &str) -> &str {
        if value.trim().is_empty() {
            "Unknown"
        } else {
            value
        }
    }

    fn command_box_stroke() -> egui::Stroke {
        egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 13))
    }
}
