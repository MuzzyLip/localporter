use std::sync::Arc;

use crate::{
    SourceError,
    adapter::{
        macos::command::CommandRunner, windows::command::parser::parse_net_connection_ports,
    },
    domain::{PortQueryScope, ProcessPortBinding},
    sources::BoundPortSource,
};

const NETSTAT_ARGS: &[&str] = &["-ano"];

pub struct NetConnectionPortSource {
    runner: Arc<dyn CommandRunner>,
}

impl NetConnectionPortSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }

    fn run_query(&self, scope: PortQueryScope) -> Result<Vec<ProcessPortBinding>, SourceError> {
        let raw = self.runner.run("netstat", NETSTAT_ARGS)?;
        parse_net_connection_ports(&raw, scope)
    }
}

impl BoundPortSource for NetConnectionPortSource {
    fn collect_bound_ports(
        &self,
        scope: PortQueryScope,
    ) -> Result<Vec<ProcessPortBinding>, SourceError> {
        self.run_query(scope)
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
            Ok(concat!(
                "  Proto  Local Address          Foreign Address        State           PID\n",
                "  TCP    0.0.0.0:3000           0.0.0.0:0              LISTENING       1234\n",
                "  TCP    127.0.0.1:51824        127.0.0.1:3000         TIME_WAIT       2000\n",
                "  UDP    0.0.0.0:5353           *:*                                    4321\n"
            )
            .to_owned())
        }
    }

    #[test]
    fn uses_netstat_and_filters_to_listening_tcp_by_default_scope() {
        let runner = Arc::new(RecordingRunner::new());
        let source = NetConnectionPortSource::new(runner.clone());

        let bindings = source
            .collect_bound_ports(PortQueryScope::ListenOnly)
            .unwrap();

        let calls = runner.calls();
        assert_eq!(bindings.len(), 2);
        assert_eq!(calls[0].0, "netstat");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, vec!["-ano"]);
    }

    #[test]
    fn includes_time_wait_rows_when_show_all_scope_is_enabled() {
        let runner = Arc::new(RecordingRunner::new());
        let source = NetConnectionPortSource::new(runner.clone());

        let bindings = source.collect_bound_ports(PortQueryScope::AllTcp).unwrap();

        let calls = runner.calls();
        assert_eq!(bindings.len(), 3);
        assert_eq!(calls[0].0, "netstat");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, vec!["-ano"]);
    }
}
