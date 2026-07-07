fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("../localporter-ui/assets/app-icon.ico");
        resource
            .compile()
            .expect("failed to compile Windows application icon resource");
    }
}
