use crate::{
    SourceError,
    domain::{PortQueryScope, ProcessPortBinding},
};

pub trait BoundPortSource: Send + Sync {
    fn collect_bound_ports(
        &self,
        scope: PortQueryScope,
    ) -> Result<Vec<ProcessPortBinding>, SourceError>;
}
