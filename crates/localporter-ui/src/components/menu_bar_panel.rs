use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, RichText, Sense, Shape, Stroke,
    ViewportCommand,
};

use crate::{APP_NAME, components::Switch, windows::constants::WINDOW_CORNER_RADIUS};

#[derive(Default)]
pub struct MenuBarPanel;

#[derive(Default)]
pub struct MenuBarPanelFooterResponse {
    pub settings_clicked: bool,
    pub kill_all_clicked: bool,
    pub quit_clicked: bool,
}

impl MenuBarPanel {
    pub const TITLE_BAR_HEIGHT: f32 = 44.0;
    pub const FOOTER_HEIGHT: f32 = 40.0;
    pub const ARROW_HEIGHT: f32 = 10.0;

    const CONTENT_HEIGHT: f32 = 26.0;
    const TITLE_WIDTH: f32 = 140.0;
    const HORIZONTAL_PADDING: f32 = 12.0;
    const VERTICAL_PADDING: f32 = 7.0;
    const BUTTON_HORIZONTAL_PADDING: f32 = 12.0;
    const BUTTON_GAP: f32 = 6.0;
    const ACTIONS_WIDTH: f32 = 148.0;
    const ACTIONS_RIGHT_PADDING: f32 = 8.0;
    const ICON_SIZE: f32 = 12.0;
    const ARROW_WIDTH: f32 = 18.0;
    const ARROW_EDGE_PADDING: f32 = 20.0;
    const TITLE: Color32 = Color32::from_rgb(26, 27, 31);
    const ACTION_TEXT: Color32 = Color32::from_rgb(82, 88, 97);
    const SETTINGS_TEXT: Color32 = Color32::from_rgb(96, 102, 112);
    const KILL_TEXT_ENABLED: Color32 = Color32::from_rgb(177, 49, 49);
    const KILL_TEXT_DISABLED: Color32 = Color32::from_rgb(164, 119, 119);
    const KILL_FILL_ENABLED: Color32 = Color32::from_rgb(255, 240, 240);
    const KILL_FILL_DISABLED: Color32 = Color32::from_rgb(247, 242, 242);
    const KILL_STROKE_ENABLED: Color32 = Color32::from_rgb(237, 196, 196);
    const KILL_STROKE_DISABLED: Color32 = Color32::from_rgb(234, 224, 224);
    const QUIT_TEXT: Color32 = Color32::from_rgb(177, 49, 49);
    const QUIT_FILL: Color32 = Color32::from_rgb(255, 240, 240);
    const QUIT_STROKE: Color32 = Color32::from_rgb(237, 196, 196);
    const PANEL_BACKGROUND: Color32 = Color32::from_rgb(251, 251, 251);

    pub fn paint_frame(
        &self,
        ui: &mut egui::Ui,
        outer_rect: egui::Rect,
        arrow_tip_x: Option<f32>,
    ) -> egui::Rect {
        let body_rect = egui::Rect::from_min_max(
            egui::pos2(outer_rect.left(), outer_rect.top() + Self::ARROW_HEIGHT),
            outer_rect.max,
        );

        ui.painter().rect_filled(
            body_rect,
            CornerRadius::same(WINDOW_CORNER_RADIUS),
            Self::PANEL_BACKGROUND,
        );

        if let Some(arrow_tip_x) = arrow_tip_x {
            self.paint_arrow(ui, outer_rect, body_rect, arrow_tip_x);
        }

        ui.painter().rect_stroke(
            body_rect,
            CornerRadius::same(WINDOW_CORNER_RADIUS),
            Stroke::new(1.0, Self::border_color()),
            egui::StrokeKind::Middle,
        );

        body_rect
    }

    pub fn show_header(&mut self, ui: &mut egui::Ui, show_all_enabled: &mut bool) -> bool {
        let width = ui.available_width();
        let mut changed = false;
        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(width, Self::TITLE_BAR_HEIGHT), Sense::hover());
        let inner_rect = outer_rect.shrink2(egui::vec2(12.0, 6.0));

        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
            ui.set_min_size(inner_rect.size());

