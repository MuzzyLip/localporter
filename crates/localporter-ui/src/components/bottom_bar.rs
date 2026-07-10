use eframe::egui::{self, Align, Color32, CornerRadius, Layout, Sense, Stroke};

#[derive(Default)]
pub struct BottomBar;

#[derive(Default)]
pub struct BottomBarResponse {
    pub settings_clicked: bool,
    pub kill_all_clicked: bool,
    pub quit_clicked: bool,
}

struct ActionButtonStyle {
    icon: Option<egui::ImageSource<'static>>,
    fill: Color32,
    stroke: Stroke,
    text_color: Color32,
    enabled: bool,
    fixed_height: bool,
}

impl BottomBar {
    pub const HEIGHT: f32 = 40.0;
    const CONTENT_HEIGHT: f32 = 26.0;
    const HORIZONTAL_PADDING: f32 = 12.0;
    const VERTICAL_PADDING: f32 = 7.0;
    const BUTTON_HORIZONTAL_PADDING: f32 = 12.0;
    const BUTTON_GAP: f32 = 6.0;
    const SETTINGS_TEXT: Color32 = Color32::from_rgb(96, 102, 112);
    const ACTION_TEXT_ENABLED: Color32 = Color32::from_rgb(177, 49, 49);
    const ACTION_TEXT_DISABLED: Color32 = Color32::from_rgb(164, 119, 119);
    const ACTION_FILL_ENABLED: Color32 = Color32::from_rgb(255, 240, 240);
    const ACTION_FILL_DISABLED: Color32 = Color32::from_rgb(247, 242, 242);
    const ACTION_STROKE_ENABLED: Color32 = Color32::from_rgb(237, 196, 196);
    const ACTION_STROKE_DISABLED: Color32 = Color32::from_rgb(234, 224, 224);
    const QUIT_TEXT: Color32 = Color32::from_rgb(177, 49, 49);
    const QUIT_FILL: Color32 = Color32::from_rgb(255, 240, 240);
    const QUIT_STROKE: Color32 = Color32::from_rgb(237, 196, 196);
    const ICON_SIZE: f32 = 12.0;

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        killable_count: usize,
        show_quit: bool,
    ) -> BottomBarResponse {
        let mut response = BottomBarResponse::default();
        let width = ui.available_width();
        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(width, Self::HEIGHT), Sense::hover());
        let inner_rect =
            outer_rect.shrink2(egui::vec2(Self::HORIZONTAL_PADDING, Self::VERTICAL_PADDING));

        let separator_y = outer_rect.top() + 0.5;
        ui.painter().line_segment(
            [
                egui::pos2(outer_rect.left(), separator_y),
                egui::pos2(outer_rect.right(), separator_y),
            ],
            Stroke::new(1.0, Self::border_color()),
        );

        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
            ui.set_min_size(inner_rect.size());
            let settings_width = self.settings_button_width(ui);
            let kill_width = self.kill_button_width(ui, killable_count);
            let quit_width = if show_quit {
                self.quit_button_width(ui)
            } else {
                0.0
            };
            let right_group_width = if show_quit {
                kill_width + Self::BUTTON_GAP + quit_width
            } else {
                kill_width
            };
            let left_rect = egui::Rect::from_min_size(
                inner_rect.left_top(),
                egui::vec2(settings_width, Self::CONTENT_HEIGHT),
            );
            let right_rect = egui::Rect::from_min_size(
                egui::pos2(inner_rect.right() - right_group_width, inner_rect.top()),
                egui::vec2(right_group_width, Self::CONTENT_HEIGHT),
            );

            ui.scope_builder(egui::UiBuilder::new().max_rect(left_rect), |ui| {
                ui.set_min_size(left_rect.size());
                if self.settings_button(ui).clicked() {
                    response.settings_clicked = true;
                }
            });

            ui.scope_builder(egui::UiBuilder::new().max_rect(right_rect), |ui| {
                ui.set_min_size(right_rect.size());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = Self::BUTTON_GAP;

                    if show_quit && self.quit_button(ui).clicked() {
                        response.quit_clicked = true;
                    }

                    if self
                        .kill_all_button(ui, killable_count, killable_count > 0)
                        .clicked()
                    {
                        response.kill_all_clicked = true;
                    }
                });
            });
        });

        response
    }

    fn settings_button(&self, ui: &mut egui::Ui) -> egui::Response {
        self.inline_action_button(
            ui,
            "Settings",
            ActionButtonStyle {
                icon: None,
                fill: Color32::TRANSPARENT,
                stroke: Stroke::NONE,
                text_color: Self::SETTINGS_TEXT,
                enabled: true,
                fixed_height: false,
            },
        )
    }

    fn kill_all_button(
        &self,
        ui: &mut egui::Ui,
        killable_count: usize,
        enabled: bool,
    ) -> egui::Response {
        let fill = if enabled {
            Self::ACTION_FILL_ENABLED
        } else {
            Self::ACTION_FILL_DISABLED
        };
        let stroke = if enabled {
            Self::ACTION_STROKE_ENABLED
        } else {
            Self::ACTION_STROKE_DISABLED
        };
        let text_color = if enabled {
            Self::ACTION_TEXT_ENABLED
        } else {
            Self::ACTION_TEXT_DISABLED
        };
        let label = format!("Kill killable({killable_count})");

        self.inline_action_button(
            ui,
            &label,
            ActionButtonStyle {
                icon: Some(Self::kill_icon_source()),
                fill,
                stroke: Stroke::new(1.0, stroke),
                text_color,
                enabled,
                fixed_height: true,
            },
        )
    }

    fn kill_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/bottom-bar/kill.svg")
    }

    fn border_color() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 13)
    }

    fn settings_fill_hovered() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 8)
    }

    fn kill_button_width(&self, ui: &egui::Ui, killable_count: usize) -> f32 {
        let text = format!("Kill killable({killable_count})");
        self.action_button_width(ui, &text, true)
    }

    fn settings_button_width(&self, ui: &egui::Ui) -> f32 {
        self.action_button_width(ui, "Settings", false)
    }

    fn quit_button_width(&self, ui: &egui::Ui) -> f32 {
        self.action_button_width(ui, "Quit", false)
    }

    fn action_button_width(&self, ui: &egui::Ui, text: &str, has_icon: bool) -> f32 {
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(11.5),
            Color32::WHITE,
        );
        let icon_width = if has_icon {
            Self::ICON_SIZE + Self::BUTTON_GAP
        } else {
            0.0
        };
        Self::BUTTON_HORIZONTAL_PADDING * 2.0 + icon_width + galley.size().x
    }

    fn inline_action_button(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        style: ActionButtonStyle,
    ) -> egui::Response {
        let ActionButtonStyle {
            icon,
            fill,
            stroke,
            text_color,
            enabled,
            fixed_height,
        } = style;
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            egui::FontId::proportional(11.5),
            text_color,
        );
        let has_icon = icon.is_some();
        let desired_size = egui::vec2(
            self.action_button_width(ui, text, has_icon),
            if fixed_height {
                Self::CONTENT_HEIGHT
            } else {
                28.0
            },
        );
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(desired_size, sense);
        let fill = if response.hovered() && stroke == Stroke::NONE {
            Self::settings_fill_hovered()
        } else {
            fill
        };

        ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
        if stroke != Stroke::NONE {
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                stroke,
                egui::StrokeKind::Middle,
            );
        }

        let mut text_left = rect.left() + Self::BUTTON_HORIZONTAL_PADDING;
        if let Some(icon) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(
                    rect.left() + Self::BUTTON_HORIZONTAL_PADDING + Self::ICON_SIZE * 0.5,
                    rect.center().y,
                ),
                egui::vec2(Self::ICON_SIZE, Self::ICON_SIZE),
            );
            ui.put(
                icon_rect,
                egui::Image::new(icon)
                    .fit_to_exact_size(icon_rect.size())
                    .tint(text_color),
            );
            text_left = icon_rect.right() + Self::BUTTON_GAP;
        }

        ui.painter().galley(
            egui::pos2(text_left, rect.center().y - galley.size().y * 0.5),
            galley,
            text_color,
        );

        response
    }

    fn quit_button(&self, ui: &mut egui::Ui) -> egui::Response {
        self.inline_action_button(
            ui,
            "Quit",
            ActionButtonStyle {
                icon: None,
                fill: Self::QUIT_FILL,
                stroke: Stroke::new(1.0, Self::QUIT_STROKE),
                text_color: Self::QUIT_TEXT,
                enabled: true,
                fixed_height: true,
            },
        )
    }
}
