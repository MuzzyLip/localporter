use std::time::Duration;

use crate::domain::{BoundPort, PortProtocol};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessSummary {
    pub pid: u32,
    pub name: String,
    pub command: String,
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn summary_with_ports(ports: Vec<BoundPort>) -> ProcessSummary {
        ProcessSummary {
            pid: 1234,
            name: "demo".to_owned(),
            command: "demo".to_owned(),
            ports,
            launcher: "Terminal".to_owned(),
            uptime: Duration::ZERO,
            cpu_percent: 0.0,
            memory_usage: 0,
        }
    }

    #[test]
    fn tcp_ports_only_include_tcp_bindings() {
        let summary = summary_with_ports(vec![
            BoundPort {
                protocol: PortProtocol::Tcp,
                port: 3000,
            },
            BoundPort {
                protocol: PortProtocol::Tcp,
                port: 9229,
            },
            BoundPort {
                protocol: PortProtocol::Udp,
                port: 5353,
            },
        ]);

        assert_eq!(summary.tcp_ports(), vec![3000, 9229]);
    }

    #[test]
    fn udp_ports_only_include_udp_bindings() {
        let summary = summary_with_ports(vec![
            BoundPort {
                protocol: PortProtocol::Tcp,
                port: 3000,
            },
            BoundPort {
                protocol: PortProtocol::Udp,
                port: 5353,
            },
            BoundPort {
                protocol: PortProtocol::Udp,
                port: 5354,
            },
        ]);

        assert_eq!(summary.udp_ports(), vec![5353, 5354]);
    }
}
