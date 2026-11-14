fn main() {
    let mut attrs = tauri_build::Attributes::new();
    // Windows' resource compiler can't read paths containing an apostrophe
    // (e.g. a folder named "coach's playbook"), so hand it a copy of the icon.
    let icon = std::path::Path::new("icons").join("icon.ico");
    let abs = std::fs::canonicalize(&icon).unwrap_or(icon.clone());
    if abs.to_string_lossy().contains('\'') {
        let copy = std::env::temp_dir().join(format!("cpb-{}-icon.ico", env!("CARGO_PKG_NAME")));
        if std::fs::copy(&icon, &copy).is_ok() {
            attrs = attrs.windows_attributes(tauri_build::WindowsAttributes::new().window_icon_path(copy));
        }
    }
    tauri_build::try_build(attrs).expect("failed to run tauri-build");
}
