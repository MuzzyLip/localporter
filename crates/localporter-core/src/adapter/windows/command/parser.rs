use std::time::Duration;

use crate::{
    domain::{BoundPort, PortProtocol, PortQueryScope, ProcessPortBinding},
    error::SourceError,
    sources::ProcessInfo,
};

pub type CimProcessRow = (u32, Option<u32>, String);
const FIELD_SEPARATOR: char = '\u{1f}';

pub fn parse_net_connection_ports(
    raw: &str,
    scope: PortQueryScope,
) -> Result<Vec<ProcessPortBinding>, SourceError> {
    let mut bindings = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(protocol) = fields.first().copied() else {
            continue;
        };

        match protocol {
            "TCP" => {
                let [_, local_address, _, state, pid, ..] = fields.as_slice() else {
                    return Err(SourceError::InvalidOutput {
                        source: "windows_net_connections",
                    });
                };
                if !tcp_state_matches_scope(scope, state) {
                    continue;
                }
                let pid = parse_pid(pid)?;
                if pid == 0 {
                    continue;
                }

                bindings.push(ProcessPortBinding {
                    pid,
                    process_name: String::new(),
                    port: BoundPort {
                        protocol: PortProtocol::Tcp,
                        port: parse_local_port(local_address)?,
                    },
                });
            }
            "UDP" => {
                let [_, local_address, _, pid, ..] = fields.as_slice() else {
                    return Err(SourceError::InvalidOutput {
                        source: "windows_net_connections",
                    });
                };
                let pid = parse_pid(pid)?;
                if pid == 0 {
                    continue;
                }

                bindings.push(ProcessPortBinding {
                    pid,
                    process_name: String::new(),
                    port: BoundPort {
                        protocol: PortProtocol::Udp,
                        port: parse_local_port(local_address)?,
                    },
                });
            }
            _ => continue,
        }
    }

    Ok(bindings)
}

pub fn parse_cim_process_info(raw: &str) -> Result<Vec<ProcessInfo>, SourceError> {
    let mut items = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let mut fields = line.splitn(8, FIELD_SEPARATOR);
        let pid = fields
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or(SourceError::InvalidOutput {
                source: "windows_process_info",
            })?;
        let ppid = parse_optional_pid(fields.next());
        let name = fields
            .next()
            .map(str::to_owned)
            .ok_or(SourceError::InvalidOutput {
                source: "windows_process_info",
            })?;
        let uptime = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .ok_or(SourceError::InvalidOutput {
                source: "windows_process_info",
            })?;
        let memory_bytes = fields
            .next()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(SourceError::InvalidOutput {
                source: "windows_process_info",
            })?;
        let cpu_percent = fields
            .next()
            .and_then(|value| value.parse::<f32>().ok())
            .ok_or(SourceError::InvalidOutput {
                source: "windows_process_info",
            })?;
        let command_line = fields
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let executable_path = fields
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        items.push(ProcessInfo {
            pid,
            ppid,
            name,
            command_line: command_line.map(str::to_owned),
            executable_path: executable_path.map(str::to_owned),
            uptime: Some(uptime),
            cpu_percent: Some(cpu_percent),
            memory_bytes: Some(memory_bytes),
        });
    }

    Ok(items)
}

pub fn parse_cim_process_rows(raw: &str) -> Result<Vec<CimProcessRow>, SourceError> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| parse_cim_pid_parent_name_row(line, "windows_process_rows"))
        .collect()
}

fn parse_optional_pid(value: Option<&str>) -> Option<u32> {
    value
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|pid| *pid != 0)
}

fn parse_pid(raw: &str) -> Result<u32, SourceError> {
    raw.parse::<u32>().map_err(|_| SourceError::InvalidOutput {
        source: "windows_net_connections",
    })
}

fn parse_local_port(raw: &str) -> Result<u16, SourceError> {
    raw.rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok())
        .ok_or(SourceError::InvalidOutput {
            source: "windows_net_connections",
        })
}

fn tcp_state_matches_scope(scope: PortQueryScope, state: &str) -> bool {
    match scope {
        PortQueryScope::ListenOnly => state == "LISTENING",
        PortQueryScope::AllTcp => matches!(state, "LISTENING" | "TIME_WAIT" | "CLOSE_WAIT"),
    }
}

