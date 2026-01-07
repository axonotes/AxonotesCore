//! # Reducer Helper Macros
//!
//! Provides synchronous reducer invocation for SpacetimeDB.
//!
//! ## Problem
//!
//! SpacetimeDB reducers are asynchronous - calling a reducer returns immediately
//! but the actual database operation happens later. To wait for completion,
//! you need to register a callback and coordinate with channels.
//!
//! ## Solution
//!
//! The `call_reducer_await!` macro handles all the coordination:
//! 1. Creates a oneshot channel for the result
//! 2. Registers a temporary callback on the reducer
//! 3. Calls the reducer
//! 4. Waits for the callback to fire
//! 5. Cleans up the callback
//! 6. Returns the result
//!
//! ## Usage
//!
//! ```ignore
//! let result = call_reducer_await!(
//!     conn,
//!     create_document,
//!     doc_id,
//!     encrypted_data
//! )?;
//! ```

/// Helper to call a reducer and wait for its completion.
///
/// This macro wraps the async callback dance required by SpacetimeDB
/// into a simple synchronous-looking call.
///
/// IMPORTANT: The callback runs on SpacetimeDB's background thread (from run_threaded()),
/// NOT on the Tokio runtime. We must use std::sync primitives, not tokio::sync.
#[macro_export]
macro_rules! call_reducer_await {
    ($conn:expr, $reducer_name:ident, $($arg:expr),* $(,)?) => {{
        use tokio::sync::oneshot;
        use std::sync::{Arc, Mutex as StdMutex};
        use spacetimedb_sdk::__codegen::log;

        log::info!("[reducer_await] Setting up {} reducer callback...", stringify!($reducer_name));

        let (tx, rx) = oneshot::channel::<Result<(), String>>();
        let tx = Arc::new(StdMutex::new(Some(tx)));

        // Clone all arguments for comparison
        let expected = ($($arg.clone()),*);

        paste::paste! {
            let callback_ref = $conn.reducers.[<on_ $reducer_name>](move |ctx, $($arg),*| {
                log::info!("[reducer_await] {} callback fired!", stringify!($reducer_name));

                // Check if ALL arguments match
                let actual = ($($arg.clone()),*);
                if actual != expected {
                    log::debug!("[reducer_await] {} callback: args don't match, ignoring", stringify!($reducer_name));
                    return;
                }

                log::debug!("[reducer_await] {} callback: args match, checking status...", stringify!($reducer_name));

                let result = match &ctx.event.status {
                    spacetimedb_sdk::Status::Failed(err) => {
                        log::error!("[reducer_await] {} failed: {}", stringify!($reducer_name), err);
                        Err(err.to_string())
                    },
                    spacetimedb_sdk::Status::Committed => {
                        log::debug!("[reducer_await] {} committed successfully", stringify!($reducer_name));
                        Ok(())
                    },
                    status => {
                        log::debug!("[reducer_await] {} unexpected status: {:?}", stringify!($reducer_name), status);
                        return;
                    },
                };

                // Use std::sync::Mutex since this callback runs on SpacetimeDB's
                // background thread, not the Tokio runtime
                if let Ok(mut guard) = tx.lock() {
                    if let Some(sender) = guard.take() {
                        log::debug!("[reducer_await] {} sending result through channel", stringify!($reducer_name));
                        let _ = sender.send(result);
                    }
                }
            });

            log::info!("[reducer_await] Calling {} reducer...", stringify!($reducer_name));
            $conn.reducers.$reducer_name($($arg),*)
                .map_err(|e| {
                    log::error!("[reducer_await] Failed to call {} reducer: {}", stringify!($reducer_name), e);
                    e.to_string()
                })?;

            log::info!("[reducer_await] {} reducer called, waiting for callback...", stringify!($reducer_name));
            let result: Result<(), String> = rx.await
                .map_err(|_| {
                    log::error!("[reducer_await] {} callback never called (channel closed)", stringify!($reducer_name));
                    concat!("Reducer ", stringify!($reducer_name), " callback never called").to_string()
                })?;

            log::debug!("[reducer_await] {} completed, cleaning up callback", stringify!($reducer_name));
            $conn.reducers.[<remove_on_ $reducer_name>](callback_ref);

            result
        }
    }};
}
