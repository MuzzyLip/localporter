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

#[derive(Default)]
pub struct MainScreenOutput {
    pub action: Option<MainScreenAction>,
    pub killable_pids: Vec<u32>,
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
        search_query: &str,
    ) -> MainScreenOutput {
        let Some(snapshot) = &state.snapshot else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("Waiting for first snapshot...");
            });
            return MainScreenOutput::default();
        };
        let uptime_offset = state.elapsed_since_collection();
        let filter = SearchFilter::parse(search_query);

        let mut kill_request = None;
        let visible_entries = snapshot
            .items
            .iter()
            .filter(|item| Self::should_show_process(item, state.show_all_enabled))
            .flat_map(|item| {
                Self::visible_row_keys(item, &filter)
                    .into_iter()
                    .map(move |row_key| (item, row_key))
            })
            .collect::<Vec<_>>();
        let visible_row_set = visible_entries
            .iter()
            .map(|(_, row_key)| *row_key)
            .collect::<HashSet<_>>();
        let mut killable_pids = visible_entries
            .iter()
            .filter_map(|(item, _)| state.is_process_killable(item).then_some(item.pid))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        killable_pids.sort_unstable();
        self.expanded_rows
            .retain(|row_key| visible_row_set.contains(row_key));

        if visible_entries.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label("No matching ports or processes");
            });
            return MainScreenOutput {
                action: None,
                killable_pids,
            };
        }

        eframe::egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            for (item, row_key) in visible_entries {
                let mut expanded = self.expanded_rows.contains(&row_key);

                if let Some(action) = self.process_panel.ui(
                    ui,
                    item,
                    row_key.port,
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
        });

        MainScreenOutput {
            action: kill_request.map(|action| match action {
                ProcessPanelAction::KillProcess(pid) => MainScreenAction::KillProcess(pid),
            }),
            killable_pids,
        }
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

    fn visible_row_keys(
        process: &localporter_core::ProcessSummary,
        filter: &SearchFilter,
    ) -> Vec<RowKey> {
        Self::row_keys_for_process(process)
            .into_iter()
            .filter(|row_key| filter.matches(process, row_key.port))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SearchFilter {
    None,
    ProcessName(String),
    PortContains(String),
    PortRange { start: u16, end: u16 },
}

impl SearchFilter {
    fn parse(query: &str) -> Self {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Self::None;
        }

        if trimmed.starts_with(':') {
            if let Some((start, end)) = Self::parse_port_range(trimmed) {
                return Self::PortRange { start, end };
            }

            let needle = trimmed
                .trim_start_matches(':')
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .collect::<String>();

            return if needle.is_empty() {
                Self::None
            } else {
                Self::PortContains(needle)
            };
        }

        Self::ProcessName(trimmed.to_ascii_lowercase())
    }

    fn matches(&self, process: &localporter_core::ProcessSummary, port: Option<BoundPort>) -> bool {
        match self {
            Self::None => true,
            Self::ProcessName(query) => process
                .name_or_unknown()
                .to_ascii_lowercase()
                .contains(query),
            Self::PortContains(query) => port
                .map(|bound_port| bound_port.port.to_string().contains(query))
                .unwrap_or(false),
            Self::PortRange { start, end } => port
                .map(|bound_port| (*start..=*end).contains(&bound_port.port))
                .unwrap_or(false),
        }
    }

    fn parse_port_range(query: &str) -> Option<(u16, u16)> {
        let normalized = query
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .collect::<String>();
        let (start, end) = normalized.split_once('-')?;
        let start = Self::parse_port_token(start)?;
        let end = Self::parse_port_token(end)?;

        (start <= end).then_some((start, end))
    }

    fn parse_port_token(token: &str) -> Option<u16> {
        token.trim_start_matches(':').parse::<u16>().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::SearchFilter;
    use localporter_core::{BoundPort, PortProtocol, ProcessSummary};
    use std::time::Duration;

    #[test]
    fn parses_port_range_with_colon_on_both_sides() {
        assert_eq!(
            SearchFilter::parse(":3000-:3999"),
            SearchFilter::PortRange {
                start: 3000,
                end: 3999,
            }
        );
    }

    #[test]
    fn parses_port_range_with_single_colon() {
        assert_eq!(
            SearchFilter::parse(":3000-3999"),
            SearchFilter::PortRange {
                start: 3000,
                end: 3999,
            }
        );
    }

    #[test]
    fn rejects_reversed_port_range() {
        assert_eq!(
            SearchFilter::parse(":2000-:20"),
            SearchFilter::PortContains("2000-:20".to_owned())
        );
    }

    #[test]
    fn port_contains_matches_port_digits() {
        let process = test_process("node");
        assert!(SearchFilter::parse(":30").matches(
            &process,
            Some(BoundPort {
                protocol: PortProtocol::Tcp,
                port: 3000,
            }),
        ));
        assert!(!SearchFilter::parse(":30").matches(
            &process,
            Some(BoundPort {
                protocol: PortProtocol::Tcp,
                port: 8080,
            }),
        ));
    }

    #[test]
    fn process_name_match_is_case_insensitive() {
        let process = test_process("Code Helper");
        assert!(SearchFilter::parse("code").matches(&process, None));
        assert!(!SearchFilter::parse("node").matches(&process, None));
    }

    fn test_process(name: &str) -> ProcessSummary {
        ProcessSummary {
            pid: 1,
            name: name.to_owned(),
            command: String::new(),
            ports: Vec::new(),
            launcher: String::new(),
            uptime: Duration::ZERO,
            cpu_percent: 0.0,
            memory_usage: 0,
        }
    }
}
