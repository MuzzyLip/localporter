use std::{collections::HashMap, sync::Arc};

use crate::{
    adapter::macos::command::{parser::parse_ps_process_info, runner::CommandRunner},
    error::SourceError,
    sources::{ProcessInfo, ProcessInfoSource},
};

pub struct PsProcessInfoSource {
    runner: Arc<dyn CommandRunner>,
}

impl PsProcessInfoSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl ProcessInfoSource for PsProcessInfoSource {
    fn collect_process_info(&self, pids: &[u32]) -> Result<HashMap<u32, ProcessInfo>, SourceError> {
        if pids.is_empty() {
            return Ok(HashMap::new());
        }

        let pid_list = pids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");

        let raw = self.runner.run(
            "ps",
            &[
                "-p", &pid_list, "-o", "pid=", "-o", "ppid=", "-o", "%cpu=", "-o", "rss=", "-o",
                "etime=", "-o", "command=", "-ww",
            ],
        )?;

        let items = parse_ps_process_info(&raw)?;
        Ok(items.into_iter().map(|item| (item.pid, item)).collect())
    }
}
