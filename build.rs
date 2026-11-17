use std::{env, fs, path::PathBuf};
fn main() {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // Cross-platform cargo check needs no PE resource linker. Native Windows
        // and the GNU cross-release both embed the supplied application icon.
        let host = env::var("HOST").unwrap_or_default();
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if host.contains("windows") || target_env == "gnu" {
            let mut resource = winresource::WindowsResource::new();
            resource
                .set_icon("assets/app-icon.ico")
                .set("ProductName", "NothingX Desktop")
                .set("ProductVersion", env!("CARGO_PKG_VERSION"))
                .set("FileVersion", env!("CARGO_PKG_VERSION"))
                .set(
                    "FileDescription",
                    "NothingX Desktop — Nothing and CMF controls",
                )
                .set("OriginalFilename", "nothingx-desktop.exe");
            if !host.contains("windows") {
                resource
                    .set_toolkit_path(".")
                    .set_windres_path("x86_64-w64-mingw32-windres");
            }
            resource.compile().expect("Windows icon resource");
        }
    }
    println!("cargo:rerun-if-changed=ui");
    println!("cargo:rerun-if-changed=assets/app-icon.ico");
    println!("cargo:rerun-if-changed=Cargo.toml");
    let mut files = Vec::new();
    fn visit(path: PathBuf, files: &mut Vec<PathBuf>) {
        for item in fs::read_dir(path).unwrap() {
            let p = item.unwrap().path();
            if p.is_dir() {
                visit(p, files)
            } else {
                files.push(p)
            }
        }
    }
    visit(PathBuf::from("ui"), &mut files);
    files.sort();
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut code =
        String::from("pub fn asset(path:&str)->Option<(&'static str,&'static [u8])>{match path{\n");
    for path in files {
        let key = path
            .strip_prefix("ui")
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let mime = match path.extension().and_then(|s| s.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") => "application/javascript",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            _ => "application/octet-stream",
        };
        code += &format!(
            "{key:?}=>Some(({mime:?},include_bytes!({:?}))),\n",
            root.join(path)
        );
    }
    code += "_=>None}}";
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("ui_assets.rs"),
        code,
    )
    .unwrap();
}
