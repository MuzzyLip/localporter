use std::sync::{Arc, Mutex};

use eframe::egui::{self, Pos2, ViewportCommand, pos2, vec2};
use localporter_core::{log_debug, log_info};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy, NSScreen};
use tray_icon::{
    MouseButton, MouseButtonState, Rect as TrayRect, TrayIcon, TrayIconBuilder, TrayIconEvent,
};

use crate::windows::constants::{APP_NAME, DEFAULT_HEIGHT, DEFAULT_WIDTH};

const PANEL_GAP: f32 = 6.0;
const PANEL_EDGE_PADDING: f32 = 8.0;
const TRAY_ICON_SIZE: u32 = 18;

pub struct MacOsMenuBarHost {
    _tray_icon: TrayIcon,
    state: Arc<Mutex<PanelState>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MacOsWindowMode {
    Standalone,
    MenuBarPanel,
}

#[derive(Clone, Copy)]
struct PanelMetrics {
    pixels_per_point: f32,
    panel_size: egui::Vec2,
    monitor_size: Option<egui::Vec2>,
}

impl Default for PanelMetrics {
    fn default() -> Self {
        Self {
            pixels_per_point: 1.0,
            panel_size: vec2(DEFAULT_WIDTH, DEFAULT_HEIGHT),
            monitor_size: None,
        }
    }
}

struct PanelState {
    last_tray_rect: Option<TrayRect>,
    panel_visible: bool,
    last_focused: Option<bool>,
    allow_app_quit: bool,
    window_mode: MacOsWindowMode,
    metrics: PanelMetrics,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            last_tray_rect: None,
            panel_visible: false,
            last_focused: None,
            allow_app_quit: false,
            window_mode: MacOsWindowMode::Standalone,
            metrics: PanelMetrics::default(),
        }
    }
}

impl MacOsMenuBarHost {
    pub fn new(ctx: &egui::Context) -> Result<Self, String> {
        let state = Arc::new(Mutex::new(PanelState::default()));
        let tray_state = Arc::clone(&state);
        let tray_ctx = ctx.clone();

        TrayIconEvent::set_event_handler(Some(move |event| {
            handle_tray_event(&tray_ctx, &tray_state, event);
        }));

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip(APP_NAME)
            .with_icon(build_template_tray_icon())
            .with_icon_as_template(true)
            .build()
            .map_err(|error| error.to_string())?;

        let host = Self {
            _tray_icon: tray_icon,
            state,
        };

        log_info!("macOS menu bar host initialized");
        host.show_standalone(ctx);

        Ok(host)
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        self.sync_metrics(ctx);
        self.handle_close_request(ctx);
        self.handle_focus_loss(ctx);
    }

    pub fn request_quit(&mut self, ctx: &egui::Context) {
        log_info!("quitting application from menu bar panel");
        {
            let mut state = self.state.lock().expect("panel state lock poisoned");
            state.allow_app_quit = true;
        }

        terminate_application();
        ctx.send_viewport_cmd(ViewportCommand::Close);
        ctx.request_repaint();
    }

    pub fn window_mode(&self) -> MacOsWindowMode {
        self.state
            .lock()
            .expect("panel state lock poisoned")
            .window_mode
    }

