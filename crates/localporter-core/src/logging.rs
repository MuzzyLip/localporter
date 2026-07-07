use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;

static LOGGER: OnceLock<FileLogger> = OnceLock::new();
static LOGGER_PATH: OnceLock<PathBuf> = OnceLock::new();

struct FileLogger {
    file: Mutex<File>,
}

impl FileLogger {
    fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        rotate_if_needed(path)?;

        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    fn write(&self, level: &str, target: &str, args: fmt::Arguments<'_>) {
        let mut message = args.to_string();
        if message.contains('\n') {
            message = message.replace('\n', "\\n");
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let seconds = now.as_secs();
        let millis = now.subsec_millis();
        let thread_id = format!("{:?}", std::thread::current().id());

        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(
                file,
                "{seconds}.{millis:03} [{level}] [{target}] [thread={thread_id}] {message}"
            );
            let _ = file.flush();
        }
    }
}

pub fn init_file_logger(path: PathBuf) -> io::Result<PathBuf> {
    if let Some(existing) = LOGGER_PATH.get() {
        return Ok(existing.clone());
    }

    let logger = FileLogger::open(&path)?;

    if LOGGER.set(logger).is_err() {
        if let Some(existing) = LOGGER_PATH.get() {
            return Ok(existing.clone());
        }
        return Err(io::Error::other("logger was initialized concurrently"));
    }

    let _ = LOGGER_PATH.set(path.clone());
    Ok(path)
}

pub fn log_message(level: &str, target: &str, args: fmt::Arguments<'_>) {
    if let Some(logger) = LOGGER.get() {
        logger.write(level, target, args);
    }
}

fn rotate_if_needed(path: &Path) -> io::Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };

    if metadata.len() < MAX_LOG_FILE_BYTES {
        return Ok(());
    }

    let Some(file_name) = path.file_name() else {
        return Ok(());
    };

    let rotated_path = path.with_file_name(format!("{}.1", file_name.to_string_lossy()));
    if rotated_path.exists() {
        let _ = fs::remove_file(&rotated_path);
    }
    fs::rename(path, rotated_path)
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::logging::log_message("DEBUG", module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::logging::log_message("INFO", module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::logging::log_message("WARN", module_path!(), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::logging::log_message("ERROR", module_path!(), format_args!($($arg)*))
    };
}
