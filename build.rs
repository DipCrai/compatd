use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let so_name = "libcompatd_preload.so";
    let so_path = format!("{out_dir}/{so_name}");
    let status = Command::new("gcc")
        .args(&["-shared", "-fPIC", "-o", &so_path, "cbits/sd_notify.c", "-ldl"])
        .status()
        .expect("failed to run gcc");

    assert!(status.success(), "sd_notify.c compilation failed");

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let dest = Path::new(&manifest).join("target").join(&profile).join(so_name);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::copy(&so_path, &dest);
    println!("cargo:warning=compatd preload shim -> {}", dest.display());
}