    pub fn menu_bar_panel_arrow_tip_x(&self, ctx: &egui::Context) -> Option<f32> {
        let (tray_rect, pixels_per_point, window_mode) = {
            let state = self.state.lock().expect("panel state lock poisoned");
            (
                state.last_tray_rect,
                state.metrics.pixels_per_point,
                state.window_mode,
            )
        };

        if window_mode != MacOsWindowMode::MenuBarPanel {
            return None;
        }

        let tray_rect = tray_rect?;
        let viewport_rect =
            ctx.input(|input| input.viewport().outer_rect.or(input.viewport().inner_rect))?;
        let tray_center_x =
            (tray_rect.position.x as f32 + tray_rect.size.width as f32 * 0.5) / pixels_per_point;

        Some(tray_center_x - viewport_rect.left())
    }

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|input| input.viewport().close_requested()) {
            let should_quit = {
                let mut state = self.state.lock().expect("panel state lock poisoned");
                let should_quit = state.allow_app_quit;
                if should_quit {
                    state.allow_app_quit = false;
                }
                should_quit
            };

            if should_quit {
                terminate_application();
                return;
            }

            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            self.hide_panel(ctx);
        }
    }

    fn handle_focus_loss(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|input| input.viewport().focused);
        let mut state = self.state.lock().expect("panel state lock poisoned");

        if state.panel_visible
            && state.window_mode == MacOsWindowMode::MenuBarPanel
            && matches!((state.last_focused, focused), (Some(true), Some(false)))
        {
            drop(state);
            self.hide_panel(ctx);
            return;
        }

        state.last_focused = focused;
    }

    fn sync_metrics(&mut self, ctx: &egui::Context) {
        let panel_size = ctx.input(|input| {
            input
                .viewport()
                .outer_rect
                .or(input.viewport().inner_rect)
                .map(|rect| rect.size())
                .unwrap_or(vec2(DEFAULT_WIDTH, DEFAULT_HEIGHT))
        });
        let monitor_size = ctx.input(|input| input.viewport().monitor_size);
        let mut state = self.state.lock().expect("panel state lock poisoned");
        state.metrics = PanelMetrics {
            pixels_per_point: ctx.pixels_per_point().max(1.0),
            panel_size,
            monitor_size,
        };
    }

    fn show_standalone(&self, ctx: &egui::Context) {
        set_regular_activation_policy();
        activate_application();
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(false));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();

        let mut state = self.state.lock().expect("panel state lock poisoned");
        state.panel_visible = true;
        state.last_focused = None;
        state.window_mode = MacOsWindowMode::Standalone;
        log_info!("window mode changed: standalone visible=true");
    }

    fn hide_panel(&self, ctx: &egui::Context) {
        set_accessory_activation_policy();
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        ctx.request_repaint();

        let mut state = self.state.lock().expect("panel state lock poisoned");
        state.panel_visible = false;
        state.last_focused = None;
    }
}

fn handle_tray_event(ctx: &egui::Context, state: &Arc<Mutex<PanelState>>, event: TrayIconEvent) {
    match event {
        TrayIconEvent::Click {
            rect,
            button,
            button_state,
            ..
        } => {
            {
                let mut state = state.lock().expect("panel state lock poisoned");
                state.last_tray_rect = Some(rect);
            }

            if button == MouseButton::Left && button_state == MouseButtonState::Up {
                log_debug!("tray icon clicked: left button up");
                toggle_panel(ctx, state, Some(rect));
            } else {
                ctx.request_repaint();
            }
        }
        TrayIconEvent::Enter { rect, .. } | TrayIconEvent::Move { rect, .. } => {
            let mut state = state.lock().expect("panel state lock poisoned");
            state.last_tray_rect = Some(rect);
        }
        TrayIconEvent::Leave { .. } | TrayIconEvent::DoubleClick { .. } => {}
        _ => {}
    }
}

fn toggle_panel(ctx: &egui::Context, state: &Arc<Mutex<PanelState>>, tray_rect: Option<TrayRect>) {
    let (visible, window_mode) = {
        let state = state.lock().expect("panel state lock poisoned");
        (state.panel_visible, state.window_mode)
    };

    if visible && window_mode == MacOsWindowMode::MenuBarPanel {
        log_info!("menu bar panel toggle: hide");
        hide_panel(ctx, state);
    } else {
        log_info!("menu bar panel toggle: show");
        show_panel(ctx, state, tray_rect);
    }
}

fn show_panel(ctx: &egui::Context, state: &Arc<Mutex<PanelState>>, tray_rect: Option<TrayRect>) {
    let panel_origin = {
        let mut state = state.lock().expect("panel state lock poisoned");
        state.window_mode = MacOsWindowMode::MenuBarPanel;
        panel_origin(tray_rect.or(state.last_tray_rect), state.metrics)
    };

    set_accessory_activation_policy();
    activate_application();
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(false));
    ctx.send_viewport_cmd(ViewportCommand::Decorations(false));
    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(panel_origin));
    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    ctx.request_repaint();

    let mut state = state.lock().expect("panel state lock poisoned");
    state.panel_visible = true;
    state.last_focused = None;
    log_info!("window mode changed: menu_bar_panel visible=true");
}

