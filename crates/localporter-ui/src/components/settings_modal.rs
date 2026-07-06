use eframe::egui::{
    self, Align, Align2, Button, Color32, ComboBox, CornerRadius, Frame, Label, Layout, Margin,
    RichText, Sense, Stroke, UiBuilder,
};

use crate::components::Switch;
use crate::state::{AppSettings, KillBehavior, RefreshInterval};

#[derive(Default)]
pub struct SettingsModal;

#[derive(Default)]
pub struct SettingsModalResponse {
    pub close_requested: bool,
    pub save_requested: bool,
}

impl SettingsModal {
    const WINDOW_WIDTH: f32 = 440.0;
    const WINDOW_HEIGHT: f32 = 304.0;
    const SECTION_GAP: f32 = 16.0;
    const OPTION_GAP: f32 = 8.0;
    const BEHAVIOR_CARD_HEIGHT: f32 = 68.0;

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut AppSettings,
        startup_supported: bool,
        version: &str,
    ) -> SettingsModalResponse {
        let mut response = SettingsModalResponse::default();
        let screen_rect = ctx.content_rect();
        let mut backdrop_clicked = false;
        egui::Area::new("settings_modal_backdrop".into())
            .order(egui::Order::Middle)
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
        egui::Window::new("Settings")
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .fixed_size(egui::vec2(Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT))
            .frame(
                Frame::window(&ctx.style_of(egui::Theme::Light))
                    .fill(Color32::from_rgb(252, 252, 252))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(0, 0, 0, 16),
                    ))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::symmetric(20, 18)),
            )
            .open(&mut open)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = Self::SECTION_GAP;

                self.header(ui, version, &mut response);
                self.section(ui, "Refresh Interval", |ui| {
                    ComboBox::from_id_salt("settings_refresh_interval")
                        .selected_text(settings.refresh_interval.label())
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for interval in RefreshInterval::ALL {
                                ui.selectable_value(
                                    &mut settings.refresh_interval,
                                    interval,
                                    interval.label(),
                                );
                            }
                        });
                });

                self.section(ui, "Kill Behavior", |ui| {
                    let (row_rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), Self::BEHAVIOR_CARD_HEIGHT),
                        Sense::hover(),
                    );
                    let card_width = ((row_rect.width() - Self::OPTION_GAP).max(0.0)) / 2.0;

                    for (index, behavior) in KillBehavior::ALL.into_iter().enumerate() {
                        let selected = settings.kill_behavior == behavior;
                        let min = egui::pos2(
                            row_rect.left() + index as f32 * (card_width + Self::OPTION_GAP),
                            row_rect.top(),
                        );
                        let card_rect = egui::Rect::from_min_size(
                            min,
                            egui::vec2(card_width, row_rect.height()),
                        );

                        if self.behavior_card(ui, behavior, selected, card_rect) && !selected {
                            settings.kill_behavior = behavior;
                        }
                    }
                });

                self.section(ui, "Launch At Startup", |ui| {
                    if startup_supported {
                        self.switch_row(
                            ui,
                            "Open LocalPorter automatically when signing in",
                            &mut settings.launch_at_startup,
                        );
                    } else {
                        ui.label(
                            RichText::new("Not supported on this platform.")
                                .size(12.0)
                                .color(Color32::from_rgb(120, 126, 134)),
                        );
                    }
                });
            });

        if backdrop_clicked {
            response.close_requested = true;
        }
        if !open {
            response.close_requested = true;
        }

        response
    }

    fn header(&self, ui: &mut egui::Ui, version: &str, response: &mut SettingsModalResponse) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Settings")
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(29, 35, 43)),
                );
                ui.label(
                    RichText::new(format!("Version {version}"))
                        .size(12.0)
                        .color(Color32::from_rgb(120, 126, 134)),
                );
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if self.option_button(ui, "Done", false).clicked() {
                    response.save_requested = true;
                    response.close_requested = true;
                }
            });
        });
    }

    fn section(&self, ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui)) {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.label(
                RichText::new(title)
                    .size(12.5)
                    .strong()
                    .color(Color32::from_rgb(94, 100, 108)),
            );
            add_contents(ui);
        });
    }

    fn behavior_card(
        &self,
        ui: &mut egui::Ui,
        behavior: KillBehavior,
        selected: bool,
        rect: egui::Rect,
    ) -> bool {
        let background_response = ui.interact(
            rect,
            ui.id().with(("kill_behavior_card", behavior.label())),
            Sense::click(),
        );
        let stroke_color = if selected {
            Color32::from_rgb(157, 184, 255)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 13)
        };
        let fill_color = if selected {
            Color32::from_rgb(235, 242, 255)
        } else {
            Color32::from_rgb(252, 252, 252)
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(12), fill_color);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(12),
            Stroke::new(1.0, stroke_color),
            egui::StrokeKind::Middle,
        );

        let content_rect = rect.shrink2(egui::vec2(12.0, 10.0));
        let mut clicked = background_response.clicked();
        ui.scope_builder(
            UiBuilder::new()
                .max_rect(content_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.set_clip_rect(content_rect);
                ui.spacing_mut().item_spacing.y = 6.0;
                ui.set_width(content_rect.width());
                clicked |= ui
                    .add(
                        Label::new(
                            RichText::new(behavior.label())
                                .size(12.5)
                                .strong()
                                .color(Self::option_text(selected)),
                        )
                        .sense(Sense::click()),
                    )
                    .clicked();
                clicked |= ui
                    .add(
                        Label::new(
                            RichText::new(behavior.description())
                                .size(11.0)
                                .color(Color32::from_rgb(120, 126, 134)),
                        )
                        .sense(Sense::click())
                        .wrap()
                        .halign(Align::LEFT),
                    )
                    .clicked();
            },
        );

        clicked
    }

    fn switch_row(&self, ui: &mut egui::Ui, label: &str, value: &mut bool) -> egui::Response {
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(label)
                    .size(12.5)
                    .color(Color32::from_rgb(67, 72, 80)),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add(Switch::new(value))
            })
            .inner
        })
        .inner
    }

    fn option_button(&self, ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
        ui.add(
            Button::new(
                RichText::new(label)
                    .size(12.0)
                    .color(Self::option_text(selected)),
            )
            .min_size(egui::vec2(0.0, 30.0))
            .fill(Self::option_fill(selected))
            .stroke(Stroke::new(1.0, Self::option_stroke(selected)))
            .corner_radius(CornerRadius::same(10)),
        )
    }

    fn option_fill(selected: bool) -> Color32 {
        if selected {
            Color32::from_rgb(235, 242, 255)
        } else {
            Color32::from_rgb(252, 252, 252)
        }
    }

    fn option_stroke(selected: bool) -> Color32 {
        if selected {
            Color32::from_rgb(157, 184, 255)
        } else {
            Color32::from_rgba_unmultiplied(0, 0, 0, 13)
        }
    }

    fn option_text(selected: bool) -> Color32 {
        if selected {
            Color32::from_rgb(41, 82, 173)
        } else {
            Color32::from_rgb(67, 72, 80)
        }
    }
}
