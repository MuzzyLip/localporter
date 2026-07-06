use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime},
};

use eframe::egui;
use localporter_core::adapter::macos::command::{CommandRunner, StdCommandRunner};
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
const TOAST_DURATION: Duration = Duration::from_secs(4);
const MAX_TOASTS: usize = 3;

pub struct AppState {
    pub show_all_enabled: bool,
    pub snapshot: Option<ProcessSnapshot>,
    ctx: egui::Context,
    request_id: u64,
    next_toast_id: u64,
    toasts: Vec<ToastNotification>,
    kill_in_flight_pids: HashSet<u32>,
    kill_waiting_refresh_pids: HashSet<u32>,
    command_tx: mpsc::Sender<CollectionCommand>,
    update_tx: mpsc::Sender<AppUpdate>,
    update_rx: mpsc::Receiver<AppUpdate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionCommand {
    UpdateRequest(CollectionRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CollectionRequest {
    scope: PortQueryScope,
    request_id: u64,
}

#[derive(Debug)]
struct SnapshotUpdate {
    request_id: u64,
    snapshot: ProcessSnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastLevel {
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToastView {
    pub id: u64,
    pub level: ToastLevel,
    pub message: String,
}

#[derive(Debug)]
struct ToastNotification {
    id: u64,
    level: ToastLevel,
    message: String,
    expires_at: Instant,
}

#[derive(Debug)]
struct ToastUpdate {
    level: ToastLevel,
    message: String,
}

#[derive(Debug)]
struct KillUpdate {
    pid: u32,
    outcome: Result<(), String>,
}

#[derive(Debug)]
enum AppUpdate {
    Snapshot(SnapshotUpdate),
    Toast(ToastUpdate),
    KillFinished(KillUpdate),
}

impl AppState {
    pub fn new(ctx: egui::Context) -> Self {
        let initial_request = CollectionRequest {
            scope: PortQueryScope::ListenOnly,
            request_id: 0,
        };
        let (update_tx, update_rx) = mpsc::channel();
        let worker_update_tx = update_tx.clone();
        let (command_tx, command_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let worker_ctx = ctx.clone();

        thread::spawn(move || {
            let service = Arc::new(build_snapshot_service());
            let current_request_id = Arc::new(AtomicU64::new(initial_request.request_id));
            let mut request = initial_request;
            let mut active_request_ids = HashSet::new();

            spawn_collection(
                service.clone(),
                worker_update_tx.clone(),
                completed_tx.clone(),
                worker_ctx.clone(),
                current_request_id.clone(),
                request,
            );
            active_request_ids.insert(request.request_id);

            loop {
                while let Ok(completed_request_id) = completed_rx.try_recv() {
                    active_request_ids.remove(&completed_request_id);
                }
                match command_rx.recv_timeout(SNAPSHOT_POLL_INTERVAL) {
                    Ok(CollectionCommand::UpdateRequest(next_request)) => {
                        request = next_request;
                        current_request_id.store(request.request_id, Ordering::Relaxed);
                        if active_request_ids.insert(request.request_id) {
                            spawn_collection(
                                service.clone(),
                                worker_update_tx.clone(),
                                completed_tx.clone(),
                                worker_ctx.clone(),
                                current_request_id.clone(),
                                request,
                            );
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if active_request_ids.insert(request.request_id) {
                            spawn_collection(
                                service.clone(),
                                worker_update_tx.clone(),
                                completed_tx.clone(),
                                worker_ctx.clone(),
                                current_request_id.clone(),
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
            ctx,
            request_id: 0,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            command_tx,
            update_tx,
            update_rx,
        }
    }

    pub fn set_show_all_enabled(&mut self, enabled: bool) {
        if self.show_all_enabled == enabled {
            return;
        }

        self.show_all_enabled = enabled;
        self.request_collection();
    }

    fn request_collection(&mut self) {
        self.request_id = self.request_id.saturating_add(1);
        let request = self.current_request();
        let _ = self
            .command_tx
            .send(CollectionCommand::UpdateRequest(request));
    }

    pub fn drain_updates(&mut self) {
        let mut latest_snapshot = None;

        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                AppUpdate::Snapshot(snapshot) if snapshot.request_id == self.request_id => {
                    latest_snapshot = Some(snapshot.snapshot);
                }
                AppUpdate::Snapshot(_) => {}
                AppUpdate::Toast(toast) => self.push_toast(toast),
                AppUpdate::KillFinished(kill) => self.handle_kill_update(kill),
            }
        }

        if let Some(snapshot) = latest_snapshot {
            self.snapshot = Some(snapshot);
            self.kill_waiting_refresh_pids.clear();
        }

        self.retain_active_toasts();
    }

    pub fn kill_process(&mut self, pid: u32) {
        if self.is_kill_pending(pid) {
            return;
        }

        self.kill_in_flight_pids.insert(pid);
        self.request_id = self.request_id.saturating_add(1);
        let update_tx = self.update_tx.clone();
        let ctx = self.ctx.clone();
        self.ctx.request_repaint();

        thread::spawn(move || {
            let outcome = match kill_process_by_pid(pid) {
                Ok(()) => Ok(()),
                Err(error) => Err(format_source_error(&error)),
            };

            let _ = update_tx.send(AppUpdate::KillFinished(KillUpdate { pid, outcome }));
            ctx.request_repaint();
        });
    }

    pub fn is_kill_pending(&self, pid: u32) -> bool {
        self.kill_in_flight_pids.contains(&pid) || self.kill_waiting_refresh_pids.contains(&pid)
    }

    pub fn elapsed_since_collection(&self) -> Duration {
        self.snapshot
            .as_ref()
            .map(|snapshot| elapsed_since(snapshot.collected_at, SystemTime::now()))
            .unwrap_or_default()
    }

    pub fn toast_views(&mut self) -> Vec<ToastView> {
        self.retain_active_toasts();
        self.toasts
            .iter()
            .map(|toast| ToastView {
                id: toast.id,
                level: toast.level,
                message: toast.message.clone(),
            })
            .collect()
    }

    fn port_query_scope(&self) -> PortQueryScope {
        if self.show_all_enabled {
            PortQueryScope::AllTcp
        } else {
            PortQueryScope::ListenOnly
        }
    }

    fn current_request(&self) -> CollectionRequest {
        CollectionRequest {
            scope: self.port_query_scope(),
            request_id: self.request_id,
        }
    }

    fn push_toast(&mut self, toast: ToastUpdate) {
        self.toasts.push(ToastNotification {
            id: self.next_toast_id,
            level: toast.level,
            message: toast.message,
            expires_at: Instant::now() + TOAST_DURATION,
        });
        self.next_toast_id = self.next_toast_id.saturating_add(1);

        if self.toasts.len() > MAX_TOASTS {
            let overflow = self.toasts.len() - MAX_TOASTS;
            self.toasts.drain(0..overflow);
        }
    }

    fn retain_active_toasts(&mut self) {
        let now = Instant::now();
        self.toasts.retain(|toast| toast.expires_at > now);
    }

    fn handle_kill_update(&mut self, kill: KillUpdate) {
        self.kill_in_flight_pids.remove(&kill.pid);

        match kill.outcome {
            Ok(()) => {
                self.kill_waiting_refresh_pids.insert(kill.pid);
                let _ = self.update_tx.send(AppUpdate::Toast(ToastUpdate {
                    level: ToastLevel::Success,
                    message: format!("Killed PID {}", kill.pid),
                }));
                let _ = self
                    .command_tx
                    .send(CollectionCommand::UpdateRequest(self.current_request()));
            }
            Err(message) => {
                self.kill_waiting_refresh_pids.remove(&kill.pid);
                let _ = self.update_tx.send(AppUpdate::Toast(ToastUpdate {
                    level: ToastLevel::Error,
                    message: format!("Failed to kill PID {}: {message}", kill.pid),
                }));
            }
        }

        self.ctx.request_repaint();
    }
}

fn spawn_collection(
    service: Arc<SnapshotService>,
    update_tx: mpsc::Sender<AppUpdate>,
    completed_tx: mpsc::Sender<u64>,
    ctx: egui::Context,
    current_request_id: Arc<AtomicU64>,
    request: CollectionRequest,
) {
    thread::spawn(move || {
        let snapshot = service.collect_snapshot(request.scope);
        let is_current = current_request_id.load(Ordering::Relaxed) == request.request_id;

        if is_current
            && update_tx
                .send(AppUpdate::Snapshot(SnapshotUpdate {
                    request_id: request.request_id,
                    snapshot,
                }))
                .is_ok()
        {
            ctx.request_repaint();
        }

        let _ = completed_tx.send(request.request_id);
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

fn kill_process_by_pid(pid: u32) -> Result<(), localporter_core::SourceError> {
    let runner = StdCommandRunner;
    let pid_arg = pid.to_string();

    #[cfg(target_os = "windows")]
    {
        runner.run("taskkill", &["/PID", &pid_arg, "/T", "/F"])?;
    }

    #[cfg(target_os = "macos")]
    {
        runner.run("kill", &["-9", &pid_arg])?;
    }

    Ok(())
}

fn format_source_error(error: &localporter_core::SourceError) -> String {
    match error {
        localporter_core::SourceError::CommandNotFound { program } => {
            format!("{program} not found")
        }
        localporter_core::SourceError::CommandFailed { program, stderr } => {
            let stderr = stderr.trim();
            if stderr.is_empty() {
                format!("{program} failed")
            } else {
                stderr.to_owned()
            }
        }
        localporter_core::SourceError::PermissionDenied { program } => {
            format!("permission denied for {program}")
        }
        localporter_core::SourceError::InvalidOutput { source } => {
            format!("invalid output from {source}")
        }
    }
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
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            ctx: egui::Context::default(),
            request_id: 0,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            command_tx,
            update_tx,
            update_rx,
        };

        state.set_show_all_enabled(true);
        state.set_show_all_enabled(false);
        assert_eq!(state.request_id, 2);
        assert!(!state.show_all_enabled);
    }

    #[test]
    fn drain_updates_ignores_stale_snapshots() {
        let (update_tx, update_rx) = mpsc::channel();
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

        update_tx
            .send(AppUpdate::Snapshot(SnapshotUpdate {
                request_id: 0,
                snapshot: stale_snapshot,
            }))
            .unwrap();
        update_tx
            .send(AppUpdate::Snapshot(SnapshotUpdate {
                request_id: 1,
                snapshot: fresh_snapshot.clone(),
            }))
            .unwrap();

        let mut state = AppState {
            show_all_enabled: true,
            snapshot: None,
            ctx: egui::Context::default(),
            request_id: 1,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            command_tx,
            update_tx,
            update_rx,
        };
        state.drain_updates();

        assert_eq!(state.snapshot, Some(fresh_snapshot));
    }

    #[test]
    fn spawn_collection_skips_stale_snapshots() {
        let (update_tx, update_rx) = mpsc::channel();
        let (completed_tx, completed_rx) = mpsc::channel();
        let current_version = Arc::new(AtomicU64::new(1));
        let service = Arc::new(build_test_snapshot_service());
        let ctx = egui::Context::default();

        spawn_collection(
            service,
            update_tx,
            completed_tx,
            ctx,
            current_version,
            CollectionRequest {
                scope: PortQueryScope::ListenOnly,
                request_id: 0,
            },
        );

        assert_eq!(completed_rx.recv().unwrap(), 0);
        assert!(update_rx.try_recv().is_err());
    }

    #[test]
    fn toast_views_drop_expired_items() {
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            ctx: egui::Context::default(),
            request_id: 0,
            next_toast_id: 1,
            toasts: vec![ToastNotification {
                id: 0,
                level: ToastLevel::Success,
                message: "stale".to_owned(),
                expires_at: Instant::now() - Duration::from_secs(1),
            }],
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            command_tx,
            update_tx,
            update_rx,
        };

        assert!(state.toast_views().is_empty());
    }

    #[test]
    fn fresh_snapshot_clears_pending_kill_guard() {
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        update_tx
            .send(AppUpdate::Snapshot(SnapshotUpdate {
                request_id: 1,
                snapshot: ProcessSnapshot {
                    collected_at: SystemTime::UNIX_EPOCH + Duration::from_secs(1),
                    items: Vec::new(),
                    warnings: Vec::new(),
                },
            }))
            .unwrap();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            ctx: egui::Context::default(),
            request_id: 1,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::from([42]),
            command_tx,
            update_tx,
            update_rx,
        };

        state.drain_updates();

        assert!(!state.is_kill_pending(42));
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
