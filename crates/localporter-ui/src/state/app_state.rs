use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;
use localporter_core::adapter::macos::command::StdCommandRunner;
#[cfg(target_os = "macos")]
use localporter_core::adapter::macos::command::{
    LsofPortSource, PsParentChainSource, PsProcessInfoSource,
};
#[cfg(target_os = "windows")]
use localporter_core::adapter::windows::command::{
    CimParentChainSource, CimProcessInfoSource, NetConnectionPortSource,
};
use localporter_core::{PortQueryScope, ProcessSnapshot, SnapshotService};

const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct AppState {
    pub show_all_enabled: bool,
    pub snapshot: Option<ProcessSnapshot>,
    scope_version: u64,
    command_tx: mpsc::Sender<CollectionCommand>,
    snapshot_rx: mpsc::Receiver<SnapshotUpdate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionCommand {
    UpdateRequest(CollectionRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CollectionRequest {
    scope: PortQueryScope,
    version: u64,
}

#[derive(Debug)]
struct SnapshotUpdate {
    version: u64,
    snapshot: ProcessSnapshot,
}

impl AppState {
    pub fn new(ctx: egui::Context) -> Self {
        let initial_request = CollectionRequest {
            scope: PortQueryScope::ListenOnly,
            version: 0,
        };
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();

        thread::spawn(move || {
            let service = Arc::new(build_snapshot_service());
            let current_version = Arc::new(AtomicU64::new(initial_request.version));
            let mut request = initial_request;
            let mut active_versions = HashSet::new();

            spawn_collection(
                service.clone(),
                snapshot_tx.clone(),
                completed_tx.clone(),
                ctx.clone(),
                current_version.clone(),
                request,
            );
            active_versions.insert(request.version);

            loop {
                while let Ok(completed_version) = completed_rx.try_recv() {
                    active_versions.remove(&completed_version);
                }
                match command_rx.recv_timeout(SNAPSHOT_POLL_INTERVAL) {
                    Ok(CollectionCommand::UpdateRequest(next_request)) => {
                        request = next_request;
                        current_version.store(request.version, Ordering::Relaxed);
                        if active_versions.insert(request.version) {
                            spawn_collection(
                                service.clone(),
                                snapshot_tx.clone(),
                                completed_tx.clone(),
                                ctx.clone(),
                                current_version.clone(),
                                request,
                            );
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if active_versions.insert(request.version) {
                            spawn_collection(
                                service.clone(),
                                snapshot_tx.clone(),
                                completed_tx.clone(),
                                ctx.clone(),
                                current_version.clone(),
                                request,
                            );
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Self {
            show_all_enabled: false,
            snapshot: None,
            scope_version: 0,
            command_tx,
            snapshot_rx,
        }
    }

    pub fn set_show_all_enabled(&mut self, enabled: bool) {
        if self.show_all_enabled == enabled {
            return;
        }

        self.show_all_enabled = enabled;
        self.scope_version = self.scope_version.saturating_add(1);
        self.sync_collection_scope();
    }

    fn sync_collection_scope(&mut self) {
        let request = CollectionRequest {
            scope: self.port_query_scope(),
            version: self.scope_version,
        };
        let _ = self
            .command_tx
            .send(CollectionCommand::UpdateRequest(request));
    }

    pub fn drain_updates(&mut self) {
        let mut latest_snapshot = None;

        while let Ok(update) = self.snapshot_rx.try_recv() {
            if update.version == self.scope_version {
                latest_snapshot = Some(update.snapshot);
            }
        }

        if let Some(snapshot) = latest_snapshot {
            self.snapshot = Some(snapshot);
        }
    }

    pub fn elapsed_since_collection(&self) -> Duration {
        self.snapshot
            .as_ref()
            .map(|snapshot| elapsed_since(snapshot.collected_at, SystemTime::now()))
            .unwrap_or_default()
    }

    fn port_query_scope(&self) -> PortQueryScope {
        if self.show_all_enabled {
            PortQueryScope::AllTcp
        } else {
            PortQueryScope::ListenOnly
        }
    }
}

fn spawn_collection(
    service: Arc<SnapshotService>,
    snapshot_tx: mpsc::Sender<SnapshotUpdate>,
    completed_tx: mpsc::Sender<u64>,
    ctx: egui::Context,
    current_version: Arc<AtomicU64>,
    request: CollectionRequest,
) {
    thread::spawn(move || {
        let snapshot = service.collect_snapshot(request.scope);
        let is_current = current_version.load(Ordering::Relaxed) == request.version;

        if is_current
            && snapshot_tx
                .send(SnapshotUpdate {
                    version: request.version,
                    snapshot,
                })
                .is_ok()
        {
            ctx.request_repaint();
        }

        let _ = completed_tx.send(request.version);
    });
}

#[cfg(target_os = "windows")]
fn build_snapshot_service() -> SnapshotService {
    let runner = Arc::new(StdCommandRunner);
    SnapshotService::new(
        Arc::new(NetConnectionPortSource::new(runner.clone())),
        Arc::new(CimProcessInfoSource::new(runner.clone())),
        Arc::new(CimParentChainSource::new(runner)),
    )
}

#[cfg(target_os = "macos")]
fn build_snapshot_service() -> SnapshotService {
    let runner = Arc::new(StdCommandRunner);
    SnapshotService::new(
        Arc::new(LsofPortSource::new(runner.clone())),
        Arc::new(PsProcessInfoSource::new(runner.clone())),
        Arc::new(PsParentChainSource::new(runner)),
    )
}

fn elapsed_since(collected_at: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(collected_at).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elapsed_since_returns_difference_when_now_is_later() {
        let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(107);

        assert_eq!(elapsed_since(collected_at, now), Duration::from_secs(7));
    }

    #[test]
    fn elapsed_since_returns_zero_when_clock_moves_backwards() {
        let collected_at = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(95);

        assert_eq!(elapsed_since(collected_at, now), Duration::ZERO);
    }

    #[test]
    fn latest_request_prefers_most_recent_scope_change() {
        let (_snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            scope_version: 0,
            command_tx,
            snapshot_rx,
        };

        state.set_show_all_enabled(true);
        state.set_show_all_enabled(false);
        assert_eq!(state.scope_version, 2);
        assert!(!state.show_all_enabled);
    }

    #[test]
    fn drain_updates_ignores_stale_snapshots() {
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let stale_snapshot = ProcessSnapshot {
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            items: Vec::new(),
            warnings: Vec::new(),
        };
        let fresh_snapshot = ProcessSnapshot {
            collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            items: Vec::new(),
            warnings: Vec::new(),
        };

        snapshot_tx
            .send(SnapshotUpdate {
                version: 0,
                snapshot: stale_snapshot,
            })
            .unwrap();
        snapshot_tx
            .send(SnapshotUpdate {
                version: 1,
                snapshot: fresh_snapshot.clone(),
            })
            .unwrap();

        let mut state = AppState {
            show_all_enabled: true,
            snapshot: None,
            scope_version: 1,
            command_tx,
            snapshot_rx,
        };
        state.drain_updates();

        assert_eq!(state.snapshot, Some(fresh_snapshot));
    }

    #[test]
    fn spawn_collection_skips_stale_snapshots() {
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let current_version = Arc::new(AtomicU64::new(1));
        let service = Arc::new(build_test_snapshot_service());
        let ctx = egui::Context::default();

        spawn_collection(
            service,
            snapshot_tx,
            completed_tx,
            ctx,
            current_version,
            CollectionRequest {
                scope: PortQueryScope::ListenOnly,
                version: 0,
            },
        );

        assert_eq!(completed_rx.recv().unwrap(), 0);
        assert!(snapshot_rx.try_recv().is_err());
    }

    #[cfg(test)]
    fn build_test_snapshot_service() -> SnapshotService {
        SnapshotService::new(
            Arc::new(StaticPortSource),
            Arc::new(StaticProcessSource),
            Arc::new(StaticParentSource),
        )
    }

    struct StaticPortSource;

    impl localporter_core::sources::BoundPortSource for StaticPortSource {
        fn collect_bound_ports(
            &self,
            _: PortQueryScope,
        ) -> Result<Vec<localporter_core::ProcessPortBinding>, localporter_core::SourceError>
        {
            Ok(Vec::new())
        }
    }

    struct StaticProcessSource;

    impl localporter_core::sources::ProcessInfoSource for StaticProcessSource {
        fn collect_process_info(
            &self,
            _: &[u32],
        ) -> Result<
            std::collections::HashMap<u32, localporter_core::sources::ProcessInfo>,
            localporter_core::SourceError,
        > {
            Ok(std::collections::HashMap::new())
        }
    }

    struct StaticParentSource;

    impl localporter_core::sources::ParentChainSource for StaticParentSource {
        fn collect_parent_chain(
            &self,
            _: u32,
            _: usize,
        ) -> Result<Vec<localporter_core::ParentProcess>, localporter_core::SourceError> {
            Ok(Vec::new())
        }
    }
}
