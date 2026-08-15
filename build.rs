fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let result = tauri_winres::WindowsResource::new()
            .set_icon("icons/icon.ico")
            .compile_for(&["meridian"]);
        if let Err(error) = result {
            panic!("failed to embed the Meridian icon in the Windows executable: {error}");
        }
    }
}
