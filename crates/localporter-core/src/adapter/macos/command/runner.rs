use crate::SourceError;
use std::{io::ErrorKind, process::Command};

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError>;
}

pub struct StdCommandRunner;

impl CommandRunner for StdCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError> {
        let output =
            Command::new(program)
                .args(args)
                .output()
                .map_err(|error| match error.kind() {
                    ErrorKind::NotFound => SourceError::CommandNotFound {
                        program: program.to_owned(),
                    },
                    ErrorKind::PermissionDenied => SourceError::PermissionDenied {
                        program: program.to_owned(),
                    },
                    _ => SourceError::CommandFailed {
                        program: program.to_owned(),
                        stderr: error.to_string(),
                    },
                })?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if is_permission_denied(&stderr) {
            return Err(SourceError::PermissionDenied {
                program: program.to_owned(),
            });
        }

        Err(SourceError::CommandFailed {
            program: program.to_owned(),
            stderr: if stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                stderr
            },
        })
    }
}

fn is_permission_denied(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("permission denied")
        || stderr.contains("operation not permitted")
        || stderr.contains("not permitted")
}
