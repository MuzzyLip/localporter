use std::sync::Arc;

use crate::{
    adapter::macos::command::{parser::parse_ps_parent_row, runner::CommandRunner},
    domain::ParentProcess,
    error::SourceError,
    sources::ParentChainSource,
};

pub struct PsParentChainSource {
    runner: Arc<dyn CommandRunner>,
}

impl PsParentChainSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self { runner }
    }
}

impl ParentChainSource for PsParentChainSource {
    fn collect_parent_chain(
        &self,
        pid: u32,
        max_depth: usize,
    ) -> Result<Vec<ParentProcess>, SourceError> {
        // 逐层 ps -p <pid> -o pid= -o ppid= -o comm= -ww
        let mut chain = Vec::new();
        let mut current_pid = pid;

        for _ in 0..max_depth {
            let pid_arg = current_pid.to_string();
            let raw = self.runner.run(
                "ps",
                &[
                    "-p", &pid_arg, "-o", "pid=", "-o", "ppid=", "-o", "command=", "-ww",
                ],
            )?;

            let (_, parent_pid, _parent_name) = parse_ps_parent_row(&raw)?;
            let Some(parent_pid) = parent_pid else {
                break;
            };

            if parent_pid == 0 || parent_pid == 1 {
                if parent_pid == 1 {
                    let parent_pid_arg = parent_pid.to_string();
                    if let Ok(raw) = self.runner.run(
                        "ps",
                        &[
                            "-p",
                            &parent_pid_arg,
                            "-o",
                            "pid=",
                            "-o",
                            "ppid=",
                            "-o",
                            "command=",
                            "-ww",
                        ],
                    ) {
                        if let Ok((resolved_pid, _, resolved_name)) = parse_ps_parent_row(&raw) {
                            chain.push(ParentProcess {
                                name: resolved_name,
                                pid: resolved_pid,
                            });
                        }
                    }
                }
                break;
            }

            let parent_pid_arg = parent_pid.to_string();
            let raw = self.runner.run(
                "ps",
                &[
                    "-p",
                    &parent_pid_arg,
                    "-o",
                    "pid=",
                    "-o",
                    "ppid=",
                    "-o",
                    "command=",
                    "-ww",
                ],
            )?;

            let (resolved_pid, resolved_parent_pid, resolved_name) = parse_ps_parent_row(&raw)?;
            chain.push(ParentProcess {
                name: resolved_name,
                pid: resolved_pid,
            });

            let Some(next_pid) = resolved_parent_pid else {
                break;
            };

            if next_pid == 0 || next_pid == resolved_pid {
                break;
            }

            current_pid = resolved_pid;
        }

        Ok(chain)
    }
}
