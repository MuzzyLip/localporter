use std::sync::Arc;

use crate::{
    SourceError,
    adapter::macos::command::{parser::parse_lsof_ports, runner::CommandRunner},
    domain::{PortProtocol, PortQueryScope, ProcessPortBinding},
    sources::BoundPortSource,
};

pub struct LsofPortSource {
    runner: Arc<dyn CommandRunner>,
}

impl LsofPortSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl BoundPortSource for LsofPortSource {
    fn collect_bound_ports(
        &self,
        scope: PortQueryScope,
    ) -> Result<Vec<ProcessPortBinding>, SourceError> {
        let tcp_args: &[&str] = match scope {
            PortQueryScope::ListenOnly => &["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"],
            PortQueryScope::AllTcp => &["-nP", "-iTCP", "-Fpcn"],
        };
        let tcp_raw = self.runner.run("lsof", tcp_args)?;
        let udp_raw = self.runner.run("lsof", &["-nP", "-iUDP", "-Fpcn"])?;

        let mut bindings = parse_lsof_ports(&tcp_raw, PortProtocol::Tcp)?;
        bindings.extend(parse_lsof_ports(&udp_raw, PortProtocol::Udp)?);

        Ok(bindings)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct RecordingRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl RecordingRunner {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError> {
            self.calls.lock().unwrap().push((
                program.to_owned(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            Ok(String::new())
        }
    }

    #[test]
    fn uses_listen_only_tcp_query_by_default_scope() {
        let runner = Arc::new(RecordingRunner::new());
        let source = LsofPortSource::new(runner.clone());

        let _ = source
            .collect_bound_ports(PortQueryScope::ListenOnly)
            .unwrap();

        let calls = runner.calls();
        assert_eq!(calls[0].0, "lsof");
        assert_eq!(calls[0].1, vec!["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"]);
        assert_eq!(calls[1].1, vec!["-nP", "-iUDP", "-Fpcn"]);
    }

    #[test]
    fn uses_all_tcp_query_when_show_all_scope_is_enabled() {
        let runner = Arc::new(RecordingRunner::new());
        let source = LsofPortSource::new(runner.clone());

        let _ = source.collect_bound_ports(PortQueryScope::AllTcp).unwrap();

        let calls = runner.calls();
        assert_eq!(calls[0].0, "lsof");
        assert_eq!(calls[0].1, vec!["-nP", "-iTCP", "-Fpcn"]);
        assert_eq!(calls[1].1, vec!["-nP", "-iUDP", "-Fpcn"]);
    }
}
