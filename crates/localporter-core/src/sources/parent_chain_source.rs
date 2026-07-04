use crate::{SourceError, domain::ParentProcess};

pub trait ParentChainSource: Send + Sync {
    fn collect_parent_chain(
        &self,
        pid: u32,
        max_depth: usize,
    ) -> Result<Vec<ParentProcess>, SourceError>;
}
