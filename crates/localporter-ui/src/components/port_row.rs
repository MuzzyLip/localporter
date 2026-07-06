use std::time::Duration;

use eframe::egui::{self, Align, Color32, Layout, Rect, RichText, Sense, UiBuilder};
use localporter_core::{BoundPort, PortProtocol, ProcessSummary};

#[derive(Default)]
pub struct PortRow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LauncherCategory {
    Browser,
    Terminal,
    Editor,
    Desktop,
    Runtime,
    System,
    Unknown,
    Generic,
}

impl LauncherCategory {
    fn detect(launcher: &str) -> Self {
        let normalized = launcher.trim().to_ascii_lowercase();

        if normalized.is_empty() || normalized == "unknown" {
            return Self::Unknown;
        }

        if Self::matches_any(
            &normalized,
            &[
                "chrome", "msedge", "edge", "firefox", "safari", "arc", "brave", "opera", "vivaldi",
            ],
        ) {
            Self::Browser
        } else if Self::matches_any(
            &normalized,
            &[
                "terminal",
                "powershell",
                "pwsh",
                "cmd",
                "wezterm",
                "iterm",
                "kitty",
                "alacritty",
                "hyper",
                "warp",
            ],
        ) {
            Self::Terminal
        } else if Self::matches_any(
            &normalized,
            &[
                "code",
                "cursor",
                "windsurf",
                "zed",
                "sublime",
                "notepad++",
                "webstorm",
                "rider",
                "idea",
                "clion",
                "pycharm",
                "goland",
                "rubymine",
                "devenv",
                "studio",
            ],
        ) {
            Self::Editor
        } else if Self::matches_any(
            &normalized,
            &["explorer", "finder", "dock", "loginwindow", "desktop"],
        ) {
            Self::Desktop
        } else if Self::matches_runtime(&normalized) {
            Self::Runtime
        } else if Self::matches_any(
            &normalized,
            &[
                "system", "launchd", "services", "svchost", "init", "systemd",
            ],
        ) {
            Self::System
        } else {
            Self::Generic
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Browser => "Browser",
            Self::Terminal => "Terminal",
            Self::Editor => "Editor",
            Self::Desktop => "Desktop",
            Self::Runtime => "Runtime",
            Self::System => "System",
            Self::Unknown => "Unknown",
            Self::Generic => "App",
        }
    }

    fn icon_source(self) -> egui::ImageSource<'static> {
        match self {
            Self::Browser => {
                egui::include_image!("../../assets/icons/port-row/launcher-browser.svg")
            }
            Self::Terminal => {
                egui::include_image!("../../assets/icons/port-row/launcher-terminal.svg")
            }
            Self::Editor => egui::include_image!("../../assets/icons/port-row/launcher-editor.svg"),
            Self::Desktop => {
                egui::include_image!("../../assets/icons/port-row/launcher-desktop.svg")
            }
            Self::Runtime => {
                egui::include_image!("../../assets/icons/port-row/launcher-runtime.svg")
            }
            Self::System => egui::include_image!("../../assets/icons/port-row/launcher-system.svg"),
            Self::Unknown => {
                egui::include_image!("../../assets/icons/port-row/launcher-unknown.svg")
            }
            Self::Generic => {
                egui::include_image!("../../assets/icons/port-row/launcher-generic.svg")
            }
        }
    }

    fn matches_any(value: &str, patterns: &[&str]) -> bool {
        patterns.iter().any(|pattern| value.contains(pattern))
    }

    fn matches_runtime(value: &str) -> bool {
        let executable = Self::executable_name(value);

        Self::matches_any(
            executable,
            &[
                "node", "nodejs", "python", "py", "java", "javac", "javaw", "cargo", "rustc",
                "deno", "bun", "ruby", "rubyw", "php", "perl", "dotnet", "mono", "scala",
                "kotlinc",
            ],
        ) || executable == "go"
    }

    fn executable_name(value: &str) -> &str {
        value
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or(value)
            .trim_end_matches(".exe")
    }
}

