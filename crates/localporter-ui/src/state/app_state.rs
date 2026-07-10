#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::{
    collections::HashSet,
    io,
    process::Command,
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
use localporter_core::{PortQueryScope, ProcessSnapshot, ProcessSummary};
use localporter_core::{log_debug, log_error, log_info, log_warn};

const TOAST_DURATION: Duration = Duration::from_secs(4);
const MAX_TOASTS: usize = 3;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use super::collection_controller::{CollectionCommand, CollectionRequest, spawn_collection_worker};
use super::settings::{
    AppSettings, KillBehavior, launch_at_startup_supported, read_launch_at_startup,
    write_launch_at_startup,
};

pub struct AppState {
    pub show_all_enabled: bool,
    pub snapshot: Option<ProcessSnapshot>,
    settings: AppSettings,
    ctx: egui::Context,
    request_id: u64,
    next_toast_id: u64,
    toasts: Vec<ToastNotification>,
    kill_in_flight_pids: HashSet<u32>,
    kill_waiting_refresh_pids: HashSet<u32>,
    poll_interval_ms: Arc<AtomicU64>,
    command_tx: mpsc::Sender<CollectionCommand>,
    update_tx: mpsc::Sender<AppUpdate>,
    update_rx: mpsc::Receiver<AppUpdate>,
}

#[derive(Debug)]
pub(super) struct SnapshotUpdate {
    pub(super) request_id: u64,
    pub(super) snapshot: ProcessSnapshot,
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
pub(super) struct KillUpdate {
    pid: u32,
    outcome: Result<(), String>,
}

#[derive(Debug)]
pub(super) struct KillAllUpdate {
    successes: Vec<u32>,
    failures: Vec<KillFailure>,
}

#[derive(Debug)]
pub(super) struct KillFailure {
    pid: u32,
    message: String,
}

#[derive(Debug)]
pub(super) enum AppUpdate {
    Snapshot(SnapshotUpdate),
    KillFinished(KillUpdate),
    KillAllFinished(KillAllUpdate),
}

impl AppState {
    pub fn new(ctx: egui::Context) -> Self {
        let mut settings = AppSettings::load();
        if launch_at_startup_supported()
            && let Ok(enabled) = read_launch_at_startup()
        {
            settings.launch_at_startup = enabled;
        }

        log_info!(
            "app state initializing: refresh_interval={}s launch_at_startup={} kill_behavior={}",
            settings.refresh_interval.seconds(),
            settings.launch_at_startup,
            settings.kill_behavior
        );

        let initial_request = CollectionRequest {
            scope: PortQueryScope::ListenOnly,
            request_id: 0,
        };
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let poll_interval_ms = Arc::new(AtomicU64::new(
            settings.refresh_interval.duration().as_millis() as u64,
        ));
        spawn_collection_worker(
            command_rx,
            update_tx.clone(),
            ctx.clone(),
            poll_interval_ms.clone(),
            initial_request,
        );

        Self {
            show_all_enabled: false,
            snapshot: None,
            settings,
            ctx,
            request_id: 0,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            poll_interval_ms,
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
        log_info!(
            "port scope changed: show_all_enabled={} scope={:?}",
            self.show_all_enabled,
            self.port_query_scope()
        );
        self.request_collection();
    }

    fn request_collection(&mut self) {
        self.request_id = self.request_id.saturating_add(1);
        let request = self.current_request();
        log_debug!(
            "queue collection request: request_id={} scope={:?}",
            request.request_id,
            request.scope
        );
        let _ = self
            .command_tx
            .send(CollectionCommand::UpdateRequest(request));
    }

    pub fn drain_updates(&mut self) {
        let mut latest_snapshot = None;

        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                AppUpdate::Snapshot(snapshot) if snapshot.request_id == self.request_id => {
                    log_debug!(
                        "received current snapshot update: request_id={} items={} warnings={}",
                        snapshot.request_id,
                        snapshot.snapshot.items.len(),
                        snapshot.snapshot.warnings.len()
                    );
                    latest_snapshot = Some(snapshot.snapshot);
                }
                AppUpdate::Snapshot(snapshot) => {
                    log_debug!(
                        "ignored stale snapshot update: request_id={} current_request_id={}",
                        snapshot.request_id,
                        self.request_id
                    );
                }
                AppUpdate::KillFinished(kill) => self.handle_kill_update(kill),
                AppUpdate::KillAllFinished(kill_all) => self.handle_kill_all_update(kill_all),
            }
        }

