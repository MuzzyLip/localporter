use std::time::Duration;

use eframe::egui::{self, Color32, CornerRadius, Sense, Stroke, StrokeKind};
use localporter_core::{BoundPort, PortProtocol, ProcessSummary};

use crate::components::{CollapsiblePanel, PortRow, PortRowDetails, PortRowDetailsAction};

pub enum ProcessPanelAction {
    OpenInBrowser(u16),
    KillProcess(u32),
}

#[derive(Default)]
pub struct ProcessPanel {
    panel: CollapsiblePanel,
    port_row: PortRow,
    details: PortRowDetails,
}

impl ProcessPanel {
    const HEADER_ACTION_ICON_SIZE: f32 = 14.0;

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        process: &ProcessSummary,
        port: Option<BoundPort>,
        uptime_offset: Duration,
        kill_pending: bool,
        expanded: &mut bool,
    ) -> Option<ProcessPanelAction> {
        let panel = &mut self.panel;
        let port_row = &mut self.port_row;
        let details = &mut self.details;

        panel.show(
            ui,
            expanded,
            |ui| port_row.ui(ui, process, port, uptime_offset),
            |ui, visible| {
                if !visible {
                    return None;
                }

                let Some(port) = Self::browser_open_port(port) else {
                    return None;
                };

                Self::open_in_browser_button(ui, port, visible)
                    .clicked()
                    .then_some(ProcessPanelAction::OpenInBrowser(port))
            },
            |ui| {
                details
                    .ui(ui, process, kill_pending)
                    .map(|action| match action {
                        PortRowDetailsAction::KillProcess(pid) => {
                            ProcessPanelAction::KillProcess(pid)
                        }
                    })
            },
        )
    }

    fn open_in_browser_button(ui: &mut egui::Ui, port: u16, visible: bool) -> egui::Response {
        let sense = if visible {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), sense);
        let fill = if visible && response.hovered() {
            Color32::from_rgb(243, 244, 246)
        } else if visible {
            Color32::from_rgb(247, 247, 247)
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if visible {
            Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 13))
        } else {
            Stroke::NONE
        };
        ui.painter().rect(
            rect,
            CornerRadius::same(8),
            fill,
            stroke,
            StrokeKind::Inside,
        );

        let icon_rect = egui::Rect::from_center_size(
            rect.center(),
            egui::vec2(Self::HEADER_ACTION_ICON_SIZE, Self::HEADER_ACTION_ICON_SIZE),
        );
        egui::Image::new(Self::open_in_browser_icon_source())
            .fit_to_exact_size(icon_rect.size())
            .tint(if visible {
                Color32::from_rgb(93, 104, 119)
            } else {
                Color32::TRANSPARENT
            })
            .paint_at(ui, icon_rect);

        let response = if visible {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        };

        if visible {
            response.on_hover_text(format!("Open in :{}", port))
        } else {
            response
        }
    }

    fn browser_open_port(port: Option<BoundPort>) -> Option<u16> {
        match port {
            Some(BoundPort {
                protocol: PortProtocol::Tcp,
                port,
            }) => Some(port),
            _ => None,
        }
    }

    fn open_in_browser_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/port-row/launch-in-browser.svg")
    }
}
