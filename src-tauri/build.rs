fn main() {
    // Ensure macOS deployment target is set for std::filesystem support (whisper.cpp)
    if std::env::var("MACOSX_DEPLOYMENT_TARGET").is_err() {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=10.15");
    }
    tauri_build::build()
}
