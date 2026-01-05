use std::sync::OnceLock;
use tauri::{AppHandle, Emitter};

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
