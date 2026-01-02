use super::*;

pub fn register_callbacks(_conn: &DbConnection) {
    // Nothing here yet
}

// ==========================================
// Subscription Callbacks
// ==========================================

pub fn on_subscription_applied(_ctx: &SubscriptionEventContext) {
    log::info!("✓ Subscriptions applied");
}

pub fn on_subscription_error(_ctx: &ErrorContext, err: Error) {
    log::error!("✗ Subscription error: {}", err);
}
