use std::time::SystemTime;

use crate::domain::ProcessSummary;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSnapshot {
    pub collected_at: SystemTime,
    pub items: Vec<ProcessSummary>,
    pub warnings: Vec<CollectionWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectionWarning {
    ProcessVanished {
        pid: u32,
    },
    PermissionDenied {
        target: &'static str,
        pid: Option<u32>,
    },
    SourceUnavailable {
        source: &'static str,
    },
    MalformedOutput {
        source: &'static str,
    },
}