        if let Some(snapshot) = latest_snapshot {
            log_debug!(
                "applied snapshot: items={} warnings={}",
                snapshot.items.len(),
                snapshot.warnings.len()
            );
            self.snapshot = Some(snapshot);
            self.kill_waiting_refresh_pids.clear();
        }

        self.retain_active_toasts();
    }

    pub fn kill_process(&mut self, pid: u32) {
        if self.is_kill_pending(pid) {
            log_debug!("skip kill request for pending pid={pid}");
            return;
        }

        self.kill_in_flight_pids.insert(pid);
        self.request_id = self.request_id.saturating_add(1);
        log_info!("kill requested: pid={pid}");
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

    pub fn open_in_browser(&mut self, port: u16) {
        let url = browser_url(port);

        match open_url_in_browser(&url) {
            Ok(()) => {
                log_info!("opened port in browser: port={port} url={url}");
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Success,
                    message: format!("Opened {url}"),
                });
            }
            Err(error) => {
                let message = format_browser_open_error(&error);
                log_warn!("failed to open port in browser: port={port} error={message}");
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Error,
                    message: format!("Failed to open {url}: {message}"),
                });
            }
        }

        self.ctx.request_repaint();
    }

    #[allow(dead_code)]
    pub fn kill_all_killable(&mut self) {
        self.kill_processes(self.killable_pids());
    }

    pub fn kill_processes(&mut self, pids: Vec<u32>) {
        let pids = self.normalize_kill_pids(pids);
        if pids.is_empty() {
            log_debug!("skip kill-all request because no killable pids remain");
            return;
        }

        log_info!("kill-all requested: pids={:?}", pids);
        for pid in &pids {
            self.kill_in_flight_pids.insert(*pid);
        }

        self.request_id = self.request_id.saturating_add(1);
        let update_tx = self.update_tx.clone();
        let ctx = self.ctx.clone();
        self.ctx.request_repaint();

        thread::spawn(move || {
            let mut successes = Vec::new();
            let mut failures = Vec::new();

            for pid in pids {
                match kill_process_by_pid(pid) {
                    Ok(()) => successes.push(pid),
                    Err(error) => failures.push(KillFailure {
                        pid,
                        message: format_source_error(&error),
                    }),
                }
            }

            let _ = update_tx.send(AppUpdate::KillAllFinished(KillAllUpdate {
                successes,
                failures,
            }));
            ctx.request_repaint();
        });
    }

    pub fn is_kill_pending(&self, pid: u32) -> bool {
        self.kill_in_flight_pids.contains(&pid) || self.kill_waiting_refresh_pids.contains(&pid)
    }

    #[allow(dead_code)]
    pub fn killable_process_count(&self) -> usize {
        self.killable_pids().len()
    }

    pub fn is_process_killable(&self, process: &ProcessSummary) -> bool {
        is_killable_process(process) && !self.is_kill_pending(process.pid)
    }

    pub fn settings(&self) -> &AppSettings {
        &self.settings
    }

    pub fn apply_settings(&mut self, mut settings: AppSettings) {
        let previous = self.settings.clone();
        let refresh_changed = previous.refresh_interval != settings.refresh_interval;
        let startup_changed = previous.launch_at_startup != settings.launch_at_startup;
        let mut has_error = false;

        log_info!(
            "apply settings requested: refresh={}s->{}s launch_at_startup={}->{} kill_behavior={}->{}",
            previous.refresh_interval.seconds(),
            settings.refresh_interval.seconds(),
            previous.launch_at_startup,
            settings.launch_at_startup,
            previous.kill_behavior,
            settings.kill_behavior
        );

        if startup_changed {
            match write_launch_at_startup(settings.launch_at_startup) {
                Ok(()) => {}
                Err(message) => {
                    log_warn!("failed to update launch at startup: {message}");
                    has_error = true;
                    settings.launch_at_startup = previous.launch_at_startup;
                    self.push_toast(ToastUpdate {
                        level: ToastLevel::Error,
                        message: format!("Failed to update launch at startup: {message}"),
                    });
                }
            }
        }

        if refresh_changed {
            self.poll_interval_ms.store(
                settings.refresh_interval.duration().as_millis() as u64,
                Ordering::Relaxed,
            );
        }

        self.settings = settings;
        let settings_changed = self.settings != previous;

        if settings_changed {
            if let Err(error) = self.settings.save() {
                log_error!("failed to save settings: {error}");
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Error,
                    message: format!("Failed to save settings: {error}"),
                });
            } else if !has_error {
                log_info!("settings applied successfully");
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Success,
                    message: "Settings saved".to_owned(),
                });
            }
        }

        if refresh_changed {
            self.request_collection();
        }

        self.ctx.request_repaint();
    }

    pub fn launch_at_startup_supported(&self) -> bool {
        launch_at_startup_supported()
    }

    pub fn kill_requires_confirmation(&self) -> bool {
        self.settings.kill_behavior == KillBehavior::Confirm
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
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Success,
                    message: format!("Killed PID {}", kill.pid),
                });
                let _ = self
                    .command_tx
                    .send(CollectionCommand::UpdateRequest(self.current_request()));
            }
            Err(message) => {
                self.kill_waiting_refresh_pids.remove(&kill.pid);
                self.push_toast(ToastUpdate {
                    level: ToastLevel::Error,
                    message: format!("Failed to kill PID {}: {message}", kill.pid),
                });
            }
        }

        self.ctx.request_repaint();
    }

    fn handle_kill_all_update(&mut self, kill_all: KillAllUpdate) {
        for pid in &kill_all.successes {
            self.kill_in_flight_pids.remove(pid);
            self.kill_waiting_refresh_pids.insert(*pid);
        }

        for failure in &kill_all.failures {
            self.kill_in_flight_pids.remove(&failure.pid);
            self.kill_waiting_refresh_pids.remove(&failure.pid);
        }

        let summary = format_kill_all_summary(&kill_all.successes, &kill_all.failures);
        let level = if kill_all.successes.is_empty() {
            ToastLevel::Error
        } else {
            ToastLevel::Success
        };

        self.push_toast(ToastUpdate {
            level,
            message: summary,
        });

        if !kill_all.successes.is_empty() {
            let _ = self
                .command_tx
                .send(CollectionCommand::UpdateRequest(self.current_request()));
        }

        self.ctx.request_repaint();
    }

    #[allow(dead_code)]
    fn killable_pids(&self) -> Vec<u32> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };

        snapshot
            .items
            .iter()
            .filter(|process| self.is_process_killable(process))
            .map(|process| process.pid)
            .collect()
    }

    fn normalize_kill_pids(&self, pids: Vec<u32>) -> Vec<u32> {
        let Some(snapshot) = &self.snapshot else {
            return Vec::new();
        };

        let mut seen = HashSet::new();
        pids.into_iter()
            .filter(|pid| seen.insert(*pid))
            .filter(|pid| {
                snapshot
                    .items
                    .iter()
                    .find(|process| process.pid == *pid)
                    .map(|process| self.is_process_killable(process))
                    .unwrap_or(false)
            })
            .collect()
    }
}

