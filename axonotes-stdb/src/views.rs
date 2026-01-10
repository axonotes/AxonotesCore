use crate::tables::{private_user__view, User};
use crate::{
    private_document__view, private_document_batch__view, private_document_key__view,
    private_document_metadata__view, private_document_permission__view,
    private_document_version_tag__view, private_live_block__view, private_pending_share__view,
    private_share_request__view, private_user_sync_conflict_history__view, Document, DocumentBatch,
    DocumentKey, DocumentMetadata, DocumentPermission, DocumentVersionTag, LiveBlock, PendingShare,
    Role, ShareRequest, UserSyncConflictHistory,
};
use spacetimedb::rt::IntoVec;
use spacetimedb::{Identity, ViewContext};
use std::collections::HashSet;

// ==================== USER VIEWS ====================
#[spacetimedb::view(name = user, public)]
pub fn user_view(ctx: &ViewContext) -> Vec<User> {
    ctx.db.private_user().identity().find(ctx.sender).into_vec()
}

/// Public user information (only public keys, no private keys)
#[derive(spacetimedb::SpacetimeType)]
pub struct PublicUserInfo {
    pub identity: Identity,
    pub public_encryption_key: Vec<u8>,
    pub public_signing_key: Vec<u8>,
}

/// Get public keys of users you collaborate with
/// Returns public keys for all users who have access to documents you can access
#[spacetimedb::view(name = public_user_keys, public)]
pub fn public_user_keys_view(ctx: &ViewContext) -> Vec<PublicUserInfo> {
    // Get all documents this user has access to
    let accessible_doc_ids = get_all_accessible_doc_ids(ctx);

    // Get all unique user_ids from those documents
    let mut user_ids = HashSet::new();
    for doc_id in accessible_doc_ids {
        for perm in ctx
            .db
            .private_document_permission()
            .by_doc()
            .filter(&doc_id)
        {
            user_ids.insert(perm.user_id);
        }
    }

    // Get public keys for all those users
    user_ids
        .into_iter()
        .filter_map(|user_id| {
            ctx.db
                .private_user()
                .identity()
                .find(user_id)
                .map(|user| PublicUserInfo {
                    identity: user.identity,
                    public_encryption_key: user.public_encryption_key,
                    public_signing_key: user.public_signing_key,
                })
        })
        .collect()
}

// ==================== DOCUMENT VIEWS ====================
fn get_all_accessible_doc_ids(ctx: &ViewContext) -> Vec<String> {
    ctx.db
        .private_document_permission()
        .by_user()
        .filter(&ctx.sender)
        .map(|perm| perm.doc_id.clone())
        .collect()
}

/// Get all documents the user has permission to access
#[spacetimedb::view(name = accessible_documents, public)]
pub fn accessible_documents_view(ctx: &ViewContext) -> Vec<Document> {
    // Get all doc_ids the user has permission for
    let accessible_doc_ids = get_all_accessible_doc_ids(ctx);

    // Get all those documents
    accessible_doc_ids
        .into_iter()
        .filter_map(|doc_id| ctx.db.private_document().doc_id().find(&doc_id))
        .collect()
}

// ==================== BATCH VIEW ====================

/// Get all batches for documents the user has access to
#[spacetimedb::view(name = accessible_batches, public)]
pub fn accessible_batches_view(ctx: &ViewContext) -> Vec<DocumentBatch> {
    // Get all accessible doc_ids
    let accessible_doc_ids = get_all_accessible_doc_ids(ctx);

    // Get all batches for those documents
    accessible_doc_ids
        .into_iter()
        .flat_map(|doc_id| {
            ctx.db
                .private_document_batch()
                .by_doc_and_timestamp()
                .filter(&doc_id)
                .collect::<Vec<_>>()
        })
        .collect()
}

// ==================== KEY VIEW ====================

/// Get all document keys for the current user
#[spacetimedb::view(name = user_document_keys, public)]
pub fn user_document_keys_view(ctx: &ViewContext) -> Vec<DocumentKey> {
    ctx.db
        .private_document_key()
        .by_user()
        .filter(&ctx.sender)
        .collect()
}

// ==================== PERMISSION VIEW ====================

/// Get all permissions for the current user
#[spacetimedb::view(name = user_permissions, public)]
pub fn user_permissions_view(ctx: &ViewContext) -> Vec<DocumentPermission> {
    ctx.db
        .private_document_permission()
        .by_user()
        .filter(&ctx.sender)
        .collect()
}

