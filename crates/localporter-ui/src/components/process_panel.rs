use std::time::Duration;

use eframe::egui;
use localporter_core::{BoundPort, ProcessSummary};

use crate::components::{CollapsiblePanel, PortRow, PortRowDetails, PortRowDetailsAction};

pub enum ProcessPanelAction {
    KillProcess(u32),
}

#[derive(Default)]
pub struct ProcessPanel {
    panel: CollapsiblePanel,
    port_row: PortRow,
    details: PortRowDetails,
}

impl ProcessPanel {
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
}
