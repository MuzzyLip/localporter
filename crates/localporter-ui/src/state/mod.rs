mod app_state;
mod settings;

pub use app_state::{AppState, ToastLevel, ToastView};
pub(crate) use settings::logs_dir_path;
pub use settings::{AppSettings, KillBehavior, RefreshInterval};
