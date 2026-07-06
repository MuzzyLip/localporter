use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    adapter::{
        macos::command::CommandRunner,
        windows::command::parser::{CimProcessRow, parse_cim_process_rows},
    },
    domain::ParentProcess,
    error::SourceError,
    sources::ParentChainSource,
};

const PROCESS_TABLE_QUERY: &str = "Get-CimInstance Win32_Process | ForEach-Object { \"$($_.ProcessId)|$($_.ParentProcessId)|$($_.Name)\" }";
const PROCESS_TABLE_CACHE_TTL: Duration = Duration::from_secs(2);

#[derive(Clone)]
struct ProcessTableCache {
    loaded_at: Instant,
    rows: Arc<HashMap<u32, CimProcessRow>>,
}

pub struct CimParentChainSource {
    runner: Arc<dyn CommandRunner>,
    cache: Mutex<Option<ProcessTableCache>>,
}

impl CimParentChainSource {
    pub fn new(runner: Arc<dyn CommandRunner>) -> Self {
        Self {
            runner,
            cache: Mutex::new(None),
        }
    }

    fn load_process_table(&self) -> Result<Arc<HashMap<u32, CimProcessRow>>, SourceError> {
        let args = [
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            PROCESS_TABLE_QUERY,
        ];
        let raw = self.runner.run("powershell", &args)?;
        let rows = parse_cim_process_rows(&raw)?;

        Ok(Arc::new(
            rows.into_iter()
                .map(|row @ (pid, _, _)| (pid, row))
                .collect::<HashMap<_, _>>(),
        ))
    }

    fn process_table(&self) -> Result<Arc<HashMap<u32, CimProcessRow>>, SourceError> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(existing) = cache.as_ref()
            && existing.loaded_at.elapsed() < PROCESS_TABLE_CACHE_TTL
        {
            return Ok(existing.rows.clone());
        }

        let rows = self.load_process_table()?;
        *cache = Some(ProcessTableCache {
            loaded_at: Instant::now(),
            rows: rows.clone(),
        });
        Ok(rows)
    }
}

impl ParentChainSource for CimParentChainSource {
    fn collect_parent_chain(
        &self,
        pid: u32,
        max_depth: usize,
    ) -> Result<Vec<ParentProcess>, SourceError> {
        let mut chain = Vec::new();
        let rows = self.process_table()?;
        let mut current_pid = pid;

        for _ in 0..max_depth {
            let Some((_, parent_pid, _)) = rows.get(&current_pid) else {
                break;
            };
            let Some(parent_pid) = parent_pid else {
                break;
            };

            if *parent_pid == current_pid {
                break;
            }

            let Some((resolved_pid, resolved_parent_pid, resolved_name)) = rows.get(parent_pid)
            else {
                break;
            };
            chain.push(ParentProcess {
                name: resolved_name.clone(),
                pid: *resolved_pid,
            });

            let Some(next_pid) = resolved_parent_pid else {
                break;
            };

            if *next_pid == 0 || *next_pid == *resolved_pid {
                break;
            }

            current_pid = *resolved_pid;
        }

        Ok(chain)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct RecordingRunner {
        calls: Mutex<Vec<(String, Vec<String>)>>,
        output: String,
    }

    impl RecordingRunner {
        fn new(output: &str) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                output: output.to_owned(),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl CommandRunner for RecordingRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, SourceError> {
            self.calls.lock().unwrap().push((
                program.to_owned(),
                args.iter().map(|arg| (*arg).to_owned()).collect(),
            ));
            Ok(self.output.clone())
        }
    }

    #[test]
    fn reuses_cached_process_table_across_parent_chain_requests() {
        let runner = Arc::new(RecordingRunner::new(
            "100|50|app.exe\n50|10|cmd.exe\n10|1|explorer.exe\n",
        ));
        let source = CimParentChainSource::new(runner.clone());

        let first = source.collect_parent_chain(100, 8).unwrap();
        let second = source.collect_parent_chain(50, 8).unwrap();

        assert_eq!(
            first,
            vec![
                ParentProcess {
                    pid: 50,
                    name: "cmd.exe".to_owned(),
                },
                ParentProcess {
                    pid: 10,
                    name: "explorer.exe".to_owned(),
                },
            ]
        );
        assert_eq!(
            second,
            vec![ParentProcess {
                pid: 10,
                name: "explorer.exe".to_owned(),
            }]
        );

        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "powershell");
        assert_eq!(calls[0].1[3], PROCESS_TABLE_QUERY);
    }
}