impl PortRow {
    const PORT_COLUMN_WIDTH: f32 = 80.0;
    const ROW_MIN_HEIGHT: f32 = 50.0;
    const PORT_LABEL_FONT_SIZE: f32 = 13.0;
    const PORT_CENTER_SPACING: f32 = 12.0;
    const TITLE_HEIGHT: f32 = 22.0;
    const META_HEIGHT: f32 = 18.0;
    const META_ICON_SIZE: f32 = 12.0;
    const META_TEXT_SIZE: f32 = 13.0;
    const META_ICON_SPACING: f32 = 4.0;

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        process: &ProcessSummary,
        port: Option<BoundPort>,
        uptime_offset: Duration,
    ) {
        ui.set_min_height(Self::ROW_MIN_HEIGHT);

        let row_rect = ui.max_rect();
        let (_, response) = ui.allocate_exact_size(
            egui::vec2(row_rect.width(), Self::ROW_MIN_HEIGHT),
            Sense::hover(),
        );
        let row_rect = response.rect;

        let port_rect = Rect::from_min_size(
            row_rect.min,
            egui::vec2(Self::PORT_COLUMN_WIDTH, Self::ROW_MIN_HEIGHT),
        );
        let center_rect = Rect::from_min_max(
            egui::pos2(
                port_rect.right() + Self::PORT_CENTER_SPACING,
                row_rect.top(),
            ),
            row_rect.right_bottom(),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(port_rect)
                .layout(Layout::centered_and_justified(egui::Direction::TopDown)),
            |ui| self.port_column(ui, port),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(center_rect)
                .layout(Layout::top_down(Align::Min)),
            |ui| self.center_column(ui, process, uptime_offset),
        );
    }

    fn port_column(&self, ui: &mut egui::Ui, port: Option<BoundPort>) {
        self.port_label(ui, port);
    }

    fn center_column(&self, ui: &mut egui::Ui, process: &ProcessSummary, uptime_offset: Duration) {
        let rect = ui.max_rect();
        let title_rect =
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), Self::TITLE_HEIGHT));
        let meta_top = (rect.bottom() - Self::META_HEIGHT).max(title_rect.bottom());
        let meta_rect = Rect::from_min_max(
            egui::pos2(rect.left(), meta_top),
            egui::pos2(rect.right(), rect.bottom()),
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(title_rect)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.label(
                    RichText::new(process.name_or_unknown())
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(32, 37, 43)),
                );
            },
        );

        ui.scope_builder(
            UiBuilder::new()
                .max_rect(meta_rect)
                .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

                self.launcher_meta(ui, &process.launcher);
                self.meta_icon_text(
                    ui,
                    Self::uptime_icon_source(),
                    Self::format_uptime(process.uptime.saturating_add(uptime_offset)),
                );
                self.meta_icon_text(
                    ui,
                    Self::cpu_icon_source(),
                    format!("{:.1}%", process.cpu_percent),
                );
                self.meta_icon_text(
                    ui,
                    Self::memory_icon_source(),
                    Self::format_memory(process.memory_usage),
                );
            },
        );
    }

    fn meta_text(&self, ui: &mut egui::Ui, text: String) {
        ui.label(
            RichText::new(text)
                .size(Self::META_TEXT_SIZE)
                .color(Color32::from_rgb(112, 118, 126)),
        );
    }

    fn meta_icon_text(&self, ui: &mut egui::Ui, icon: egui::ImageSource<'static>, text: String) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = Self::META_ICON_SPACING;

            ui.add(
                egui::Image::new(icon)
                    .fit_to_exact_size(egui::vec2(Self::META_ICON_SIZE, Self::META_ICON_SIZE))
                    .tint(Color32::from_rgb(112, 118, 126)),
            );
            self.meta_text(ui, text);
        });
    }

    fn launcher_meta(&self, ui: &mut egui::Ui, launcher: &str) {
        let category = LauncherCategory::detect(launcher);
        let response = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = Self::META_ICON_SPACING;

                ui.add(
                    egui::Image::new(category.icon_source())
                        .fit_to_exact_size(egui::vec2(Self::META_ICON_SIZE, Self::META_ICON_SIZE))
                        .tint(Color32::from_rgb(112, 118, 126)),
                );
                self.meta_text(ui, Self::value_or_unknown(launcher).to_owned());
            })
            .response;

        response.on_hover_text(format!("Launcher category: {}", category.label()));
    }

    fn port_label(&self, ui: &mut egui::Ui, port: Option<BoundPort>) {
        let text = match port {
            Some(port) => format!(":{}", port.port),
            None => "Unknown".to_owned(),
        };
        let color = self.port_color(port.map(|value| value.protocol));

        ui.label(
            RichText::new(text)
                .monospace()
                .size(Self::PORT_LABEL_FONT_SIZE)
                .color(color),
        );
    }

    fn port_color(&self, protocol: Option<PortProtocol>) -> Color32 {
        match protocol {
            Some(PortProtocol::Tcp) => Color32::from_rgb(120, 170, 255),
            Some(PortProtocol::Udp) => Color32::from_rgb(104, 200, 156),
            None => Color32::from_rgb(148, 154, 163),
        }
    }

    fn value_or_unknown(value: &str) -> &str {
        if value.trim().is_empty() {
            "Unknown"
        } else {
            value
        }
    }

    fn format_uptime(uptime: Duration) -> String {
        let total_secs = uptime.as_secs();
        let hours = total_secs / 3600;
        let minutes = (total_secs % 3600) / 60;
        let seconds = total_secs % 60;

        if hours > 0 {
            format!("{hours}h {minutes}m {seconds}s")
        } else if minutes > 0 {
            format!("{minutes}m {seconds}s")
        } else {
            format!("{seconds}s")
        }
    }

    fn format_memory(memory_bytes: u64) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;
        const GB: f64 = MB * 1024.0;

        let bytes = memory_bytes as f64;

        if bytes >= GB {
            format!("{:.1} GB", bytes / GB)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes / MB)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes / KB)
        } else {
            format!("{memory_bytes} B")
        }
    }

    fn uptime_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/port-row/uptime.svg")
    }

    fn cpu_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/port-row/cpu-usage.svg")
    }

    fn memory_icon_source() -> egui::ImageSource<'static> {
        egui::include_image!("../../assets/icons/port-row/memory-usage.svg")
    }
}

