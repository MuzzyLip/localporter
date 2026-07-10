use crate::SourceError;
use std::{
    io::{ErrorKind, Read},
    process::{Command, Stdio},
    thread,
    time::Duration,
};
use wait_timeout::ChildExt;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::{log_error, log_warn};

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);

pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError>;
}

pub struct StdCommandRunner {
    timeout: Duration,
}

impl StdCommandRunner {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

impl Default for StdCommandRunner {
    fn default() -> Self {
        Self::with_timeout(DEFAULT_COMMAND_TIMEOUT)
    }
}

impl CommandRunner for StdCommandRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError> {
        let mut command = background_command(program);
        command
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().map_err(|error| match error.kind() {
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
                log_error!("command spawn failed: program={program} args={args:?} error={error}");
                SourceError::CommandFailed {
                    program: program.to_owned(),
                    stderr: error.to_string(),
                }
            }
        })?;

        let stdout = child
            .stdout
            .take()
            .expect("piped stdout should be available");
        let stderr = child
            .stderr
            .take()
            .expect("piped stderr should be available");
        let stdout_task = thread::spawn(move || read_all(stdout));
        let stderr_task = thread::spawn(move || read_all(stderr));

        let status = match child.wait_timeout(self.timeout) {
            Ok(Some(status)) => status,
            Ok(None) => {
                log_warn!(
                    "command timed out: program={program} args={args:?} timeout_ms={}",
                    self.timeout.as_millis()
                );
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_task.join();
                let _ = stderr_task.join();
                return Err(SourceError::CommandTimedOut {
                    program: program.to_owned(),
                });
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_task.join();
                let _ = stderr_task.join();
                return Err(SourceError::CommandFailed {
                    program: program.to_owned(),
                    stderr: error.to_string(),
                });
            }
        };

        let stdout = stdout_task.join().unwrap_or_default();
        let stderr_bytes = stderr_task.join().unwrap_or_default();

        if status.success() {
            return Ok(String::from_utf8_lossy(&stdout).into_owned());
        }

        let stderr = String::from_utf8_lossy(&stderr_bytes).trim().to_owned();
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
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "signal".to_owned()),
            if stderr.is_empty() {
                String::from_utf8_lossy(&stdout).trim().to_owned()
            } else {
                stderr.clone()
            }
        );

        Err(SourceError::CommandFailed {
            program: program.to_owned(),
            stderr: if stderr.is_empty() {
                String::from_utf8_lossy(&stdout).trim().to_owned()
            } else {
                stderr
            },
        })
    }
}

fn read_all(mut reader: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = reader.read_to_end(&mut bytes);
    bytes
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    #[test]
    fn command_runner_terminates_commands_after_timeout() {
        let runner = StdCommandRunner::with_timeout(Duration::from_millis(100));

        #[cfg(target_os = "windows")]
        let result = runner.run(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 2"],
        );
        #[cfg(target_os = "macos")]
        let result = runner.run("sh", &["-c", "sleep 2"]);

        assert!(matches!(result, Err(SourceError::CommandTimedOut { .. })));
    }
}
