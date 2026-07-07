use std::{env, fmt, fs, io, path::PathBuf, time::Duration};

#[cfg(target_os = "windows")]
use std::process::Command;

use localporter_core::{log_debug, log_error, log_info, log_warn};

const SETTINGS_FILE_NAME: &str = "settings.conf";
const SETTINGS_DIR_NAME: &str = "LocalPorter";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshInterval {
    OneSecond,
    TwoSeconds,
    ThreeSeconds,
    FiveSeconds,
}

impl RefreshInterval {
    pub const ALL: [Self; 4] = [
        Self::OneSecond,
        Self::TwoSeconds,
        Self::ThreeSeconds,
        Self::FiveSeconds,
    ];

    pub fn duration(self) -> Duration {
        Duration::from_secs(self.seconds())
    }

    pub fn seconds(self) -> u64 {
        match self {
            Self::OneSecond => 1,
            Self::TwoSeconds => 2,
            Self::ThreeSeconds => 3,
            Self::FiveSeconds => 5,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OneSecond => "1 second",
            Self::TwoSeconds => "2 seconds",
            Self::ThreeSeconds => "3 seconds",
            Self::FiveSeconds => "5 seconds",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "1" | "1s" => Some(Self::OneSecond),
            "2" | "2s" => Some(Self::TwoSeconds),
            "3" | "3s" => Some(Self::ThreeSeconds),
            "5" | "5s" => Some(Self::FiveSeconds),
            _ => None,
        }
    }
}

impl Default for RefreshInterval {
    fn default() -> Self {
        Self::TwoSeconds
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KillBehavior {
    Direct,
    Confirm,
}

impl KillBehavior {
    pub const ALL: [Self; 2] = [Self::Direct, Self::Confirm];

    pub fn label(self) -> &'static str {
        match self {
            Self::Direct => "Kill directly",
            Self::Confirm => "Confirm before kill",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Direct => "Single kill and Kill killable run immediately.",
            Self::Confirm => "Ask for confirmation before kill actions.",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "direct" => Some(Self::Direct),
            "confirm" => Some(Self::Confirm),
            _ => None,
        }
    }
}

impl Default for KillBehavior {
    fn default() -> Self {
        Self::Confirm
    }
}

impl fmt::Display for KillBehavior {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Direct => f.write_str("direct"),
            Self::Confirm => f.write_str("confirm"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppSettings {
    pub refresh_interval: RefreshInterval,
    pub kill_behavior: KillBehavior,
    pub launch_at_startup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            refresh_interval: RefreshInterval::default(),
            kill_behavior: KillBehavior::default(),
            launch_at_startup: false,
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let Some(path) = settings_file_path() else {
            log_warn!("settings path unavailable, using defaults");
            return Self::default();
        };

        let Ok(contents) = fs::read_to_string(path) else {
            log_debug!("settings file missing or unreadable, using defaults");
            return Self::default();
        };

        let mut settings = Self::default();

        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key.trim() {
                "refresh_interval" => {
                    if let Some(interval) = RefreshInterval::parse(value) {
                        settings.refresh_interval = interval;
                    }
                }
                "kill_behavior" => {
                    if let Some(behavior) = KillBehavior::parse(value) {
                        settings.kill_behavior = behavior;
                    }
                }
                "launch_at_startup" => {
                    settings.launch_at_startup = matches!(value.trim(), "true" | "1");
                }
                _ => {}
            }
        }

        log_debug!(
            "settings loaded: refresh_interval={}s kill_behavior={} launch_at_startup={}",
            settings.refresh_interval.seconds(),
            settings.kill_behavior,
            settings.launch_at_startup
        );
        settings
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = settings_file_path() else {
            log_error!("failed to save settings: settings path unavailable");
            return Err(io::Error::other("settings path unavailable"));
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = format!(
            "refresh_interval={}\nkill_behavior={}\nlaunch_at_startup={}\n",
            self.refresh_interval.seconds(),
            self.kill_behavior,
            self.launch_at_startup
        );
        fs::write(&path, contents)?;
        log_info!("settings saved: path={}", path.display());
        Ok(())
    }
}

pub fn launch_at_startup_supported() -> bool {
    cfg!(target_os = "windows") || cfg!(target_os = "macos")
}

pub fn read_launch_at_startup() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        read_windows_launch_at_startup()
    }

    #[cfg(target_os = "macos")]
    {
        read_macos_launch_at_startup()
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        Err("Launch at startup is not supported on this platform".to_owned())
    }
}

