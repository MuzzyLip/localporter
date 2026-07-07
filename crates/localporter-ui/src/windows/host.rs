use eframe::{CreationContext, egui};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowMode {
    Standalone,
    MacOsMenuBarPanel,
}

pub struct WindowHost {
    #[cfg(target_os = "macos")]
    macos_menu_bar: Option<super::macos::MacOsMenuBarHost>,
}

impl WindowHost {
    pub fn new(cc: &CreationContext<'_>) -> Self {
        #[cfg(target_os = "macos")]
        {
            let macos_menu_bar = match super::macos::MacOsMenuBarHost::new(&cc.egui_ctx) {
                Ok(host) => Some(host),
                Err(error) => {
                    eprintln!("failed to initialize macOS menu bar host: {error}");
                    cc.egui_ctx
                        .send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    None
                }
            };

            return Self { macos_menu_bar };
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = cc;
            Self {}
        }
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        if let Some(host) = self.macos_menu_bar.as_mut() {
            host.update(ctx);
        }

        #[cfg(not(target_os = "macos"))]
        let _ = ctx;
    }

    pub fn mode(&self) -> WindowMode {
        #[cfg(target_os = "macos")]
        {
            if let Some(host) = self.macos_menu_bar.as_ref() {
                return match host.window_mode() {
                    super::macos::MacOsWindowMode::Standalone => WindowMode::Standalone,
                    super::macos::MacOsWindowMode::MenuBarPanel => WindowMode::MacOsMenuBarPanel,
                };
            }
        }

        WindowMode::Standalone
    }

    pub fn menu_bar_panel_arrow_tip_x(&self, ctx: &egui::Context) -> Option<f32> {
        #[cfg(target_os = "macos")]
        if let Some(host) = self.macos_menu_bar.as_ref() {
            return host.menu_bar_panel_arrow_tip_x(ctx);
        }

        let _ = ctx;
        None
    }

    pub fn request_quit(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "macos")]
        if let Some(host) = self.macos_menu_bar.as_mut() {
            host.request_quit(ctx);
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
