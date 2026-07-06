use std::{collections::HashMap, sync::Arc};

use crate::{
    adapter::{macos::command::CommandRunner, windows::command::parser::parse_cim_process_info},
    error::SourceError,
    sources::{ProcessInfo, ProcessInfoSource},
};

pub struct CimProcessInfoSource {
    runner: Arc<dyn CommandRunner>,
}

impl CimProcessInfoSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl ProcessInfoSource for CimProcessInfoSource {
    fn collect_process_info(&self, pids: &[u32]) -> Result<HashMap<u32, ProcessInfo>, SourceError> {
        if pids.is_empty() {
            return Ok(HashMap::new());
        }

        let process_filter = build_or_filter("ProcessId", pids);
        let perf_filter = build_or_filter("IDProcess", pids);
        let script = format!(
            concat!(
                "$cpuByPid = @{{}}; ",
                "Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -Filter \"{perf_filter}\" | ",
                "ForEach-Object {{ $cpuByPid[[uint32]$_.IDProcess] = [single]$_.PercentProcessorTime }}; ",
                "Get-CimInstance Win32_Process -Filter \"{process_filter}\" | ",
                "ForEach-Object {{ ",
                "$uptime = [math]::Max([int64]((New-TimeSpan -Start $_.CreationDate -End (Get-Date)).TotalSeconds), 0); ",
                "$cpu = if ($cpuByPid.ContainsKey([uint32]$_.ProcessId)) {{ $cpuByPid[[uint32]$_.ProcessId] }} else {{ 0 }}; ",
                "\"$($_.ProcessId)|$($_.ParentProcessId)|$($_.Name)|$uptime|$($_.WorkingSetSize)|$cpu\" ",
                "}}"
            ),
            perf_filter = perf_filter,
            process_filter = process_filter,
        );
        let args = ["-NoProfile", "-NonInteractive", "-Command", script.as_str()];
        let raw = self.runner.run("powershell", &args)?;
        let items = parse_cim_process_info(&raw)?;

        Ok(items.into_iter().map(|item| (item.pid, item)).collect())
    }
}

fn build_or_filter(field_name: &str, values: &[u32]) -> String {
    values
        .iter()
        .map(|value| format!("{field_name} = {value}"))
        .collect::<Vec<_>>()
        .join(" OR ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_or_filter_for_cim_queries() {
        assert_eq!(
            build_or_filter("ProcessId", &[12, 34, 56]),
            "ProcessId = 12 OR ProcessId = 34 OR ProcessId = 56"
        );
    }
}
