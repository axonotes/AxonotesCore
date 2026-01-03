/// Helper to call a reducer and wait for its completion
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
