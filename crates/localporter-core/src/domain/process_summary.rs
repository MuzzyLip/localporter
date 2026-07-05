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

    pub fn primary_port(&self) -> Option<BoundPort> {
        self.ports.first().copied()
    }

    pub fn remaining_port_count(&self) -> usize {
        self.ports.len().saturating_sub(1)
    }

    pub fn primary_port_text(&self) -> String {
        let Some(primary_port) = self.primary_port() else {
            return "Unknown".to_owned();
        };

        let remaining_count = self.remaining_port_count();
        if remaining_count == 0 {
            format!(":{}", primary_port.port)
        } else {
            format!(":{} +{remaining_count}", primary_port.port)
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
            name: "demo".to_owned(),
            ports,
            launcher: "Terminal".to_owned(),
            uptime: Duration::ZERO,
            cpu_percent: 0.0,
            memory_usage: 0,
        }
    }

    #[test]
    fn primary_port_text_returns_unknown_when_no_ports_exist() {
        let summary = summary_with_ports(Vec::new());

        assert_eq!(summary.primary_port_text(), "Unknown");
        assert_eq!(summary.remaining_port_count(), 0);
        assert_eq!(summary.primary_port(), None);
    }

    #[test]
    fn primary_port_text_returns_single_port_without_suffix() {
        let summary = summary_with_ports(vec![BoundPort {
            protocol: PortProtocol::Tcp,
            port: 3000,
        }]);

        assert_eq!(summary.primary_port_text(), "3000");
        assert_eq!(summary.remaining_port_count(), 0);
    }

    #[test]
    fn primary_port_text_appends_remaining_port_count() {
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

        assert_eq!(summary.primary_port_text(), "3000 +2");
        assert_eq!(summary.remaining_port_count(), 2);
    }
}
