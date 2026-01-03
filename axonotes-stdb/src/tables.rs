use spacetimedb::{Identity, SpacetimeType};

// ==================== USER ====================
#[spacetimedb::table(name = private_user)]
pub struct User {
    #[primary_key]
    pub identity: Identity,

    // Asymmetric Encryption
    pub public_encryption_key: Vec<u8>,
    pub pwd_encrypted_private_encryption_key: Vec<u8>,
    pub mnemonic_encrypted_private_encryption_key: Vec<u8>,

    // Signing
    pub public_signing_key: Vec<u8>,
    pub pwd_encrypted_private_signing_key: Vec<u8>,
    pub mnemonic_encrypted_private_signing_key: Vec<u8>,
}

// ==================== DOCUMENT ====================
#[spacetimedb::table(name = private_document)]
pub struct Document {
    #[primary_key]
    pub doc_id: String, // Document UUID
    #[index(btree)]
    pub owner_id: Identity, // Document Owner
    pub current_public_signing_key: Vec<u8>, // Ed25519 public key (for signing batches)
    pub key_timestamp: u64,                  // Current key version timestamp
}

// ==================== DOCUMENT BATCH ====================
#[spacetimedb::table(
    name = private_document_batch,
    index(name = by_doc_and_timestamp, btree(columns = [doc_id, timestamp]))
)]
pub struct DocumentBatch {
    #[primary_key]
    pub batch_id: String, // Random UUID
    pub doc_id: String,          // Document identifier
    pub timestamp: u64,          // When the batch got created
    pub encrypted_data: Vec<u8>, // block_id + patches (ChaCha20-Poly1305)
}

// ==================== DOCUMENT KEY ====================
#[spacetimedb::table(
    name = private_document_key,
    index(name = by_doc_and_user, btree(columns = [doc_id, user_id])),
    index(name = by_user, btree(columns = [user_id])),
)]
pub struct DocumentKey {
    #[primary_key]
    pub key_id: String, // Random UUID
    pub doc_id: String,    // Document identifier
    pub user_id: Identity, // Who the key belongs to
    #[index(btree)]
    pub key_timestamp: u64, // Which key version
    pub encrypted_data: Vec<u8>, // Encrypted signing and encryption key with user's public key
}

// ==================== DOCUMENT PERMISSION ====================
#[spacetimedb::table(
    name = private_document_permission,
    index(name = by_doc, btree(columns = [doc_id])),
    index(name = by_user, btree(columns = [user_id])),
    index(name = by_doc_and_user, btree(columns = [doc_id, user_id])),
)]
pub struct DocumentPermission {
    #[primary_key]
    pub permission_id: String, // Random UUID
    pub doc_id: String,    // Document identifier
    pub user_id: Identity, // Who the rule belongs to
    pub role: Role,
}

#[derive(SpacetimeType)]
pub enum Role {
    Owner,  // Singleton, transferable, cannot be removed
    Editor, // Can edit, share, manage users
    Reader, // Read-only access
}

// ==================== DOCUMENT METADATA ====================
#[spacetimedb::table(
    name = private_document_metadata,
    index(name = by_user_and_doc, btree(columns = [user_id, doc_id])),
    index(name = by_user, btree(columns = [user_id])),
)]
pub struct DocumentMetadata {
    #[primary_key]
    pub meta_id: String, // Random UUID
    pub user_id: Identity,
    pub doc_id: String,

    // Encrypted JSON blob containing:
    // {
    //   "version": 1,
    //   "path": "/School/Math/Pythagorean.doc",
    //   "tags": ["geometry", "important"]
    // }
    pub encrypted_blob: Vec<u8>,
}

// ==================== DOCUMENT VERSION TAG ====================
#[spacetimedb::table(
    name = private_document_version_tag,
    index(name = by_doc, btree(columns = [doc_id])),
)]
pub struct DocumentVersionTag {
    #[primary_key]
    pub tag_id: String, // Random UUID
    pub doc_id: String,

    // Encrypted JSON blob containing:
    // {
    //   "tag_name": "v1.0",
    //   "timestamp": 1234567890,
    //   "created_by": "user_id",
    //   "created_at": 1234567890
    // }
    pub encrypted_blob: Vec<u8>,
}

// ==================== LIVE BLOCK ====================
#[spacetimedb::table(
    name = private_live_block,
    index(name = by_doc, btree(columns = [doc_id])),
    index(name = by_doc_and_block, btree(columns = [doc_id, block_id])),
    index(name = by_user, btree(columns = [user_id])),
)]
pub struct LiveBlock {
    #[primary_key]
    pub live_block_id: String, // Random UUID
    pub doc_id: String,
    pub block_id: u64, // Real block ID
    pub user_id: Identity,
    pub encrypted_content: Vec<u8>, // Full block state (ChaCha20-Poly1305)
    pub encrypted_username: Vec<u8>, // Username encrypted (ChaCha20-Poly1305)
    pub locked_at: Option<u128>,    // Some(timestamp) = Locked, None = Focused
}
