//! Standard Tauri build hook for generated platform resources and configuration.
//! Keep account/storage behavior in runtime modules rather than build scripts.

fn main() {
    tauri_build::build()
}
