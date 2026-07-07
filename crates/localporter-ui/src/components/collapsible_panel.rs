use eframe::egui::{
    self, Align, Color32, CornerRadius, Frame, Layout, Margin, Rect, Sense, Stroke, StrokeKind,
    UiBuilder,
};

#[derive(Default)]
pub struct CollapsiblePanel;

impl CollapsiblePanel {
    const HEADER_MIN_HEIGHT: f32 = 50.0;
    const ICON_WIDTH: f32 = 20.0;
    const HEADER_ACTION_SIZE: f32 = 30.0;
    const HEADER_ACTION_GAP: f32 = 8.0;
    const CONTENT_ICON_SPACING: f32 = 12.0;
    const PANEL_FILL: Color32 = Color32::from_rgb(251, 251, 251);

    pub fn show<Header, HeaderAction, Body, Action>(
        &mut self,
        ui: &mut egui::Ui,
        expanded: &mut bool,
        header: Header,
        header_action: HeaderAction,
        body: Body,
    ) -> Option<Action>
    where
        Header: FnOnce(&mut egui::Ui),
        HeaderAction: FnOnce(&mut egui::Ui, bool) -> Option<Action>,
        Body: FnOnce(&mut egui::Ui) -> Option<Action>,
    {
        let mut action = None;

        Frame::new()
            .fill(Self::PANEL_FILL)
            .stroke(Self::panel_stroke())
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                let width = ui.available_width();
                let (header_rect, response) = ui.allocate_exact_size(
                    egui::vec2(width, Self::HEADER_MIN_HEIGHT),
                    Sense::hover(),
                );
                let pointer_pos = ui.ctx().pointer_hover_pos();
                let header_hovered =
                    pointer_pos.is_some_and(|position| header_rect.contains(position));
                let show_header_action = header_hovered;

                let icon_rect = Rect::from_min_size(
                    egui::pos2(header_rect.right() - Self::ICON_WIDTH, header_rect.top()),
                    egui::vec2(Self::ICON_WIDTH, header_rect.height()),
                );
                let action_rect = Rect::from_min_size(
                    egui::pos2(
                        icon_rect.left() - Self::HEADER_ACTION_GAP - Self::HEADER_ACTION_SIZE,
                        header_rect.top() + (header_rect.height() - Self::HEADER_ACTION_SIZE) * 0.5,
                    ),
                    egui::vec2(Self::HEADER_ACTION_SIZE, Self::HEADER_ACTION_SIZE),
                );
                let content_rect = Rect::from_min_max(
                    header_rect.min,
                    egui::pos2(
                        (action_rect.left() - Self::CONTENT_ICON_SPACING).max(header_rect.left()),
                        header_rect.bottom(),
                    ),
                );
                let toggle_rect = Rect::from_min_max(
                    header_rect.min,
                    egui::pos2(
                        if show_header_action {
                            action_rect.left()
                        } else {
                            header_rect.right()
                        },
                        header_rect.bottom(),
                    ),
                );

                let toggle_response = ui
                    .interact(toggle_rect, response.id.with("toggle"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                let icon_toggle_response = ui
                    .interact(icon_rect, response.id.with("toggle_icon"), Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                ui.scope_builder(UiBuilder::new().max_rect(header_rect), |ui| {
                    ui.set_min_size(header_rect.size());

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(content_rect)
                            .layout(Layout::top_down(Align::Min)),
                        header,
                    );

                    if action.is_none() {
                        ui.scope_builder(
                            UiBuilder::new()
                                .max_rect(action_rect)
                                .layout(Layout::right_to_left(Align::Center)),
                            |ui| {
                                action = header_action(ui, show_header_action);
                            },
                        );
                    }

                    ui.scope_builder(
                        UiBuilder::new()
                            .max_rect(icon_rect)
                            .layout(Layout::right_to_left(Align::Center)),
                        |ui| self.disclosure_icon(ui, *expanded),
                    );
                });

                if toggle_response.clicked() || icon_toggle_response.clicked() {
                    *expanded = !*expanded;
                    ui.ctx().request_repaint();
                }

                if *expanded {
                    ui.add_space(8.0);
                    self.body_separator(ui);
                    ui.add_space(10.0);
                    if action.is_none() {
                        action = body(ui);
                    }
                }
            });

        action
    }

    fn disclosure_icon(&self, ui: &mut egui::Ui, expanded: bool) {
        let icon_rect =
            egui::Rect::from_center_size(ui.max_rect().center(), egui::vec2(16.0, 16.0));
        egui::Image::new(Self::disclosure_icon_source(expanded))
            .fit_to_exact_size(icon_rect.size())
            .tint(Color32::from_rgb(148, 154, 163))
            .paint_at(ui, icon_rect);
    }

    fn body_separator(&self, ui: &mut egui::Ui) {
        let (separator_rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), Sense::hover());
        ui.painter().rect_stroke(
            separator_rect,
            CornerRadius::ZERO,
            Self::panel_stroke(),
            StrokeKind::Middle,
        );
    }

    fn panel_stroke() -> Stroke {
        Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 13))
    }

    fn disclosure_icon_source(expanded: bool) -> egui::ImageSource<'static> {
        if expanded {
            egui::include_image!("../../assets/icons/collapsible/chevron-down.svg")
        } else {
            egui::include_image!("../../assets/icons/collapsible/chevron-right.svg")
        }
    }
}
