use std::{collections::HashMap, time::Duration};

use crate::SourceError;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub name: String,
    pub uptime: Option<Duration>,
    pub cpu_percent: Option<f32>,
    pub memory_bytes: Option<u64>,
}

pub trait ProcessInfoSource: Send + Sync {
    fn collect_process_info(&self, pids: &[u32]) -> Result<HashMap<u32, ProcessInfo>, SourceError>;
}
