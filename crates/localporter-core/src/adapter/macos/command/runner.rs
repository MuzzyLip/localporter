use crate::SourceError;
use std::{io::ErrorKind, process::Command};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::{log_error, log_warn};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError>;
}

pub struct StdCommandRunner;

impl CommandRunner for StdCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError> {
        let output = background_command(program)
            .args(args)
            .output()
            .map_err(|error| match error.kind() {
                ErrorKind::NotFound => {
                    log_error!("command spawn failed: program={program} args={args:?} kind=not_found");
                    SourceError::CommandNotFound {
                        program: program.to_owned(),
                    }
                }
                ErrorKind::PermissionDenied => {
                    log_warn!(
                        "command spawn denied: program={program} args={args:?} kind=permission_denied"
                    );
                    SourceError::PermissionDenied {
                        program: program.to_owned(),
                    }
                }
                _ => {
                    log_error!(
                        "command spawn failed: program={program} args={args:?} error={error}"
                    );
                    SourceError::CommandFailed {
                        program: program.to_owned(),
                        stderr: error.to_string(),
                    }
                }
            })?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if is_permission_denied(&stderr) {
            log_warn!(
                "command denied: program={program} args={args:?} stderr={}",
                stderr
            );
            return Err(SourceError::PermissionDenied {
                program: program.to_owned(),
            });
        }

        log_warn!(
            "command failed: program={program} args={args:?} status={} stderr={}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_owned()),
            if stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).trim().to_owned()
            } else {
                stderr.clone()
            }
        );

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

fn background_command(program: &str) -> Command {
    #[cfg(target_os = "windows")]
    let mut command = Command::new(program);

    #[cfg(not(target_os = "windows"))]
    let command = Command::new(program);

    #[cfg(target_os = "windows")]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

fn is_permission_denied(stderr: &str) -> bool {
    let stderr = stderr.to_ascii_lowercase();
    stderr.contains("permission denied")
        || stderr.contains("operation not permitted")
        || stderr.contains("not permitted")
}
