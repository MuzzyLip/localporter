use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Frame, Layout, RichText, Sense, Stroke, TextEdit,
};

#[derive(Default)]
pub struct FilterBar;

impl FilterBar {
    const HEIGHT: f32 = 30.0;
    const INPUT_HEIGHT: f32 = 30.0;
    const CLEAR_BUTTON_SIZE: f32 = 20.0;
    const CLEAR_ICON_SIZE: f32 = 12.0;
    const INPUT_FONT_SIZE: f32 = 13.0;
    const INPUT_FILL: Color32 = Color32::from_rgb(247, 248, 250);
    const INPUT_TEXT: Color32 = Color32::from_rgb(42, 47, 54);
    const HINT_TEXT: Color32 = Color32::from_rgb(142, 148, 157);
    const BUTTON_TEXT: Color32 = Color32::from_rgb(67, 72, 80);

    pub fn show(&mut self, ui: &mut egui::Ui, query: &mut String) {
        let width = ui.available_width();
        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(width, Self::HEIGHT), egui::Sense::hover());
        let input_rect = outer_rect;

        ui.scope_builder(egui::UiBuilder::new().max_rect(input_rect), |ui| {
            ui.painter().rect(
                input_rect,
                CornerRadius::same(0),
                Self::INPUT_FILL,
                Stroke::new(1.0, Self::input_stroke()),
                egui::StrokeKind::Inside,
            );

            let content_rect = input_rect.shrink2(egui::vec2(10.0, 0.0));
            ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let has_query = !query.trim().is_empty();
                    let clear_clicked = self.clear_button(ui, has_query).clicked();

                    if clear_clicked {
                        query.clear();
                    }

                    let response = ui.add_sized(
                        [ui.available_width(), Self::INPUT_HEIGHT],
                        TextEdit::singleline(query)
                            .id_salt("filter_bar_search_input")
                            .font(FontId::proportional(Self::INPUT_FONT_SIZE))
                            .vertical_align(Align::Center)
                            .min_size(egui::vec2(0.0, Self::INPUT_HEIGHT))
                            .hint_text(
                                RichText::new("Search process or :port / :port-:port")
                                    .color(Self::HINT_TEXT),
                            )
                            .text_color(Self::INPUT_TEXT)
                            .frame(Frame::NONE),
                    );

                    if clear_clicked {
                        response.request_focus();
                        ui.ctx().request_repaint();
                    }
                });
            });
        });
    }

    fn input_stroke() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 16)
    }

    fn button_hover_fill() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 8)
    }

    fn clear_button(&self, ui: &mut egui::Ui, enabled: bool) -> egui::Response {
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(Self::CLEAR_BUTTON_SIZE, Self::CLEAR_BUTTON_SIZE),
            sense,
        );

        if enabled && response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), Self::button_hover_fill());
        }

        let tint = if enabled {
            Self::BUTTON_TEXT
        } else {
            Color32::TRANSPARENT
        };
        let icon_rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(Self::CLEAR_ICON_SIZE, Self::CLEAR_ICON_SIZE),
        );
        ui.put(
            icon_rect,
            egui::Image::new(Self::clear_icon_source())
                .fit_to_exact_size(icon_rect.size())
                .tint(tint),
        );

        response
    }

    fn clear_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/filter-bar/clear.svg")
    }
}
