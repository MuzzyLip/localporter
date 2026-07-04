use std::time::Duration;

use crate::{
    domain::{BoundPort, PortProtocol, ProcessPortBinding},
    error::SourceError,
    sources::ProcessInfo,
};

pub fn parse_lsof_ports(
    raw: &str,
    protocol: PortProtocol,
) -> Result<Vec<ProcessPortBinding>, SourceError> {
    let mut current_pid = None;
    let mut current_name = String::new();
    let mut bindings = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (field, value) = line.split_at(1);
        match field {
            "p" => current_pid = value.parse::<u32>().ok(),
            "c" => current_name = value.to_owned(),
            "n" => {
                let Some(pid) = current_pid else {
                    continue;
                };
                let Some(port) = extract_port(value) else {
                    continue;
                };

                bindings.push(ProcessPortBinding {
                    pid,
                    process_name: current_name.clone(),
                    port: BoundPort { protocol, port },
                });
            }
            _ => {}
        }
    }

    Ok(bindings)
}

pub fn parse_ps_process_info(raw: &str) -> Result<Vec<ProcessInfo>, SourceError> {
    let mut items = Vec::new();

    for line in raw.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let (pid, rest) = take_field(line).ok_or(SourceError::InvalidOutput {
            source: "ps_process_info",
        })?;
        let (ppid, rest) = take_field(rest).ok_or(SourceError::InvalidOutput {
            source: "ps_process_info",
        })?;
        let (cpu, rest) = take_field(rest).ok_or(SourceError::InvalidOutput {
            source: "ps_process_info",
        })?;
        let (rss, rest) = take_field(rest).ok_or(SourceError::InvalidOutput {
            source: "ps_process_info",
        })?;
        let (etime, rest) = take_field(rest).ok_or(SourceError::InvalidOutput {
            source: "ps_process_info",
        })?;
        let command = rest.trim();

        items.push(ProcessInfo {
            pid: pid.parse().map_err(|_| SourceError::InvalidOutput {
                source: "ps_process_info",
            })?,
            ppid: parse_optional_pid(ppid),
            name: extract_process_name(command),
            uptime: parse_etime(etime),
            cpu_percent: cpu.parse::<f32>().ok(),
            memory_bytes: rss.parse::<u64>().ok().map(|value| value * 1024),
        });
    }

    Ok(items)
}

pub fn parse_ps_parent_row(raw: &str) -> Result<(u32, Option<u32>, String), SourceError> {
    let line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or(SourceError::InvalidOutput {
            source: "ps_parent_row",
        })?;

    let (pid, rest) = take_field(line).ok_or(SourceError::InvalidOutput {
        source: "ps_parent_row",
    })?;
    let (ppid, rest) = take_field(rest).ok_or(SourceError::InvalidOutput {
        source: "ps_parent_row",
    })?;
    let command = rest.trim();

    Ok((
        pid.parse().map_err(|_| SourceError::InvalidOutput {
            source: "ps_parent_row",
        })?,
        parse_optional_pid(ppid),
        extract_process_name(command),
    ))
}

pub fn parse_etime(raw: &str) -> Option<Duration> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let (days, time_part) = match raw.split_once('-') {
        Some((days, rest)) => (days.parse::<u64>().ok()?, rest),
        None => (0, raw),
    };

    let segments = time_part
        .split(':')
        .map(|segment| segment.parse::<u64>().ok())
        .collect::<Option<Vec<_>>>()?;

    let seconds = match segments.as_slice() {
        [minutes, seconds] => minutes * 60 + seconds,
        [hours, minutes, seconds] => hours * 3600 + minutes * 60 + seconds,
        _ => return None,
    };

    Some(Duration::from_secs(days * 86_400 + seconds))
}

fn extract_port(value: &str) -> Option<u16> {
    let value = value.trim().split_whitespace().next()?;
    value.rsplit(':').next()?.parse::<u16>().ok()
}

fn take_field(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    if input.is_empty() {
        return None;
    }

    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    Some((&input[..end], &input[end..]))
}

fn parse_optional_pid(value: &str) -> Option<u32> {
    value.parse::<u32>().ok().filter(|pid| *pid != 0)
}

fn extract_process_name(command: &str) -> String {
    let command = command.trim();
    if command.is_empty() {
        return String::new();
    }

    if let Some((_, rest)) = command.split_once(".app/Contents/MacOS/") {
        return rest.split_whitespace().next().unwrap_or(rest).to_owned();
    }

    let first_token = command.split_whitespace().next().unwrap_or(command);
    first_token
        .rsplit('/')
        .next()
        .unwrap_or(first_token)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lsof_tcp_ports() {
        let raw = "p29220\ncCode Helper (Plugin)\nf35\nn127.0.0.1:60027\np30388\ncrust-analyzer\nn*:50051\n";
        let bindings = parse_lsof_ports(raw, PortProtocol::Tcp).unwrap();

        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].pid, 29220);
        assert_eq!(bindings[0].process_name, "Code Helper (Plugin)");
        assert_eq!(bindings[0].port.protocol, PortProtocol::Tcp);
        assert_eq!(bindings[0].port.port, 60027);
        assert_eq!(bindings[1].port.port, 50051);
    }

    #[test]
    fn parses_lsof_udp_ports_and_skips_wildcards_without_numeric_port() {
        let raw = "p450\ncidentityservicesd\nn*:*\np466\ncsharingd\nn*:54389\n";
        let bindings = parse_lsof_ports(raw, PortProtocol::Udp).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].pid, 466);
        assert_eq!(bindings[0].process_name, "sharingd");
        assert_eq!(bindings[0].port.protocol, PortProtocol::Udp);
        assert_eq!(bindings[0].port.port, 54389);
    }

    #[test]
    fn parses_ps_process_info_rows() {
        let raw = "29220 28785 0.0 32064 01:45:22 /Applications/Visual Studio Code.app/Contents/MacOS/Code --type=renderer\n";
        let items = parse_ps_process_info(raw).unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].pid, 29220);
        assert_eq!(items[0].ppid, Some(28785));
        assert_eq!(items[0].name, "Code");
        assert_eq!(items[0].memory_bytes, Some(32_833_536));
    }

    #[test]
    fn parses_parent_row() {
        let raw = "28785 1 /Applications/Visual Studio Code.app/Contents/MacOS/Code\n";
        let row = parse_ps_parent_row(raw).unwrap();

        assert_eq!(row.0, 28785);
        assert_eq!(row.1, Some(1));
        assert_eq!(row.2, "Code");
    }

    #[test]
    fn parses_etime_formats() {
        assert_eq!(parse_etime("01:02"), Some(Duration::from_secs(62)));
        assert_eq!(parse_etime("01:02:03"), Some(Duration::from_secs(3723)));
        assert_eq!(
            parse_etime("2-01:02:03"),
            Some(Duration::from_secs(176_523))
        );
        assert_eq!(parse_etime(""), None);
    }
}
