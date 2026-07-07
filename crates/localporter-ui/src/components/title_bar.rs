use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, RichText, Sense, Stroke,
    ViewportCommand,
};

use crate::{APP_NAME, components::Switch};

#[derive(Default)]
pub struct TitleBar;

impl TitleBar {
    const HEIGHT: f32 = 44.0;
    const BUTTON_HEIGHT: f32 = 24.0;
    const WINDOWS_BUTTON_WIDTH: f32 = 24.0;
    const WINDOWS_BUTTON_GAP: f32 = 4.0;
    const WINDOWS_CLUSTER_WIDTH: f32 =
        Self::WINDOWS_BUTTON_WIDTH * 3.0 + Self::WINDOWS_BUTTON_GAP * 2.0;
    const MAC_BUTTON_SIZE: f32 = 12.0;
    const MAC_BUTTON_GAP: f32 = 8.0;
    const MAC_CLUSTER_WIDTH: f32 = Self::MAC_BUTTON_SIZE * 3.0 + Self::MAC_BUTTON_GAP * 2.0;
    const TITLE_WIDTH: f32 = 140.0;
    const ACTIONS_WIDTH: f32 = 148.0;
    const ACTIONS_RIGHT_PADDING: f32 = 8.0;
    const TITLE: Color32 = Color32::from_rgb(26, 27, 31);
    const ACTION_TEXT: Color32 = Color32::from_rgb(82, 88, 97);
    const DESKTOP_ICON_SIZE: f32 = 12.0;
    const MAC_ICON_SIZE: f32 = 8.0;

    pub fn show(&mut self, ui: &mut egui::Ui, show_all_enabled: &mut bool) -> bool {
        let width = ui.available_width();
        let maximized = ui
            .ctx()
            .input(|input| input.viewport().maximized.unwrap_or(false));
        let mut changed = false;

        let (outer_rect, _) =
            ui.allocate_exact_size(egui::vec2(width, Self::HEIGHT), Sense::hover());
        let inner_rect = outer_rect.shrink2(egui::vec2(12.0, 6.0));

        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
            ui.set_min_size(inner_rect.size());

            if cfg!(target_os = "macos") {
                changed = self.macos_layout(ui, maximized, show_all_enabled);
            } else {
                changed = self.desktop_layout(ui, maximized, show_all_enabled);
            }
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

    fn border_color() -> Color32 {
        Color32::from_rgba_unmultiplied(0, 0, 0, 13)
    }

    fn macos_layout(
        &mut self,
        ui: &mut egui::Ui,
        maximized: bool,
        show_all_enabled: &mut bool,
    ) -> bool {
        let drag_width =
            (ui.available_width() - Self::MAC_CLUSTER_WIDTH - Self::ACTIONS_WIDTH).max(0.0);
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Self::MAC_BUTTON_GAP;

            ui.allocate_ui_with_layout(
                egui::vec2(Self::MAC_CLUSTER_WIDTH, Self::BUTTON_HEIGHT),
                Layout::left_to_right(Align::Center),
                |ui| {
                    let close = self.allocate_macos_button(ui);
                    let minimize = self.allocate_macos_button(ui);
                    let maximize = self.allocate_macos_button(ui);
                    let show_icons = close.response.hovered()
                        || minimize.response.hovered()
                        || maximize.response.hovered();

                    self.paint_macos_button(ui, &close, MacButtonKind::Close, show_icons);
                    self.paint_macos_button(ui, &minimize, MacButtonKind::Minimize, show_icons);
                    self.paint_macos_button(ui, &maximize, MacButtonKind::Maximize, show_icons);

                    if close.response.clicked() {
                        self.send_action(ui.ctx(), MacButtonKind::Close.into(), maximized);
                    }

                    if minimize.response.clicked() {
                        self.send_action(ui.ctx(), MacButtonKind::Minimize.into(), maximized);
                    }

                    if maximize.response.clicked() {
                        self.send_action(ui.ctx(), MacButtonKind::Maximize.into(), maximized);
                    }
                },
            );

            self.drag_region(ui, drag_width, maximized, Align2::CENTER_CENTER, true);

            ui.allocate_ui_with_layout(
                egui::vec2(Self::ACTIONS_WIDTH, Self::BUTTON_HEIGHT),
                Layout::right_to_left(Align::Center),
                |ui| changed = self.action_group(ui, show_all_enabled),
            );
        });

