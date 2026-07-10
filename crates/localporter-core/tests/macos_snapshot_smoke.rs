#![cfg(target_os = "macos")]

use std::sync::Arc;

use localporter_core::{
    PortQueryScope, SnapshotService,
    adapter::macos::command::{
        LsofPortSource, PsParentChainSource, PsProcessInfoSource, StdCommandRunner,
    },
};

#[test]
fn collects_local_ports_with_best_effort_and_prints_snapshot() {
    let runner = Arc::new(StdCommandRunner::default());
    let service = SnapshotService::new(
        Arc::new(LsofPortSource::new(runner.clone())),
        Arc::new(PsProcessInfoSource::new(runner.clone())),
        Arc::new(PsParentChainSource::new(runner)),
    );

    let snapshot = service.collect_snapshot(PortQueryScope::ListenOnly);

    println!("collected_at: {:?}", snapshot.collected_at);
    println!("items: {}", snapshot.items.len());

    for item in &snapshot.items {
        let tcp_ports = item.tcp_ports();
        let udp_ports = item.udp_ports();
        println!(
            "name={} launcher={} tcp_ports={:?} udp_ports={:?} uptime_secs={} cpu_percent={:.2} memory_bytes={}",
            item.name_or_unknown(),
            item.launcher,
            tcp_ports,
            udp_ports,
            item.uptime.as_secs(),
            item.cpu_percent,
            item.memory_usage
        );
    }

    if snapshot.warnings.is_empty() {
        println!("warnings: none");
    } else {
        println!("warnings: {}", snapshot.warnings.len());
        for warning in &snapshot.warnings {
            println!("warning={warning:?}");
        }
    }

    assert!(snapshot.items.iter().all(|item| !item.ports.is_empty()));
    assert!(
        snapshot
            .items
            .iter()
            .all(|item| item.ports.iter().all(|port| port.port > 0))
    );
}
