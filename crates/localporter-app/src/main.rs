use localporter_ui::{APP_NAME, StandaloneApp, init_logging, standalone_native_options};

fn main() -> Result<(), eframe::Error> {
    if let Err(error) = init_logging() {
        eprintln!("failed to initialize logging: {error}");
    }

    let options = standalone_native_options();

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(StandaloneApp::new(cc)))),
    )
}
