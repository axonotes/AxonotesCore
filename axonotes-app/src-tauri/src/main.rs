//! # Axonotes Application Entry Point
//!
//! This is the main entry point for the Tauri desktop application.
//! The actual application logic is in `lib.rs` to enable library testing.

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    axonotes_app_lib::run()
}
