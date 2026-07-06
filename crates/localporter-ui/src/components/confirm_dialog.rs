use eframe::egui::{
    self, Align, Align2, Button, Color32, CornerRadius, Frame, Layout, Margin, RichText, Sense,
    Stroke,
};

#[derive(Default)]
pub struct ConfirmDialog;

#[derive(Default)]
pub struct ConfirmDialogResponse {
    pub confirmed: bool,
    pub canceled: bool,
}

impl ConfirmDialog {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        title: &str,
        message: &str,
        confirm_label: &str,
    ) -> ConfirmDialogResponse {
        let mut response = ConfirmDialogResponse::default();
        let screen_rect = ctx.content_rect();
        let mut backdrop_clicked = false;
        egui::Area::new("confirm_dialog_backdrop".into())
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .show(ctx, |ui| {
                let (rect, backdrop_response) =
                    ui.allocate_exact_size(screen_rect.size(), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::ZERO,
                    Color32::from_rgba_unmultiplied(15, 23, 42, 56),
                );
                backdrop_clicked = backdrop_response.clicked();
            });

        let mut open = true;
        egui::Window::new(title)
            .order(egui::Order::Tooltip)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .fixed_size(egui::vec2(360.0, 176.0))
            .frame(
                Frame::window(&ctx.style_of(egui::Theme::Light))
                    .fill(Color32::from_rgb(252, 252, 252))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, 16),
                    ))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::symmetric(18, 16)),
            )
            .open(&mut open)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 14.0;

                ui.label(
                    RichText::new(title)
                        .size(17.0)
                        .strong()
                        .color(Color32::from_rgb(29, 35, 43)),
                );
                ui.label(
                    RichText::new(message)
                        .size(12.5)
                        .color(Color32::from_rgb(96, 102, 112)),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.horizontal(|ui| {
                        if self.cancel_button(ui).clicked() {
                            response.canceled = true;
                        }
                        if self.confirm_button(ui, confirm_label).clicked() {
                            response.confirmed = true;
                        }
                    });
                });
            });

        if backdrop_clicked {
            response.canceled = true;
        }
        if !open {
            response.canceled = true;
        }

        response
    }

    fn cancel_button(&self, ui: &mut egui::Ui) -> egui::Response {
        ui.add_sized(
            [76.0, 32.0],
            Button::new(
                RichText::new("Cancel")
                    .size(12.0)
                    .color(Color32::from_rgb(82, 88, 97)),
            )
            .fill(Color32::from_rgb(247, 248, 250))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 13),
            ))
            .corner_radius(CornerRadius::same(10)),
        )
    }

    fn confirm_button(&self, ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add_sized(
            [120.0, 32.0],
            Button::new(
                RichText::new(label)
                    .size(12.0)
                    .strong()
                    .color(Color32::WHITE),
            )
            .fill(Color32::from_rgb(217, 61, 61))
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(10)),
        )
    }
}