fn parse_cim_pid_parent_name_row(
    line: &str,
    source: &'static str,
) -> Result<CimProcessRow, SourceError> {
    let mut fields = line.splitn(3, '|');
    let pid = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(SourceError::InvalidOutput { source })?;
    let ppid = parse_optional_pid(fields.next());
    let name = fields
        .next()
        .map(str::to_owned)
        .ok_or(SourceError::InvalidOutput { source })?;

    Ok((pid, ppid, name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_windows_listen_and_udp_rows_from_netstat() {
        let raw = concat!(
            "Active Connections\n\n",
            "  Proto  Local Address          Foreign Address        State           PID\n",
            "  TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       1234\n",
            "  TCP    127.0.0.1:53412        127.0.0.1:53413        ESTABLISHED     9000\n",
            "  UDP    0.0.0.0:5353           *:*                                    4321\n"
        );
        let bindings = parse_net_connection_ports(raw, PortQueryScope::ListenOnly).unwrap();

        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].pid, 1234);
        assert_eq!(bindings[0].port.protocol, PortProtocol::Tcp);
        assert_eq!(bindings[0].port.port, 3000);
        assert_eq!(bindings[1].pid, 4321);
        assert_eq!(bindings[1].port.protocol, PortProtocol::Udp);
        assert_eq!(bindings[1].port.port, 5353);
    }

    #[test]
    fn includes_time_wait_and_close_wait_only_in_show_all_scope() {
        let raw = concat!(
            "  Proto  Local Address          Foreign Address        State           PID\n",
            "  TCP    127.0.0.1:51824        127.0.0.1:3000         TIME_WAIT       2000\n",
            "  TCP    127.0.0.1:51825        127.0.0.1:3000         CLOSE_WAIT      2001\n",
            "  TCP    127.0.0.1:3000         0.0.0.0:0              LISTENING       1234\n"
        );

        let listen_only = parse_net_connection_ports(raw, PortQueryScope::ListenOnly).unwrap();
        let show_all = parse_net_connection_ports(raw, PortQueryScope::AllTcp).unwrap();

        assert_eq!(listen_only.len(), 1);
        assert_eq!(listen_only[0].port.port, 3000);
        assert_eq!(show_all.len(), 3);
        assert_eq!(show_all[0].port.port, 51824);
        assert_eq!(show_all[1].port.port, 51825);
        assert_eq!(show_all[2].port.port, 3000);
    }

    #[test]
    fn skips_pid_zero_rows_from_netstat_output() {
        let raw = concat!(
            "  Proto  Local Address          Foreign Address        State           PID\n",
            "  TCP    127.0.0.1:51824        127.0.0.1:3000         TIME_WAIT       0\n",
            "  UDP    0.0.0.0:5353           *:*                                    0\n",
            "  TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       1234\n"
        );

        let bindings = parse_net_connection_ports(raw, PortQueryScope::AllTcp).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].pid, 1234);
        assert_eq!(bindings[0].port.protocol, PortProtocol::Tcp);
        assert_eq!(bindings[0].port.port, 3000);
    }

    #[test]
    fn parses_windows_process_info_rows() {
        let raw = "1234\u{1f}567\u{1f}node.exe\u{1f}42\u{1f}1048576\u{1f}3.5\u{1f}\"C:\\Program Files\\nodejs\\node.exe\" server.js\u{1f}C:\\Program Files\\nodejs\\node.exe\n";
        let items = parse_cim_process_info(raw).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].pid, 1234);
        assert_eq!(items[0].ppid, Some(567));
        assert_eq!(items[0].name, "node.exe");
        assert_eq!(
            items[0].command_line.as_deref(),
            Some("\"C:\\Program Files\\nodejs\\node.exe\" server.js")
        );
        assert_eq!(
            items[0].executable_path.as_deref(),
            Some("C:\\Program Files\\nodejs\\node.exe")
        );
        assert_eq!(items[0].uptime, Some(Duration::from_secs(42)));
        assert_eq!(items[0].memory_bytes, Some(1_048_576));
        assert_eq!(items[0].cpu_percent, Some(3.5));
    }

    #[test]
    fn parses_multiple_windows_process_rows() {
        let raw = "123|1|explorer.exe\n456|123|cmd.exe\n";
        let rows = parse_cim_process_rows(raw).unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (123, Some(1), "explorer.exe".to_owned()));
        assert_eq!(rows[1], (456, Some(123), "cmd.exe".to_owned()));
    }
}
