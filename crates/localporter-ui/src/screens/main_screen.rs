use std::collections::HashSet;

use crate::{
    components::{ProcessPanel, ProcessPanelAction},
    state::AppState,
};
use localporter_core::BoundPort;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RowKey {
    pid: u32,
    port: Option<BoundPort>,
}

pub struct MainScreen {
    process_panel: ProcessPanel,
    expanded_rows: HashSet<RowKey>,
}

pub enum MainScreenAction {
    KillProcess(u32),
}

impl Default for MainScreen {
    fn default() -> Self {
        Self {
            process_panel: ProcessPanel::default(),
            expanded_rows: HashSet::new(),
        }
    }
}

impl MainScreen {
    pub fn ui(
        &mut self,
        ui: &mut eframe::egui::Ui,
        state: &mut AppState,
    ) -> Option<MainScreenAction> {
        let Some(snapshot) = &state.snapshot else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("Waiting for first snapshot...");
            });
            return None;
        };
        let uptime_offset = state.elapsed_since_collection();

        let mut kill_request = None;
        let visible_items = snapshot
            .items
            .iter()
            .filter(|item| Self::should_show_process(item, state.show_all_enabled))
            .collect::<Vec<_>>();
        let visible_rows = visible_items
            .iter()
            .copied()
            .flat_map(Self::row_keys_for_process)
            .collect::<HashSet<_>>();
        self.expanded_rows
            .retain(|row_key| visible_rows.contains(row_key));

        eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            for item in visible_items {
                if item.ports.is_empty() {
                    let row_key = RowKey {
                        pid: item.pid,
                        port: None,
                    };
                    let mut expanded = self.expanded_rows.contains(&row_key);

                    if let Some(action) = self.process_panel.ui(
                        ui,
                        item,
                        None,
                        uptime_offset,
                        state.is_kill_pending(item.pid),
                        &mut expanded,
                    ) {
                        kill_request = Some(action);
                    }

                    if expanded {
                        self.expanded_rows.insert(row_key);
                    } else {
                        self.expanded_rows.remove(&row_key);
                    }
                    continue;
                }

                for &port in &item.ports {
                    let row_key = RowKey {
                        pid: item.pid,
                        port: Some(port),
                    };
                    let mut expanded = self.expanded_rows.contains(&row_key);

                    if let Some(action) = self.process_panel.ui(
                        ui,
                        item,
                        Some(port),
                        uptime_offset,
                        state.is_kill_pending(item.pid),
                        &mut expanded,
                    ) {
                        kill_request = Some(action);
                    }

                    if expanded {
                        self.expanded_rows.insert(row_key);
                    } else {
                        self.expanded_rows.remove(&row_key);
                    }
                }
            }
        });

        kill_request.map(|action| match action {
            ProcessPanelAction::KillProcess(pid) => MainScreenAction::KillProcess(pid),
        })
    }

    fn should_show_process(
        process: &localporter_core::ProcessSummary,
        show_all_enabled: bool,
    ) -> bool {
        #[cfg(target_os = "windows")]
        if !show_all_enabled && process.pid == 4 {
            return false;
        }

        let _ = show_all_enabled;
        true
    }

    fn row_keys_for_process(process: &localporter_core::ProcessSummary) -> Vec<RowKey> {
        if process.ports.is_empty() {
            return vec![RowKey {
                pid: process.pid,
                port: None,
            }];
        }

        process
            .ports
            .iter()
            .copied()
            .map(|port| RowKey {
                pid: process.pid,
                port: Some(port),
            })
            .collect()
    }
}