            ui.horizontal(|ui| {
                let drag_width =
                    (ui.available_width() - Self::TITLE_WIDTH - Self::ACTIONS_WIDTH).max(0.0);
                let (title_rect, _) = ui.allocate_exact_size(
                    egui::vec2(Self::TITLE_WIDTH, Self::CONTENT_HEIGHT),
                    Sense::hover(),
                );
                ui.painter().text(
                    title_rect.left_center(),
                    Align2::LEFT_CENTER,
                    APP_NAME,
                    FontId::proportional(14.0),
                    Self::TITLE,
                );
                let (_drag_rect, drag_response) = ui.allocate_exact_size(
                    egui::vec2(drag_width, Self::CONTENT_HEIGHT),
                    Sense::click_and_drag(),
                );

                if drag_response.drag_started() {
                    ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
                }

                ui.allocate_ui_with_layout(
                    egui::vec2(Self::ACTIONS_WIDTH, Self::CONTENT_HEIGHT),
                    Layout::right_to_left(Align::Center),
                    |ui| {
                        let outer_rect = ui.max_rect();
                        let inner_rect = egui::Rect::from_min_max(
                            outer_rect.min,
                            egui::pos2(
                                (outer_rect.max.x - Self::ACTIONS_RIGHT_PADDING)
                                    .max(outer_rect.min.x),
                                outer_rect.max.y,
                            ),
                        );

                        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.spacing_mut().item_spacing.x = 12.0;
                                changed = ui.add(Switch::new(show_all_enabled)).changed();
                                ui.label(
                                    RichText::new("Show all")
                                        .size(13.0)
                                        .color(Self::ACTION_TEXT)
                                        .strong(),
                                );
                            });
                        });
                    },
                );
            });
        });

        let separator_y = outer_rect.bottom() - 0.5;
        ui.painter().line_segment(
            [
                egui::pos2(outer_rect.left(), separator_y),
                egui::pos2(outer_rect.right(), separator_y),
            ],
            Stroke::new(1.0, Self::border_color()),
        );

        changed
    }

    pub fn show_footer(
        &mut self,
        ui: &mut egui::Ui,
        killable_count: usize,
    ) -> MenuBarPanelFooterResponse {
        let width = ui.available_width();
        let mut response = MenuBarPanelFooterResponse::default();
        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(width, Self::FOOTER_HEIGHT), Sense::hover());
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

            let settings_width = self.action_button_width(ui, "Settings");
            let kill_width = self.kill_button_width(ui, killable_count);
            let quit_width = self.action_button_width(ui, "Quit");
            let right_group_width = kill_width + Self::BUTTON_GAP + quit_width;
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

                    if self.quit_button(ui).clicked() {
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

    fn border_color() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 13)
    }

    fn settings_fill_hovered() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 8)
    }

    fn action_button_width(&self, ui: &egui::Ui, text: &str) -> f32 {
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            FontId::proportional(11.5),
            Color32::WHITE,
        );
        Self::BUTTON_HORIZONTAL_PADDING * 2.0 + galley.size().x
    }

    fn settings_button(&self, ui: &mut egui::Ui) -> egui::Response {
        self.action_button(
            ui,
            "Settings",
            None,
            Color32::TRANSPARENT,
            Stroke::NONE,
            Self::SETTINGS_TEXT,
            true,
        )
    }

    fn kill_all_button(
        &self,
        ui: &mut egui::Ui,
        killable_count: usize,
        enabled: bool,
    ) -> egui::Response {
        let fill = if enabled {
            Self::KILL_FILL_ENABLED
        } else {
            Self::KILL_FILL_DISABLED
        };
        let stroke = if enabled {
            Self::KILL_STROKE_ENABLED
        } else {
            Self::KILL_STROKE_DISABLED
        };
        let text_color = if enabled {
            Self::KILL_TEXT_ENABLED
        } else {
            Self::KILL_TEXT_DISABLED
        };
        let label = format!("Kill killable({killable_count})");

        self.action_button(
            ui,
            &label,
            Some(Self::kill_icon_source()),
            fill,
            Stroke::new(1.0, stroke),
            text_color,
            enabled,
        )
    }

    fn quit_button(&self, ui: &mut egui::Ui) -> egui::Response {
        self.action_button(
            ui,
            "Quit",
            None,
            Self::QUIT_FILL,
            Stroke::new(1.0, Self::QUIT_STROKE),
            Self::QUIT_TEXT,
            true,
        )
    }

    fn kill_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/bottom-bar/kill.svg")
    }

    fn kill_button_width(&self, ui: &egui::Ui, killable_count: usize) -> f32 {
        let label = format!("Kill killable({killable_count})");
        self.action_button_width_with_icon(ui, &label, true)
    }

    fn action_button(
        &self,
        ui: &mut egui::Ui,
        text: &str,
        icon: Option<egui::ImageSource<'static>>,
        fill: Color32,
        stroke: Stroke,
        text_color: Color32,
        enabled: bool,
    ) -> egui::Response {
        let galley =
            ui.painter()
                .layout_no_wrap(text.to_owned(), FontId::proportional(11.5), text_color);
        let has_icon = icon.is_some();
        let desired_size = egui::vec2(
            self.action_button_width_with_icon(ui, text, has_icon),
            Self::CONTENT_HEIGHT,
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

    fn action_button_width_with_icon(&self, ui: &egui::Ui, text: &str, has_icon: bool) -> f32 {
        let galley = ui.painter().layout_no_wrap(
            text.to_owned(),
            FontId::proportional(11.5),
            Color32::WHITE,
        );
        let icon_width = if has_icon {
            Self::ICON_SIZE + Self::BUTTON_GAP
        } else {
            0.0
        };

        Self::BUTTON_HORIZONTAL_PADDING * 2.0 + icon_width + galley.size().x
    }

    fn paint_arrow(
        &self,
        ui: &mut egui::Ui,
        outer_rect: egui::Rect,
        body_rect: egui::Rect,
        arrow_tip_x: f32,
    ) {
        let arrow_tip_x = arrow_tip_x.clamp(
            body_rect.left() + Self::ARROW_EDGE_PADDING,
            body_rect.right() - Self::ARROW_EDGE_PADDING,
        );
        let tip = egui::pos2(arrow_tip_x, outer_rect.top() + 0.5);
        let base_y = body_rect.top() + 0.5;
        let left = egui::pos2(arrow_tip_x - Self::ARROW_WIDTH * 0.5, base_y);
        let right = egui::pos2(arrow_tip_x + Self::ARROW_WIDTH * 0.5, base_y);

        ui.painter().add(Shape::convex_polygon(
            vec![left, tip, right],
            Self::PANEL_BACKGROUND,
            Stroke::NONE,
        ));
        ui.painter()
            .line_segment([left, right], Stroke::new(2.0, Self::PANEL_BACKGROUND));
        ui.painter()
            .line_segment([left, tip], Stroke::new(1.0, Self::border_color()));
        ui.painter()
            .line_segment([tip, right], Stroke::new(1.0, Self::border_color()));
    }
}
