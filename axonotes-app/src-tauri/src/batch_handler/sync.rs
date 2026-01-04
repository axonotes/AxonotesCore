use crate::stdb_bindings::{
    AccessibleBatchesTableAccess, DbConnection, DocumentBatch, SubscriptionEventContext,
};
use crate::utils::timestamp::timestamp;
use spacetimedb_sdk::{DbContext, Table};

pub async fn setup_batch_sync(conn: &DbConnection) -> Result<(), String> {
    let start_time = timestamp();

    // TODO: sanitising is not hugely important since start_time should just be a number. but might be worth investigating in the future
    conn.subscription_builder()
        .on_applied(|ctx| {
            // This will happen everytime there is an update to the accessible_batch table
            sync_batches(ctx);
        })
        .on_error(|error_ctx, error| {
            // This will error when something ain't right.
        })
        .subscribe([format!(
            "SELECT * FROM accessible_batches WHERE timestamp > {start_time}"
        )]);

    Ok(())
}

pub async fn sync_batches(ctx: &SubscriptionEventContext) -> Result<(), String> {
    let stdb_batches: Vec<DocumentBatch> = ctx.db.accessible_batches().iter().collect();

    // TODO: Sync local batches with server batches

    Ok(())
}
