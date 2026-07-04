use localporter_ui::{APP_NAME, StandaloneApp, standalone_native_options};

fn main() -> Result<(), eframe::Error> {
    let options = standalone_native_options();

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(StandaloneApp::new(cc)))),
    )
}
