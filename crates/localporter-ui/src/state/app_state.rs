use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};

use eframe::egui;
use localporter_core::{
    ProcessSnapshot, SnapshotService,
    adapter::macos::command::{
        LsofPortSource, PsParentChainSource, PsProcessInfoSource, StdCommandRunner,
    },
};

const SNAPSHOT_POLL_INTERVAL: Duration = Duration::from_secs(2);

pub struct AppState {
    pub status_text: String,
    pub snapshot: Option<ProcessSnapshot>,
    snapshot_rx: mpsc::Receiver<ProcessSnapshot>,
}

impl AppState {
    pub fn new(ctx: egui::Context) -> Self {
        let (snapshot_tx, snapshot_rx) = mpsc::channel();

        thread::spawn(move || {
            let runner = Arc::new(StdCommandRunner);
            let service = SnapshotService::new(
                Arc::new(LsofPortSource::new(runner.clone())),
                Arc::new(PsProcessInfoSource::new(runner.clone())),
                Arc::new(PsParentChainSource::new(runner)),
            );

            loop {
                let snapshot = service.collect_snapshot();
                if snapshot_tx.send(snapshot).is_err() {
                    break;
                }

                ctx.request_repaint();
                thread::sleep(SNAPSHOT_POLL_INTERVAL);
            }
        });

        Self {
            status_text: "Collecting TCP LISTEN + UDP Bound every 2s...".to_owned(),
            snapshot: None,
            snapshot_rx,
        }
    }

    pub fn drain_updates(&mut self) {
        let mut latest_snapshot = None;

        while let Ok(snapshot) = self.snapshot_rx.try_recv() {
            latest_snapshot = Some(snapshot);
        }

        if let Some(snapshot) = latest_snapshot {
            self.status_text = format!(
                "Last snapshot: {} processes, {} warnings",
                snapshot.items.len(),
                snapshot.warnings.len()
            );
            self.snapshot = Some(snapshot);
        }
    }
}
