//! # Global App Handle
//!
//! Provides global access to the Tauri `AppHandle` for emitting events
//! from anywhere in the application.
//!
//! ## Usage
//!
//! ```ignore
//! // During Tauri setup
//! app_handle::init(app.handle().clone());
//!
//! // Later, to emit events
//! app_handle::emit("my-event", payload)?;
//! ```
//!
//! ## Why Global?
//!
//! SpacetimeDB callbacks and background tasks don't have access to
//! Tauri's managed state. The global handle enables event emission
//! from anywhere.

use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

/// Global storage for the Tauri app handle.
///
/// Initialized once during `setup()` and accessed throughout the app lifetime.
static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

/// Initialize the global app handle (call once during setup)
pub fn init(handle: AppHandle) {
    APP_HANDLE
        .set(handle)
        .expect("AppHandle already initialized");
}

/// Get the global app handle
pub fn get() -> &'static AppHandle {
    APP_HANDLE
        .get()
        .expect("AppHandle not initialized. Call init() first.")
}

/// Emit an event to the frontend
pub fn emit<S: serde::Serialize + Clone>(event: &str, payload: S) -> Result<(), String> {
    get().emit(event, payload).map_err(|e| e.to_string())
}
