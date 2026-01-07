# Axonotes SpaceTimeDB Module

**Real-time, end-to-end encrypted collaborative document backend powered by SpaceTimeDB.**

---

## About

This module implements the server-side logic for Axonotes using [SpaceTimeDB](https://spacetimedb.com/), a real-time relational database that compiles to WebAssembly. It handles document management, real-time collaboration, access control, and secure key distribution while ensuring the server never has access to unencrypted content.

## Features

- **End-to-End Encryption:** All document content encrypted client-side using ChaCha20-Poly1305
- **Real-Time Collaboration:** Block-level locking with live content streaming
- **Cryptographic Access Control:** Ed25519 signatures verify all state-changing operations
- **Key Rotation:** Automatic re-encryption when collaborators are removed
- **Secure Sharing:** Time-limited share codes for document invitations
- **Version History:** Complete edit history with named version tags

## Directory Structure

```
axonotes-stdb/
├── Cargo.toml
└── src/
    ├── lib.rs                 # Module entry point
    ├── tables.rs              # Database table definitions
    ├── views.rs               # SpaceTimeDB views (read queries)
    ├── crypto/
    │   └── ed25519.rs         # Signature verification
    ├── reducers/              # State-changing operations
    │   ├── user.rs            # User account management
    │   ├── document.rs        # Document CRUD
    │   ├── batch.rs           # Document patches
    │   ├── key.rs             # Key rotation
    │   ├── permission.rs      # Access control
    │   ├── share.rs           # Document sharing
    │   ├── live_block.rs      # Real-time editing
    │   ├── metadata.rs        # Document metadata
    │   ├── version_tag.rs     # Version snapshots
    │   └── lifecycle.rs       # Connect/disconnect hooks
    └── utils/
        ├── auth.rs            # Signature verification utilities
        ├── uuid.rs            # Deterministic UUID generation
        └── share_code.rs      # Share code generation
```

## Database Schema

### Core Tables

| Table                          | Purpose                                 |
| ------------------------------ | --------------------------------------- |
| `private_user`                 | User accounts with encrypted keypairs   |
| `private_document`             | Document metadata and signing keys      |
| `private_document_batch`       | Encrypted document patches              |
| `private_document_key`         | Per-user encrypted document keys        |
| `private_document_permission`  | User roles (Owner/Editor/Reader)        |
| `private_document_metadata`    | User-specific document organization     |
| `private_document_version_tag` | Named version snapshots                 |
| `private_live_block`           | Real-time collaborative editing state   |
| `private_pending_share`        | Active share codes                      |
| `private_share_request`        | Users requesting to join via share code |

## Security Model

### Encryption Hierarchy

```
User Password/Mnemonic
    ↓ (Client-side Argon2id)
User Private Keys (Ed25519 + X25519)
    ↓ (X25519 key exchange)
Document Keys (per-document, rotated on user removal)
    ↓ (ChaCha20-Poly1305)
Document Content
```

### What the Server Never Sees

- User passwords (encrypted client-side)
- Unencrypted document content
- Plaintext encryption keys
- Unencrypted metadata (paths, tags)

### Access Control

| Role       | Capabilities                                            |
| ---------- | ------------------------------------------------------- |
| **Owner**  | Full control, transfer ownership, delete document       |
| **Editor** | Edit content, manage collaborators, create version tags |
| **Reader** | Read-only access                                        |

## Key Operations

### Reducers (Write Operations)

| Category         | Operations                                                                                    |
| ---------------- | --------------------------------------------------------------------------------------------- |
| **User**         | `create_user`, `set_encryption_keys`                                                          |
| **Document**     | `create_document`, `delete_document`                                                          |
| **Batches**      | `upload_batch` (with signature verification)                                                  |
| **Keys**         | `rotate_document_keys` (atomic key rotation)                                                  |
| **Permissions**  | `add_user_to_document`, `remove_user_from_document`, `change_user_role`, `transfer_ownership` |
| **Sharing**      | `create_pending_share`, `join_pending_share`, `close_pending_share`                           |
| **Live Blocks**  | `try_lock_block`, `update_live_block`, `unlock_block`                                         |
| **Metadata**     | `create_document_metadata`, `update_document_metadata`                                        |
| **Version Tags** | `create_version_tag`, `delete_version_tag`                                                    |

### Views (Read Operations)

All views filter results based on caller identity:

- `user()` - Current user's record
- `accessible_documents()` - Documents user can access
- `accessible_batches()` - Patches for accessible documents
- `user_document_keys()` - User's encrypted document keys
- `user_permissions()` - User's document permissions
- `accessible_live_blocks()` - Real-time editing state
- `my_pending_shares()` - User's active share codes

## Development

### Prerequisites

- Rust toolchain (2021 edition)
- SpaceTime CLI (`spacetime`)

### Commands

```bash
# Generate client bindings
axogen run stdb generate

# Start development server
axogen run stdb dev

# Start with fresh data
axogen run stdb dev -c
```

### Publishing

The module compiles to WebAssembly (`cdylib`) and is published to a SpaceTimeDB instance using the SpaceTime CLI.

## Tech Stack

- **Platform:** [SpaceTimeDB](https://spacetimedb.com/) 1.11+
- **Language:** Rust 2021 edition
- **Cryptography:**
  - `ed25519-dalek` - Digital signatures
  - `blake3` - Cryptographic hashing
  - `uuid` - Unique identifiers
  - `rand` - Secure random number generation