pub fn write_launch_at_startup(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        write_windows_launch_at_startup(enabled)
    }

    #[cfg(target_os = "macos")]
    {
        write_macos_launch_at_startup(enabled)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = enabled;
        Err("Launch at startup is not supported on this platform".to_owned())
    }
}

fn settings_file_path() -> Option<PathBuf> {
    settings_dir_path().map(|dir| dir.join(SETTINGS_FILE_NAME))
}

pub(crate) fn logs_dir_path() -> Option<PathBuf> {
    settings_dir_path().map(|dir| dir.join("logs"))
}

fn settings_dir_path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .map(|path| path.join(SETTINGS_DIR_NAME))
    }

    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME").map(PathBuf::from).map(|path| {
            path.join("Library/Application Support")
                .join(SETTINGS_DIR_NAME)
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|path| path.join(".config").join(SETTINGS_DIR_NAME.to_lowercase()))
    }
}

#[cfg(target_os = "windows")]
const WINDOWS_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(target_os = "windows")]
const WINDOWS_RUN_VALUE_NAME: &str = "LocalPorter";

#[cfg(target_os = "windows")]
fn read_windows_launch_at_startup() -> Result<bool, String> {
    let output = Command::new("reg")
        .args(["query", WINDOWS_RUN_KEY, "/v", WINDOWS_RUN_VALUE_NAME])
        .output()
        .map_err(|error| format!("failed to query startup registry: {error}"))?;

    Ok(output.status.success())
}

#[cfg(target_os = "windows")]
fn write_windows_launch_at_startup(enabled: bool) -> Result<(), String> {
    if enabled {
        let current_exe = env::current_exe()
            .map_err(|error| format!("failed to resolve current exe: {error}"))?;
        let command_value = format!("\"{}\"", current_exe.display());
        let output = Command::new("reg")
            .args([
                "add",
                WINDOWS_RUN_KEY,
                "/v",
                WINDOWS_RUN_VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &command_value,
                "/f",
            ])
            .output()
            .map_err(|error| format!("failed to update startup registry: {error}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(command_error_message(&output))
        }
    } else {
        if !read_windows_launch_at_startup()? {
            return Ok(());
        }

        let output = Command::new("reg")
            .args([
                "delete",
                WINDOWS_RUN_KEY,
                "/v",
                WINDOWS_RUN_VALUE_NAME,
                "/f",
            ])
            .output()
            .map_err(|error| format!("failed to update startup registry: {error}"))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(command_error_message(&output))
        }
    }
}

#[cfg(target_os = "macos")]
fn read_macos_launch_at_startup() -> Result<bool, String> {
    Ok(macos_launch_agent_path()?.exists())
}

#[cfg(target_os = "macos")]
fn write_macos_launch_at_startup(enabled: bool) -> Result<(), String> {
    let path = macos_launch_agent_path()?;

    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create LaunchAgents directory: {error}"))?;
        }

        let current_exe = env::current_exe()
            .map_err(|error| format!("failed to resolve current exe: {error}"))?;
        let contents = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.localporter.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
"#,
            current_exe.display()
        );

        fs::write(path, contents)
            .map_err(|error| format!("failed to write launch agent: {error}"))?;
        Ok(())
    } else if path.exists() {
        fs::remove_file(path).map_err(|error| format!("failed to remove launch agent: {error}"))
    } else {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn macos_launch_agent_path() -> Result<PathBuf, String> {
    let home = env::var_os("HOME").ok_or_else(|| "HOME is not set".to_owned())?;
    Ok(PathBuf::from(home).join("Library/LaunchAgents/com.localporter.app.plist"))
}

#[cfg(target_os = "windows")]
fn command_error_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if !stderr.is_empty() {
        return stderr;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !stdout.is_empty() {
        return stdout;
    }

    "command failed".to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_interval_parser_accepts_seconds() {
        assert_eq!(
            RefreshInterval::parse("1"),
            Some(RefreshInterval::OneSecond)
        );
        assert_eq!(
            RefreshInterval::parse("2s"),
            Some(RefreshInterval::TwoSeconds)
        );
        assert_eq!(RefreshInterval::parse("9"), None);
    }

    #[test]
    fn kill_behavior_parser_accepts_known_values() {
        assert_eq!(KillBehavior::parse("direct"), Some(KillBehavior::Direct));
        assert_eq!(KillBehavior::parse("confirm"), Some(KillBehavior::Confirm));
        assert_eq!(KillBehavior::parse("unknown"), None);
    }

    #[test]
    fn kill_behavior_defaults_to_confirm() {
        assert_eq!(KillBehavior::default(), KillBehavior::Confirm);
    }
}