/// Get all permissions for documents where user is owner or editor
/// (so they can see who else has access)
#[spacetimedb::view(name = manageable_permissions, public)]
pub fn manageable_permissions_view(ctx: &ViewContext) -> Vec<DocumentPermission> {
    // Get doc_ids where user is owner or editor
    let manageable_doc_ids: Vec<String> = ctx
        .db
        .private_document_permission()
        .by_user()
        .filter(&ctx.sender)
        .filter(|perm| matches!(perm.role, Role::Owner | Role::Editor))
        .map(|perm| perm.doc_id.clone())
        .collect();

    // Get all permissions for those documents
    manageable_doc_ids
        .into_iter()
        .flat_map(|doc_id| {
            ctx.db
                .private_document_permission()
                .by_doc()
                .filter(&doc_id)
                .collect::<Vec<_>>()
        })
        .collect()
}

// ==================== METADATA VIEW ====================

/// Get all document metadata for the current user
#[spacetimedb::view(name = user_metadata, public)]
pub fn user_metadata_view(ctx: &ViewContext) -> Vec<DocumentMetadata> {
    ctx.db
        .private_document_metadata()
        .by_user()
        .filter(&ctx.sender)
        .collect()
}

// ==================== VERSION TAG VIEW ====================

/// Get all version tags for documents the user has access to
#[spacetimedb::view(name = accessible_version_tags, public)]
pub fn accessible_version_tags_view(ctx: &ViewContext) -> Vec<DocumentVersionTag> {
    // Get all accessible doc_ids
    let accessible_doc_ids = get_all_accessible_doc_ids(ctx);

    // Get all version tags for those documents
    accessible_doc_ids
        .into_iter()
        .flat_map(|doc_id| {
            ctx.db
                .private_document_version_tag()
                .by_doc()
                .filter(&doc_id)
                .collect::<Vec<_>>()
        })
        .collect()
}

// ==================== LIVE BLOCK VIEW ====================

/// Get all live blocks for documents the user has access to
#[spacetimedb::view(name = accessible_live_blocks, public)]
pub fn accessible_live_blocks_view(ctx: &ViewContext) -> Vec<LiveBlock> {
    // Get all accessible doc_ids
    let accessible_doc_ids = get_all_accessible_doc_ids(ctx);

    // Get all live blocks for those documents
    accessible_doc_ids
        .into_iter()
        .flat_map(|doc_id| {
            ctx.db
                .private_live_block()
                .by_doc()
                .filter(&doc_id)
                .collect::<Vec<_>>()
        })
        .collect()
}

// ==================== SHARE VIEWS ====================

/// Get all pending shares created by the current user
#[spacetimedb::view(name = my_pending_shares, public)]
pub fn my_pending_shares_view(ctx: &ViewContext) -> Vec<PendingShare> {
    ctx.db
        .private_pending_share()
        .by_creator()
        .filter(&ctx.sender)
        .collect()
}

/// Get all share requests for shares created by the current user
/// (to see who joined the share and their public keys)
#[spacetimedb::view(name = pending_share_requests, public)]
pub fn pending_share_requests_view(ctx: &ViewContext) -> Vec<ShareRequest> {
    // Get all share codes created by this user
    let my_share_codes: Vec<String> = ctx
        .db
        .private_pending_share()
        .by_creator()
        .filter(&ctx.sender)
        .map(|share| share.share_code.clone())
        .collect();

    // Get all share requests for those codes
    my_share_codes
        .into_iter()
        .flat_map(|share_code| {
            ctx.db
                .private_share_request()
                .by_share_code()
                .filter(&share_code)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Get all share requests for the current user
/// (to see shares they've joined and are waiting on)
#[spacetimedb::view(name = my_share_requests, public)]
pub fn my_share_requests_view(ctx: &ViewContext) -> Vec<ShareRequest> {
    ctx.db
        .private_share_request()
        .by_user()
        .filter(&ctx.sender)
        .collect()
}

// ==================== CONFLICT HISTORY VIEW ====================

/// Get all sync conflict history for the current user
#[spacetimedb::view(name = user_conflict_history, public)]
pub fn user_conflict_history_view(ctx: &ViewContext) -> Vec<UserSyncConflictHistory> {
    ctx.db
        .private_user_sync_conflict_history()
        .by_user()
        .filter(&ctx.sender)
        .collect()
}
