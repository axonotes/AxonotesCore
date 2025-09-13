// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

/// Program entry point; delegates startup to `app_lib::run()`.
///
/// This `main` function calls the library startup routine and intentionally ignores its return value.
///
/// # Examples
///
/// ```
/// // Equivalent to the application entry point:
/// app_lib::run();
/// ```
fn main() {
    app_lib::run()
}
