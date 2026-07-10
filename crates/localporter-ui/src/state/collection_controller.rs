use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
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
use localporter_core::{PortQueryScope, SnapshotService, log_debug, log_info};

use super::app_state::{AppUpdate, SnapshotUpdate};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CollectionCommand {
    UpdateRequest(CollectionRequest),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CollectionRequest {
    pub(super) scope: PortQueryScope,
    pub(super) request_id: u64,
}

pub(super) fn spawn_collection_worker(
    command_rx: mpsc::Receiver<CollectionCommand>,
    update_tx: mpsc::Sender<AppUpdate>,
    ctx: egui::Context,
    poll_interval_ms: Arc<AtomicU64>,
    initial_request: CollectionRequest,
) {
    thread::spawn(move || {
        log_info!("collection worker started");
        let service = build_snapshot_service();
        let mut request = initial_request;

        loop {
            log_debug!(
                "collect snapshot start: request_id={} scope={:?}",
                request.request_id,
                request.scope
            );
            let snapshot = service.collect_snapshot(request.scope);

            let latest_request = drain_latest_request(&command_rx, request);
            let is_current = latest_request.request_id == request.request_id;
            log_debug!(
                "collect snapshot complete: request_id={} scope={:?} items={} warnings={} is_current={}",
                request.request_id,
                request.scope,
                snapshot.items.len(),
                snapshot.warnings.len(),
                is_current
            );

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

            if !is_current {
                request = latest_request;
                continue;
            }

            let timeout = Duration::from_millis(poll_interval_ms.load(Ordering::Relaxed).max(250));
            match command_rx.recv_timeout(timeout) {
                Ok(CollectionCommand::UpdateRequest(next_request)) => {
                    request = drain_latest_request(&command_rx, next_request);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    log_info!("collection worker stopped: command channel disconnected");
                    break;
                }
            }
        }
    });
}

fn drain_latest_request(
    command_rx: &mpsc::Receiver<CollectionCommand>,
    mut latest: CollectionRequest,
) -> CollectionRequest {
    while let Ok(CollectionCommand::UpdateRequest(request)) = command_rx.try_recv() {
        latest = request;
    }
    latest
}

#[cfg(target_os = "windows")]
fn build_snapshot_service() -> SnapshotService {
    let runner = Arc::new(StdCommandRunner::default());
    SnapshotService::new(
        Arc::new(NetConnectionPortSource::new(runner.clone())),
        Arc::new(CimProcessInfoSource::new(runner.clone())),
        Arc::new(CimParentChainSource::new(runner)),
    )
}

#[cfg(target_os = "macos")]
fn build_snapshot_service() -> SnapshotService {
    let runner = Arc::new(StdCommandRunner::default());
    SnapshotService::new(
        Arc::new(LsofPortSource::new(runner.clone())),
        Arc::new(PsProcessInfoSource::new(runner.clone())),
        Arc::new(PsParentChainSource::new(runner)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queued_requests_are_coalesced_to_the_latest() {
        let (tx, rx) = mpsc::channel();
        tx.send(CollectionCommand::UpdateRequest(CollectionRequest {
            scope: PortQueryScope::AllTcp,
            request_id: 2,
        }))
        .unwrap();
        tx.send(CollectionCommand::UpdateRequest(CollectionRequest {
            scope: PortQueryScope::ListenOnly,
            request_id: 3,
        }))
        .unwrap();

        let latest = drain_latest_request(
            &rx,
            CollectionRequest {
                scope: PortQueryScope::ListenOnly,
                request_id: 1,
            },
        );

        assert_eq!(latest.request_id, 3);
        assert_eq!(latest.scope, PortQueryScope::ListenOnly);
    }
}
