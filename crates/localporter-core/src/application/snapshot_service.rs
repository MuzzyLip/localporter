use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{
    domain::{BoundPort, CollectionWarning, ProcessOrigin, ProcessSnapshot, ProcessSummary},
    error::SourceError,
    sources::{BoundPortSource, ParentChainSource, ProcessInfoSource},
};

pub struct SnapshotService {
    port_source: Arc<dyn BoundPortSource>,
    process_source: Arc<dyn ProcessInfoSource>,
    parent_chain_source: Arc<dyn ParentChainSource>,
    max_parent_depth: usize,
}

impl SnapshotService {
    pub fn new(
        port_source: Arc<dyn BoundPortSource>,
        process_source: Arc<dyn ProcessInfoSource>,
        parent_chain_source: Arc<dyn ParentChainSource>,
    ) -> Self {
        Self {
            port_source,
            process_source,
            parent_chain_source,
            max_parent_depth: 8,
        }
    }

    pub fn collect_snapshot(&self) -> ProcessSnapshot {
        let collected_at = SystemTime::now();
        let mut warnings = Vec::new();

        let bindings = match self.port_source.collect_bound_ports() {
            Ok(bindings) => bindings,
            Err(error) => {
                warnings.push(map_source_error("bound_ports", None, error));
                return ProcessSnapshot {
                    collected_at,
                    items: Vec::new(),
                    warnings,
                };
            }
        };

        let mut bindings_by_pid: BTreeMap<u32, Vec<crate::domain::ProcessPortBinding>> =
            BTreeMap::new();
        for binding in bindings {
            bindings_by_pid
                .entry(binding.pid)
                .or_default()
                .push(binding);
        }

        let pids: Vec<u32> = bindings_by_pid.keys().copied().collect();
        let process_info = match self.process_source.collect_process_info(&pids) {
            Ok(info) => info,
            Err(error) => {
                warnings.push(map_source_error("process_info", None, error));
                HashMap::new()
            }
        };

        let mut items = Vec::with_capacity(bindings_by_pid.len());

        for (pid, bindings) in bindings_by_pid {
            let ports = dedup_ports(bindings.iter().map(|binding| binding.port).collect());
            let binding_name = bindings
                .iter()
                .find_map(|binding| {
                    (!binding.process_name.is_empty()).then_some(binding.process_name.as_str())
                })
                .unwrap_or("Unknown");

            let info = process_info.get(&pid);
            if info.is_none() {
                warnings.push(CollectionWarning::ProcessVanished { pid });
            }

            let origin = match self
                .parent_chain_source
                .collect_parent_chain(pid, self.max_parent_depth)
            {
                Ok(parent_chain) => build_origin(parent_chain),
                Err(error) => {
                    warnings.push(map_source_error("parent_chain", Some(pid), error));
                    ProcessOrigin::default()
                }
            };

            let name = info
                .map(|info| info.name.as_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(binding_name)
                .to_owned();

            items.push(ProcessSummary {
                name,
                ports,
                launcher: origin.resolved_name_or_unknown().to_owned(),
                uptime: info.and_then(|info| info.uptime).unwrap_or(Duration::ZERO),
                cpu_percent: info.and_then(|info| info.cpu_percent).unwrap_or(0.0),
                memory_usage: info.and_then(|info| info.memory_bytes).unwrap_or(0),
            });
        }

        ProcessSnapshot {
            collected_at,
            items,
            warnings,
        }
    }
}

fn dedup_ports(mut ports: Vec<BoundPort>) -> Vec<BoundPort> {
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn build_origin(parent_chain: Vec<crate::domain::ParentProcess>) -> ProcessOrigin {
    let immediate_parent = parent_chain.first().cloned();
    let resolved_name = immediate_parent
        .as_ref()
        .map(|parent| parent.name.clone())
        .unwrap_or_default();

    ProcessOrigin {
        resolved_name,
        immediate_parent,
        parent_chain,
    }
}

fn map_source_error(
    source: &'static str,
    pid: Option<u32>,
    error: SourceError,
) -> CollectionWarning {
    match error {
        SourceError::PermissionDenied { .. } => CollectionWarning::PermissionDenied {
            target: source,
            pid,
        },
        SourceError::InvalidOutput { .. } => CollectionWarning::MalformedOutput { source },
        SourceError::CommandNotFound { .. } | SourceError::CommandFailed { .. } => {
            CollectionWarning::SourceUnavailable { source }
        }
    }
}
