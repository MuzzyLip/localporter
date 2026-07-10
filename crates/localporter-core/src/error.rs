#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    CommandNotFound { program: String },
    CommandFailed { program: String, stderr: String },
    CommandTimedOut { program: String },
    PermissionDenied { program: String },
    InvalidOutput { source: &'static str },
}