fn elapsed_since(collected_at: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(collected_at).unwrap_or_default()
}

fn kill_process_by_pid(pid: u32) -> Result<(), localporter_core::SourceError> {
    let runner = StdCommandRunner::default();
    let pid_arg = pid.to_string();

    #[cfg(target_os = "windows")]
    {
        runner.run("taskkill", &["/PID", &pid_arg, "/T", "/F"])?;
    }

    #[cfg(target_os = "macos")]
    {
        runner.run("kill", &["-9", &pid_arg])?;
    }

    log_info!("kill command succeeded: pid={pid}");
    Ok(())
}

fn browser_url(port: u16) -> String {
    format!("http://127.0.0.1:{port}")
}

fn open_url_in_browser(url: &str) -> io::Result<()> {
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(url)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn()?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "open in browser is not supported on this platform",
    ))
}

fn format_browser_open_error(error: &io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "browser launcher command not found".to_owned(),
        io::ErrorKind::PermissionDenied => "permission denied".to_owned(),
        _ => error.to_string(),
    }
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
        localporter_core::SourceError::CommandTimedOut { program } => {
            format!("{program} timed out")
        }
        localporter_core::SourceError::PermissionDenied { program } => {
            format!("permission denied for {program}")
        }
        localporter_core::SourceError::InvalidOutput { source } => {
            format!("invalid output from {source}")
        }
    }
}

