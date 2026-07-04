use std::time::Duration;

use crate::domain::{BoundPort, PortProtocol};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSummary {
    pub name: String,
    pub ports: Vec<BoundPort>,
    pub launcher: String,
    pub uptime: Duration,
    pub cpu_percent: f32,
    pub memory_usage: u64,
}

impl ProcessSummary {
    pub fn name_or_unknown(&self) -> &str {
        if self.name.is_empty() {
            "Unknown"
        } else {
            &self.name
        }
    }

    pub fn tcp_ports(&self) -> Vec<u16> {
        self.ports
            .iter()
            .filter(|port| port.protocol == PortProtocol::Tcp)
            .map(|port| port.port)
            .collect()
    }

    pub fn udp_ports(&self) -> Vec<u16> {
        self.ports
            .iter()
            .filter(|port| port.protocol == PortProtocol::Udp)
            .map(|port| port.port)
            .collect()
    }
}
