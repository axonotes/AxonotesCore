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
#[macro_export]
macro_rules! call_reducer_await {
    ($conn:expr, $reducer_name:ident, $($arg:expr),* $(,)?) => {{
        use tokio::sync::oneshot;
        use std::sync::Arc;
        use tokio::sync::Mutex as TokioMutex;

        let (tx, rx) = oneshot::channel::<Result<(), String>>();
        let tx = Arc::new(TokioMutex::new(Some(tx)));

        // Clone all arguments for comparison
        let expected = ($($arg.clone()),*);

        paste::paste! {
            let callback_ref = $conn.reducers.[<on_ $reducer_name>](move |ctx, $($arg),*| {
                // Check if ALL arguments match
                let actual = ($($arg.clone()),*);
                if actual != expected {
                    return;
                }

                let result = match &ctx.event.status {
                    spacetimedb_sdk::Status::Failed(err) => Err(err.to_string()),
                    spacetimedb_sdk::Status::Committed => Ok(()),
                    _ => return,
                };

                let tx = tx.clone();
                tokio::spawn(async move {
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(result);
                    }
                });
            });

            $conn.reducers.$reducer_name($($arg),*)
                .map_err(|e| e.to_string())?;

            let result: Result<(), String> = rx.await
                .map_err(|_| concat!("Reducer ", stringify!($reducer_name), " callback never called").to_string())?;

            $conn.reducers.[<remove_on_ $reducer_name>](callback_ref);

            result
        }
    }};
}
