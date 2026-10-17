//! Native protection only. No plaintext or portable-secret fallback.
#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(not(any(windows, target_os = "macos")))]
compile_error!("Protected storage is implemented only for Windows and experimental macOS");
