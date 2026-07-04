use crate::{SourceError, domain::ProcessPortBinding};

pub trait BoundPortSource: Send + Sync {
    fn collect_bound_ports(&self) -> Result<Vec<ProcessPortBinding>, SourceError>;
}
