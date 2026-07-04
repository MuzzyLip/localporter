#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParentProcess {
    pub name: String,
    pub pid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcessOrigin {
    pub resolved_name: String,
    pub immediate_parent: Option<ParentProcess>,
    pub parent_chain: Vec<ParentProcess>,
}

impl ProcessOrigin {
    pub fn resolved_name_or_unknown(&self) -> &str {
        if self.resolved_name.is_empty() {
            "Unknown"
        } else {
            &self.resolved_name
        }
    }
}
