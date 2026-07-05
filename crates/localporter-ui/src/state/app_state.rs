use std::{
    sync::{Arc, mpsc},
    thread,
    time::{Duration, SystemTime},
};

use eframe::egui;
use localporter_core::{
    PortQueryScope, ProcessSnapshot, SnapshotService,
    adapter::macos::command::{
        LsofPortSource, PsParentChainSource, PsProcessInfoSource, StdCommandRunner,
    },
};

const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct AppState {
    pub show_all_enabled: bool,
    pub snapshot: Option<ProcessSnapshot>,
    command_tx: mpsc::Sender<CollectionCommand>,
    snapshot_rx: mpsc::Receiver<ProcessSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionCommand {
    UpdateScope(PortQueryScope),
}

impl AppState {
    pub fn new(ctx: egui::Context) -> Self {
        let initial_scope = PortQueryScope::ListenOnly;
        let (snapshot_tx, snapshot_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();

        thread::spawn(move || {
            let runner = Arc::new(StdCommandRunner);
            let service = SnapshotService::new(
                Arc::new(LsofPortSource::new(runner.clone())),
                Arc::new(PsProcessInfoSource::new(runner.clone())),
                Arc::new(PsParentChainSource::new(runner)),
            );
            let mut scope = initial_scope;

            loop {
                let snapshot = service.collect_snapshot(scope);
                if snapshot_tx.send(snapshot).is_err() {
                    break;
                }

                ctx.request_repaint();

                match command_rx.recv_timeout(SNAPSHOT_POLL_INTERVAL) {
                    Ok(CollectionCommand::UpdateScope(next_scope)) => {
                        scope = next_scope;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        Self {
            show_all_enabled: false,
            snapshot: None,
            command_tx,
            snapshot_rx,
        }
    }

    pub fn set_show_all_enabled(&mut self, enabled: bool) {
        if self.show_all_enabled == enabled {
            return;
        }

        self.show_all_enabled = enabled;
        self.sync_collection_scope();
    }

    fn sync_collection_scope(&mut self) {
        let scope = self.port_query_scope();
        let _ = self.command_tx.send(CollectionCommand::UpdateScope(scope));
    }

    pub fn drain_updates(&mut self) {
        let mut latest_snapshot = None;

        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            latest_snapshot = Some(snapshot);
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
}
