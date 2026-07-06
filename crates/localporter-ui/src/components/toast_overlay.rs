use eframe::egui::{self, Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText};

use crate::state::{ToastLevel, ToastView};

#[derive(Default)]
pub struct ToastOverlay;

impl ToastOverlay {
    pub fn show(&mut self, ctx: &egui::Context, toasts: &[ToastView]) {
        if toasts.is_empty() {
            return;
        }

        egui::Area::new("toast_overlay".into())
            .anchor(Align2::RIGHT_TOP, egui::vec2(-16.0, 16.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down(Align::Max), |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    for toast in toasts {
                        Frame::new()
                            .fill(Self::toast_fill(toast.level))
                            .stroke(Self::toast_stroke(toast.level))
                            .corner_radius(CornerRadius::same(10))
                            .inner_margin(Margin::symmetric(12, 10))
                            .show(ui, |ui| {
                                ui.set_max_width(280.0);
                                ui.label(
                                    RichText::new(&toast.message)
                                        .size(12.0)
                                        .color(Self::toast_text(toast.level)),
                                );
                            });
                    }
                });
            });
    }

    fn toast_fill(level: ToastLevel) -> Color32 {
        match level {
            ToastLevel::Success => Color32::from_rgb(241, 250, 243),
            ToastLevel::Error => Color32::from_rgb(253, 242, 242),
        }
    }

    fn toast_stroke(level: ToastLevel) -> egui::Stroke {
        match level {
            ToastLevel::Success => egui::Stroke::new(1.0, Color32::from_rgb(116, 184, 128)),
            ToastLevel::Error => egui::Stroke::new(1.0, Color32::from_rgb(226, 121, 121)),
        }
    }

    fn toast_text(level: ToastLevel) -> Color32 {
        match level {
            ToastLevel::Success => Color32::from_rgb(30, 84, 42),
            ToastLevel::Error => Color32::from_rgb(138, 33, 33),
        }
    }
}
