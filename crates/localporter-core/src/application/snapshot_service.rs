use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    thread,
    time::{Duration, SystemTime},
};

use crate::{
    domain::{
        BoundPort, CollectionWarning, PortQueryScope, ProcessOrigin, ProcessSnapshot,
        ProcessSummary,
    },
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

    pub fn collect_snapshot(&self, scope: PortQueryScope) -> ProcessSnapshot {
        let collected_at = SystemTime::now();
        let mut warnings = Vec::new();

        let bindings = match self.port_source.collect_bound_ports(scope) {
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
            if binding.pid == 0 {
                continue;
            }
            bindings_by_pid
                .entry(binding.pid)
                .or_default()
                .push(binding);
        }

        let pids: Vec<u32> = bindings_by_pid.keys().copied().collect();
        let (process_info, process_warnings, parent_origins, parent_warnings) =
            thread::scope(|scope| {
                let process_task =
                    scope.spawn(|| match self.process_source.collect_process_info(&pids) {
                        Ok(info) => (info, Vec::new()),
                        Err(error) => (
                            HashMap::new(),
                            vec![map_source_error("process_info", None, error)],
                        ),
                    });
                let parent_task = scope.spawn(|| {
                    let mut origins = HashMap::with_capacity(pids.len());
                    let mut warnings = Vec::new();

                    for &pid in &pids {
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
                        origins.insert(pid, origin);
                    }

                    (origins, warnings)
                });

                let (process_info, process_warnings) = process_task.join().unwrap();
                let (parent_origins, parent_warnings) = parent_task.join().unwrap();
                (
                    process_info,
                    process_warnings,
                    parent_origins,
                    parent_warnings,
                )
            });
        warnings.extend(process_warnings);
        warnings.extend(parent_warnings);

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

            let origin = parent_origins.get(&pid).cloned().unwrap_or_default();

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

#[cfg(test)]
mod tests {
    use std::{
        sync::Arc,
        thread,
        time::{Duration, Instant},
    };

    use super::*;
    use crate::{
        domain::{BoundPort, ParentProcess, PortProtocol, ProcessPortBinding},
        sources::{ParentChainSource, ProcessInfo, ProcessInfoSource},
    };

    struct StaticPortSource;

    impl BoundPortSource for StaticPortSource {
        fn collect_bound_ports(
            &self,
            _: PortQueryScope,
        ) -> Result<Vec<ProcessPortBinding>, SourceError> {
            Ok(vec![
                ProcessPortBinding {
                    pid: 0,
                    process_name: "Idle".to_owned(),
                    port: BoundPort {
                        protocol: PortProtocol::Tcp,
                        port: 1111,
                    },
                },
                ProcessPortBinding {
                    pid: 1234,
                    process_name: "app.exe".to_owned(),
                    port: BoundPort {
                        protocol: PortProtocol::Tcp,
                        port: 3000,
                    },
                },
            ])
        }
    }

    struct DelayedProcessSource {
        delay: Duration,
    }

    impl ProcessInfoSource for DelayedProcessSource {
        fn collect_process_info(
            &self,
            pids: &[u32],
        ) -> Result<HashMap<u32, ProcessInfo>, SourceError> {
            thread::sleep(self.delay);
            Ok(HashMap::from([(
                pids[0],
                ProcessInfo {
                    pid: pids[0],
                    ppid: Some(42),
                    name: "app.exe".to_owned(),
                    uptime: Some(Duration::from_secs(30)),
                    cpu_percent: Some(1.5),
                    memory_bytes: Some(2048),
                },
            )]))
        }
    }

    struct DelayedParentSource {
        delay: Duration,
    }

    impl ParentChainSource for DelayedParentSource {
        fn collect_parent_chain(
            &self,
            _: u32,
            _: usize,
        ) -> Result<Vec<ParentProcess>, SourceError> {
            thread::sleep(self.delay);
            Ok(vec![ParentProcess {
                pid: 42,
                name: "explorer.exe".to_owned(),
            }])
        }
    }

    #[test]
    fn collects_process_details_and_parent_chain_in_parallel() {
        let service = SnapshotService::new(
            Arc::new(StaticPortSource),
            Arc::new(DelayedProcessSource {
                delay: Duration::from_millis(250),
            }),
            Arc::new(DelayedParentSource {
                delay: Duration::from_millis(250),
            }),
        );

        let started_at = Instant::now();
        let snapshot = service.collect_snapshot(PortQueryScope::ListenOnly);
        let elapsed = started_at.elapsed();

        assert_eq!(snapshot.items.len(), 1);
        assert!(elapsed < Duration::from_millis(450), "elapsed: {elapsed:?}");
        assert_eq!(snapshot.items[0].launcher, "explorer.exe");
        assert_eq!(snapshot.items[0].ports[0].port, 3000);
    }
}
