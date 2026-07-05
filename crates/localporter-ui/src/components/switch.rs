use eframe::egui::{self, Color32, CornerRadius, Response, Sense, Stroke, Widget};

pub struct Switch<'a> {
    value: &'a mut bool,
}

impl<'a> Switch<'a> {
    const WIDTH: f32 = 30.0;
    const HEIGHT: f32 = 18.0;
    const KNOB_PADDING: f32 = 2.0;

    pub fn new(value: &'a mut bool) -> Self {
        Self { value }
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut egui::Ui) -> Response {
        let desired_size = egui::vec2(Self::WIDTH, Self::HEIGHT);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            *self.value = !*self.value;
            response.mark_changed();
        }

        let how_on = ui.ctx().animate_bool(response.id, *self.value);
        let radius = rect.height() * 0.5;
        let knob_radius = radius - Self::KNOB_PADDING;
        let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let knob_center = egui::pos2(knob_x, rect.center().y);
        let track_fill = if how_on > 0.5 {
            Color32::from_rgb(26, 27, 31)
        } else {
            Color32::from_rgba_unmultiplied(26, 27, 31, 28)
        };
        let track_stroke = Stroke::new(
            1.0,
            if *self.value {
                Color32::from_rgb(26, 27, 31)
            } else {
                Color32::from_rgba_unmultiplied(26, 27, 31, 36)
            },
        );

        ui.painter()
            .rect_filled(rect, CornerRadius::same(255), track_fill);
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(255),
            track_stroke,
            egui::StrokeKind::Middle,
        );
        ui.painter()
            .circle_filled(knob_center, knob_radius, Color32::WHITE);
        ui.painter().circle_stroke(
            knob_center,
            knob_radius,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(26, 27, 31, 18)),
        );

        response
    }
}
