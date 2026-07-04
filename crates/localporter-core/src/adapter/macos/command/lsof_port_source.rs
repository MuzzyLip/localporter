use std::sync::Arc;

use crate::{
    SourceError,
    adapter::macos::command::{parser::parse_lsof_ports, runner::CommandRunner},
    domain::PortProtocol,
    domain::ProcessPortBinding,
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
    fn collect_bound_ports(&self) -> Result<Vec<ProcessPortBinding>, SourceError> {
        // TCP LISTEN sockets are the primary server ports we want to surface.
        let tcp_raw = self
            .runner
            .run("lsof", &["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])?;
        let udp_raw = self.runner.run("lsof", &["-nP", "-iUDP", "-Fpcn"])?;

        let mut bindings = parse_lsof_ports(&tcp_raw, PortProtocol::Tcp)?;
        bindings.extend(parse_lsof_ports(&udp_raw, PortProtocol::Udp)?);

        Ok(bindings)
    }
}