#[cfg(test)]
mod tests {
    use super::LauncherCategory;

    #[test]
    fn detects_browser_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("chrome.exe"),
            LauncherCategory::Browser
        );
    }

    #[test]
    fn detects_terminal_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("WindowsTerminal.exe"),
            LauncherCategory::Terminal
        );
    }

    #[test]
    fn detects_editor_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("Code.exe"),
            LauncherCategory::Editor
        );
    }

    #[test]
    fn detects_desktop_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("explorer.exe"),
            LauncherCategory::Desktop
        );
    }

    #[test]
    fn detects_system_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("systemd"),
            LauncherCategory::System
        );
    }

    #[test]
    fn detects_runtime_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("node.exe"),
            LauncherCategory::Runtime
        );
        assert_eq!(
            LauncherCategory::detect("C:\\Python311\\python.exe"),
            LauncherCategory::Runtime
        );
        assert_eq!(
            LauncherCategory::detect("cargo.exe"),
            LauncherCategory::Runtime
        );
    }

    #[test]
    fn detects_unknown_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("unknown"),
            LauncherCategory::Unknown
        );
    }

    #[test]
    fn falls_back_to_generic_launcher_category() {
        assert_eq!(
            LauncherCategory::detect("custom-launcher.exe"),
            LauncherCategory::Generic
        );
    }
}
