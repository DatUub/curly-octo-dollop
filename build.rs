fn main() {
    // Only compile resources on Windows
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        // Set the application icon from icon.ico in the project root
        res.set_icon("icon.ico");
        res.compile()
            .expect("Failed to compile Windows resources - ensure icon.ico exists in the project root");
    }
}