        changed
    }

    fn desktop_layout(
        &mut self,
        ui: &mut egui::Ui,
        maximized: bool,
        show_all_enabled: &mut bool,
    ) -> bool {
        let drag_width = (ui.available_width()
            - Self::TITLE_WIDTH
            - Self::ACTIONS_WIDTH
            - Self::WINDOWS_CLUSTER_WIDTH)
            .max(0.0);
        let mut changed = false;

        ui.horizontal(|ui| {
            self.drag_region(ui, Self::TITLE_WIDTH, maximized, Align2::LEFT_CENTER, true);

            self.drag_region(ui, drag_width, maximized, Align2::CENTER_CENTER, false);

            ui.allocate_ui_with_layout(
                egui::vec2(Self::ACTIONS_WIDTH, Self::BUTTON_HEIGHT),
                Layout::right_to_left(Align::Center),
                |ui| changed = self.action_group(ui, show_all_enabled),
            );

            ui.allocate_ui_with_layout(
                egui::vec2(Self::WINDOWS_CLUSTER_WIDTH, Self::BUTTON_HEIGHT),
                Layout::right_to_left(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = Self::WINDOWS_BUTTON_GAP;

                    self.desktop_button(ui, DesktopButtonKind::Close, maximized);
                    self.desktop_button(ui, DesktopButtonKind::Maximize, maximized);
                    self.desktop_button(ui, DesktopButtonKind::Minimize, maximized);
                },
            );
        });

        changed
    }

    fn drag_region(
        &self,
        ui: &mut egui::Ui,
        width: f32,
        maximized: bool,
        title_align: Align2,
        show_title: bool,
    ) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width, Self::BUTTON_HEIGHT),
            Sense::click_and_drag(),
        );

        if response.drag_started() {
            ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        }

        if response.double_clicked() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }

        if show_title {
            let anchor = match title_align {
                Align2::LEFT_CENTER => rect.left_center(),
                _ => rect.center(),
            };

            ui.painter().text(
                anchor,
                title_align,
                APP_NAME,
                FontId::proportional(14.0),
                Self::TITLE,
            );
        }
    }

    fn action_group(&mut self, ui: &mut egui::Ui, show_all_enabled: &mut bool) -> bool {
        let outer_rect = ui.max_rect();
        let inner_rect = egui::Rect::from_min_max(
            outer_rect.min,
            egui::pos2(
                (outer_rect.max.x - Self::ACTIONS_RIGHT_PADDING).max(outer_rect.min.x),
                outer_rect.max.y,
            ),
        );
        let mut changed = false;

        ui.scope_builder(egui::UiBuilder::new().max_rect(inner_rect), |ui| {
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                changed = ui.add(Switch::new(show_all_enabled)).changed();
                self.action_text(ui, "Show all");
            });
        });

        changed
    }

    fn action_text(&self, ui: &mut egui::Ui, text: &str) {
        ui.label(
            RichText::new(text)
                .size(13.0)
                .color(Self::ACTION_TEXT)
                .strong(),
        );
    }

    fn desktop_button(&self, ui: &mut egui::Ui, kind: DesktopButtonKind, maximized: bool) {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(Self::WINDOWS_BUTTON_WIDTH, Self::BUTTON_HEIGHT),
            Sense::click(),
        );

        let fill = if response.hovered() {
            Color32::from_rgba_unmultiplied(0, 0, 0, 10)
        } else {
            Color32::TRANSPARENT
        };

        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
        self.paint_desktop_icon(ui, rect, kind, maximized, response.hovered());

        if response.clicked() {
            self.send_action(ui.ctx(), kind.into(), maximized);
        }
    }

    fn paint_desktop_icon(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        kind: DesktopButtonKind,
        maximized: bool,
        hovered: bool,
    ) {
        let tint = if hovered && kind == DesktopButtonKind::Close {
            Color32::WHITE
        } else {
            Color32::from_rgb(70, 77, 87)
        };
        let icon_rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(Self::DESKTOP_ICON_SIZE, Self::DESKTOP_ICON_SIZE),
        );

        ui.put(
            icon_rect,
            egui::Image::new(self.desktop_icon_source(kind, maximized))
                .fit_to_exact_size(icon_rect.size())
                .tint(tint),
        );
    }

    fn allocate_macos_button(&self, ui: &mut egui::Ui) -> MacButtonAllocation {
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(Self::MAC_BUTTON_SIZE, Self::MAC_BUTTON_SIZE),
            Sense::click(),
        );

        MacButtonAllocation { rect, response }
    }

    fn paint_macos_button(
        &self,
        ui: &mut egui::Ui,
        button: &MacButtonAllocation,
        kind: MacButtonKind,
        show_icon: bool,
    ) {
        let rect = button.rect;
        let center = rect.center();
        let radius = Self::MAC_BUTTON_SIZE * 0.5;
        let fill = match kind {
            MacButtonKind::Close => Color32::from_rgb(255, 95, 86),
            MacButtonKind::Minimize => Color32::from_rgb(255, 189, 46),
            MacButtonKind::Maximize => Color32::from_rgb(39, 201, 63),
        };

        ui.painter().circle_filled(center, radius, fill);
        ui.painter().circle_stroke(
            center,
            radius,
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 30)),
        );

        if show_icon {
            self.paint_macos_icon(ui, center, kind);
        }
    }

    fn paint_macos_icon(&self, ui: &mut egui::Ui, center: egui::Pos2, kind: MacButtonKind) {
        let icon_rect = egui::Rect::from_center_size(
            center,
            egui::vec2(Self::MAC_ICON_SIZE, Self::MAC_ICON_SIZE),
        );

        ui.put(
            icon_rect,
            egui::Image::new(self.macos_icon_source(kind))
                .fit_to_exact_size(icon_rect.size())
                .tint(Color32::from_rgba_unmultiplied(60, 60, 60, 220)),
        );
    }

    fn desktop_icon_source(
        &self,
        kind: DesktopButtonKind,
        maximized: bool,
    ) -> egui::ImageSource<'static> {
        match kind {
            DesktopButtonKind::Close => {
                egui::include_image!("../../assets/icons/titlebar/desktop-close.svg")
            }
            DesktopButtonKind::Minimize => {
                egui::include_image!("../../assets/icons/titlebar/desktop-minimize.svg")
            }
            DesktopButtonKind::Maximize if maximized => {
                egui::include_image!("../../assets/icons/titlebar/desktop-restore.svg")
            }
            DesktopButtonKind::Maximize => {
                egui::include_image!("../../assets/icons/titlebar/desktop-maximize.svg")
            }
        }
    }

    fn macos_icon_source(&self, kind: MacButtonKind) -> egui::ImageSource<'static> {
        match kind {
            MacButtonKind::Close => {
                egui::include_image!("../../assets/icons/titlebar/macos-close.svg")
            }
            MacButtonKind::Minimize => {
                egui::include_image!("../../assets/icons/titlebar/macos-minimize.svg")
            }
            MacButtonKind::Maximize => {
                egui::include_image!("../../assets/icons/titlebar/macos-maximize.svg")
            }
        }
    }

    fn send_action(&self, ctx: &egui::Context, action: WindowAction, maximized: bool) {
        match action {
            WindowAction::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
            WindowAction::Minimize => {
                if cfg!(target_os = "macos") {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                } else {
                    ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
                }
            }
            WindowAction::ToggleMaximize => {
                ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DesktopButtonKind {
    Minimize,
    Maximize,
    Close,
}

#[derive(Clone, Copy)]
enum MacButtonKind {
    Close,
    Minimize,
    Maximize,
}

#[derive(Clone, Copy)]
enum WindowAction {
    Close,
    Minimize,
    ToggleMaximize,
}

struct MacButtonAllocation {
    rect: egui::Rect,
    response: egui::Response,
}

impl From<DesktopButtonKind> for WindowAction {
    fn from(value: DesktopButtonKind) -> Self {
        match value {
            DesktopButtonKind::Minimize => Self::Minimize,
            DesktopButtonKind::Maximize => Self::ToggleMaximize,
            DesktopButtonKind::Close => Self::Close,
        }
    }
}

impl From<MacButtonKind> for WindowAction {
    fn from(value: MacButtonKind) -> Self {
        match value {
            MacButtonKind::Close => Self::Close,
            MacButtonKind::Minimize => Self::Minimize,
            MacButtonKind::Maximize => Self::ToggleMaximize,
        }
    }
}