fn hide_panel(ctx: &egui::Context, state: &Arc<Mutex<PanelState>>) {
    ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    ctx.request_repaint();

    let mut state = state.lock().expect("panel state lock poisoned");
    state.panel_visible = false;
    state.last_focused = None;
    log_info!("menu bar panel hidden");
}

fn panel_origin(tray_rect: Option<TrayRect>, metrics: PanelMetrics) -> Pos2 {
    let Some(tray_rect) = tray_rect else {
        return default_panel_origin(metrics.panel_size);
    };

    let tray_left = tray_rect.position.x as f32 / metrics.pixels_per_point;
    let tray_top = tray_rect.position.y as f32 / metrics.pixels_per_point;
    let tray_width = tray_rect.size.width as f32 / metrics.pixels_per_point;
    let tray_height = tray_rect.size.height as f32 / metrics.pixels_per_point;

    let anchor_x = tray_left + tray_width * 0.5;
    let mut x = anchor_x - metrics.panel_size.x * 0.5;
    let mut y = tray_top + tray_height + PANEL_GAP;

    if let Some(size) = metrics.monitor_size {
        x = x.clamp(
            PANEL_EDGE_PADDING,
            (size.x - metrics.panel_size.x - PANEL_EDGE_PADDING).max(PANEL_EDGE_PADDING),
        );
        y = y.min((size.y - metrics.panel_size.y - PANEL_EDGE_PADDING).max(PANEL_EDGE_PADDING));
    }

    pos2(x.max(PANEL_EDGE_PADDING), y.max(PANEL_EDGE_PADDING))
}

fn default_panel_origin(panel_size: egui::Vec2) -> Pos2 {
    let Some(mtm) = MainThreadMarker::new() else {
        return pos2(PANEL_EDGE_PADDING, PANEL_EDGE_PADDING);
    };

    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return pos2(PANEL_EDGE_PADDING, PANEL_EDGE_PADDING);
    };

    let visible_frame = screen.visibleFrame();
    let x = (visible_frame.origin.x as f32 + visible_frame.size.width as f32
        - panel_size.x
        - PANEL_EDGE_PADDING)
        .max(PANEL_EDGE_PADDING);
    let y = (visible_frame.origin.y as f32 + visible_frame.size.height as f32
        - panel_size.y
        - PANEL_EDGE_PADDING)
        .max(PANEL_EDGE_PADDING);

    pos2(x, y)
}

fn build_template_tray_icon() -> tray_icon::Icon {
    let mut rgba = vec![0_u8; (TRAY_ICON_SIZE * TRAY_ICON_SIZE * 4) as usize];

    for y in 0..TRAY_ICON_SIZE {
        for x in 0..TRAY_ICON_SIZE {
            if pixel_alpha(x, y) == 0 {
                continue;
            }

            let index = ((y * TRAY_ICON_SIZE + x) * 4) as usize;
            rgba[index] = 0;
            rgba[index + 1] = 0;
            rgba[index + 2] = 0;
            rgba[index + 3] = 255;
        }
    }

    tray_icon::Icon::from_rgba(rgba, TRAY_ICON_SIZE, TRAY_ICON_SIZE)
        .expect("tray icon pixels should be valid")
}

fn pixel_alpha(x: u32, y: u32) -> u8 {
    let left_bar = (x >= 4 && x <= 6) && (y >= 3 && y <= 14);
    let bottom_bar = (x >= 4 && x <= 13) && (y >= 12 && y <= 14);
    let right_bar = (x >= 11 && x <= 13) && (y >= 6 && y <= 14);
    let port_hole = (x >= 8 && x <= 9) && (y >= 6 && y <= 8);

    if (left_bar || bottom_bar || right_bar) && !port_hole {
        255
    } else {
        0
    }
}

fn set_accessory_activation_policy() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
}

fn set_regular_activation_policy() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let _ = app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
}

fn activate_application() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    app.activate();
}

fn terminate_application() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    app.terminate(None);
}
