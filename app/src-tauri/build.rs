/// Build script entry point that invokes Tauri's build-time generation.
///
/// This function is the Cargo build script entry (`build.rs`) and calls
/// `tauri_build::build()` to generate platform-specific code and configuration
/// required by a Tauri application.
///
/// # Examples
///
/// ```
/// // In a build script (build.rs)
/// fn main() {
///     tauri_build::build();
/// }
/// ```
fn main() {
    tauri_build::build()
}