fn format_kill_all_summary(successes: &[u32], failures: &[KillFailure]) -> String {
    match (successes.len(), failures.len()) {
        (0, 0) => "No killable processes".to_owned(),
        (success_count, 0) => format!("Killed {success_count} killable process(es)"),
        (0, 1) => format!(
            "Failed to kill PID {}: {}",
            failures[0].pid, failures[0].message
        ),
        (0, failure_count) => format!("Failed to kill {failure_count} killable process(es)"),
        (success_count, failure_count) => {
            format!("Killed {success_count} process(es), {failure_count} failed")
        }
    }
}

fn is_killable_process(process: &localporter_core::ProcessSummary) -> bool {
    if process.pid == 0 {
        return false;
    }

    let name = process.name.trim();
    if name.is_empty() || name.eq_ignore_ascii_case("Unknown") {
        return false;
    }

    let command = process.command.trim();
    let launcher = process.launcher.trim();

    if command.is_empty() || command.eq_ignore_ascii_case("Unknown") {
        return false;
    }

    if command.eq_ignore_ascii_case(name) && launcher.eq_ignore_ascii_case("Unknown") {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        const WINDOWS_BLOCKLIST: &[&str] = &[
            "system",
            "system idle process",
            "registry",
            "memory compression",
            "secure system",
            "smss.exe",
            "csrss.exe",
            "wininit.exe",
            "services.exe",
            "lsass.exe",
            "winlogon.exe",
            "dwm.exe",
            "fontdrvhost.exe",
            "svchost.exe",
        ];

        if process.pid == 4
            || WINDOWS_BLOCKLIST
                .iter()
                .any(|blocked| name.eq_ignore_ascii_case(blocked))
        {
            return false;
        }

        if WINDOWS_BLOCKLIST
            .iter()
            .any(|blocked| launcher.eq_ignore_ascii_case(blocked))
        {
            return false;
        }
    }

    #[cfg(target_os = "macos")]
    {
        const MACOS_BLOCKLIST: &[&str] = &[
            "launchd",
            "kernel_task",
            "WindowServer",
            "Finder",
            "Dock",
            "loginwindow",
            "ControlCenter",
            "SystemUIServer",
        ];

        if process.pid == 1
            || MACOS_BLOCKLIST
                .iter()
                .any(|blocked| name.eq_ignore_ascii_case(blocked))
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use localporter_core::ProcessSummary;

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
            settings: AppSettings::default(),
            ctx: egui::Context::default(),
            request_id: 0,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
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
            settings: AppSettings::default(),
            ctx: egui::Context::default(),
            request_id: 1,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::new(),
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
            command_tx,
            update_tx,
            update_rx,
        };
        state.drain_updates();

        assert_eq!(state.snapshot, Some(fresh_snapshot));
    }

    #[test]
    fn toast_views_drop_expired_items() {
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            settings: AppSettings::default(),
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
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
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
            settings: AppSettings::default(),
            ctx: egui::Context::default(),
            request_id: 1,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::new(),
            kill_waiting_refresh_pids: HashSet::from([42]),
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
            command_tx,
            update_tx,
            update_rx,
        };

        state.drain_updates();

        assert!(!state.is_kill_pending(42));
    }

    #[test]
    fn killable_process_count_excludes_pending_pids() {
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, _command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: Some(ProcessSnapshot {
                collected_at: SystemTime::UNIX_EPOCH,
                items: vec![
                    test_process(101, "node.exe", "node server.js", "powershell.exe"),
                    test_process(202, "python.exe", "python -m http.server", "Code.exe"),
                ],
                warnings: Vec::new(),
            }),
            settings: AppSettings::default(),
            ctx: egui::Context::default(),
            request_id: 0,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::from([101]),
            kill_waiting_refresh_pids: HashSet::new(),
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
            command_tx,
            update_tx,
            update_rx,
        };

        assert_eq!(state.killable_process_count(), 1);

        state.kill_waiting_refresh_pids.insert(202);
        assert_eq!(state.killable_process_count(), 0);
    }

    #[test]
    fn kill_all_update_requests_single_refresh_and_summary_toast() {
        let (update_tx, update_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        let mut state = AppState {
            show_all_enabled: false,
            snapshot: None,
            settings: AppSettings::default(),
            ctx: egui::Context::default(),
            request_id: 7,
            next_toast_id: 0,
            toasts: Vec::new(),
            kill_in_flight_pids: HashSet::from([100, 200]),
            kill_waiting_refresh_pids: HashSet::new(),
            poll_interval_ms: Arc::new(AtomicU64::new(2_000)),
            command_tx,
            update_tx,
            update_rx,
        };

        state.handle_kill_all_update(KillAllUpdate {
            successes: vec![100],
            failures: vec![KillFailure {
                pid: 200,
                message: "permission denied".to_owned(),
            }],
        });

        assert!(state.kill_waiting_refresh_pids.contains(&100));
        assert!(!state.kill_waiting_refresh_pids.contains(&200));
        assert!(!state.kill_in_flight_pids.contains(&100));
        assert!(!state.kill_in_flight_pids.contains(&200));

        assert_eq!(
            command_rx.try_recv().unwrap(),
            CollectionCommand::UpdateRequest(CollectionRequest {
                scope: PortQueryScope::ListenOnly,
                request_id: 7,
            })
        );

        let toasts = state.toast_views();
        assert_eq!(toasts.len(), 1);
        assert_eq!(toasts[0].level, ToastLevel::Success);
        assert_eq!(toasts[0].message, "Killed 1 process(es), 1 failed");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_system_processes_are_not_killable_in_batch() {
        assert!(!is_killable_process(&test_process(
            4,
            "System",
            "System",
            "services.exe",
        )));
        assert!(!is_killable_process(&test_process(
            120,
            "svchost.exe",
            "C:\\Windows\\System32\\svchost.exe -k LocalService",
            "services.exe",
        )));
        assert!(is_killable_process(&test_process(
            4321,
            "node.exe",
            "node server.js",
            "powershell.exe",
        )));
    }

    fn test_process(pid: u32, name: &str, command: &str, launcher: &str) -> ProcessSummary {
        ProcessSummary {
            pid,
            name: name.to_owned(),
            command: command.to_owned(),
            ports: Vec::new(),
            launcher: launcher.to_owned(),
            uptime: Duration::ZERO,
            cpu_percent: 0.0,
            memory_usage: 0,
        }
    }

    #[test]
    fn browser_url_targets_loopback_http() {
        assert_eq!(browser_url(3000), "http://127.0.0.1:3000");
    }
}
