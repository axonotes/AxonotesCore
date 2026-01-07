# Axonotes Tauri Backend

**Rust backend for the Axonotes desktop application with end-to-end encryption, offline-first storage, and real-time collaboration.**

---

## About

This is the Rust backend of the Axonotes Tauri application. It handles all client-side logic including encryption, local database management, SpaceTimeDB synchronization, OAuth authentication, and blob storage. The backend exposes Tauri commands that the SvelteKit frontend calls via IPC.

## Features

- **End-to-End Encryption:** All content encrypted client-side before transmission
- **Offline-First:** SQLCipher database for local persistence with background sync
- **Real-Time Collaboration:** SpaceTimeDB subscriptions for live updates
- **Multi-Account Support:** Profile-scoped connections and storage
- **Secure Key Management:** BIP-39 mnemonic recovery with Argon2id key derivation
- **Blob Streaming:** Local encrypted cache with streaming decryption

## Directory Structure

```
src-tauri/
├── Cargo.toml
├── tauri.conf.json          # Tauri configuration
├── src/
│   ├── lib.rs                 # Tauri command registration
│   ├── main.rs                # Application entry point
│   ├── config.rs              # Generated app configuration
│   ├── commands/              # Tauri IPC commands (48 total)
│   │   ├── auth_cmd.rs        # Login/logout
│   │   ├── profile_cmd.rs     # Profile management
│   │   ├── database_cmd.rs    # Database lock/unlock
│   │   ├── encryption_cmd.rs  # Key management
│   │   ├── document_cmd.rs    # Document CRUD
│   │   ├── block_cmd.rs       # Block operations
│   │   ├── collab_cmd.rs      # Real-time collaboration
│   │   ├── share_cmd.rs       # Document sharing
│   │   ├── version_cmd.rs     # Version tags
│   │   ├── storage_cmd.rs     # Blob operations
│   │   └── workspace_cmd.rs   # UI workspace layouts
│   ├── crypto/                # Cryptographic primitives
│   │   ├── argon2.rs          # Password hashing
│   │   ├── bip39.rs           # Mnemonic phrases
│   │   ├── chacha20.rs        # Symmetric encryption
│   │   ├── ed25519.rs         # Digital signatures
│   │   └── x25519.rs          # Key exchange
│   ├── encryption/            # High-level encryption
│   │   ├── user.rs            # User key encryption
│   │   ├── document.rs        # Document encryption
│   │   ├── batch.rs           # Batch encryption
│   │   └── live_block.rs      # Live block encryption
│   ├── database/              # SQLCipher storage
│   │   ├── mod.rs             # Connection management
│   │   ├── schema.rs          # Table definitions
│   │   └── workspaces.rs      # UI workspace storage
│   ├── stdb/                  # SpaceTimeDB integration
│   │   ├── context.rs         # Profile-scoped operations
│   │   └── callbacks/         # Subscription handlers
│   ├── stdb_bindings/         # Generated SpaceTimeDB bindings
│   ├── storage/               # Blob storage client
│   │   ├── handler.rs         # HTTP server
│   │   ├── cache.rs           # Local blob cache
│   │   ├── chunked_crypto.rs  # Streaming encryption
│   │   └── server.rs          # Blob server
│   ├── storage_bindings/      # Storage API client
│   ├── batch_handler/         # Block & patch processing
│   ├── share/                 # Document sharing logic
│   │   └── key_rotation.rs    # Key rotation on user removal
│   ├── events/                # Tauri event emission
│   └── workos_auth/           # OAuth 2.0 + PKCE
└── tests/
    └── storage_integration.rs
```

## Tauri Commands

### Authentication & Profiles

| Command              | Description            |
| -------------------- | ---------------------- |
| `start_login`        | OAuth login via WorkOS |
| `logout`             | Delete user profile    |
| `get_active_profile` | Current user info      |
| `switch_profile`     | Change active user     |
| `refresh_token`      | Refresh OAuth token    |

### Database Management

| Command                   | Description                    |
| ------------------------- | ------------------------------ |
| `unlock_database`         | Decrypt database with password |
| `lock_database`           | Lock database                  |
| `set_database_encryption` | Set/change password            |
| `wipe_database`           | Delete all local data          |

### Encryption Keys

| Command                        | Description                          |
| ------------------------------ | ------------------------------------ |
| `create_stdb_user`             | Generate keys with password/mnemonic |
| `sync_stdb_keys_with_pwd`      | Recover keys with password           |
| `sync_stdb_keys_with_mnemonic` | Recover keys with BIP-39 phrase      |
| `update_pwd_from_mnemonic`     | Change password                      |

### Documents & Blocks

| Command           | Description                  |
| ----------------- | ---------------------------- |
| `create_document` | New encrypted document       |
| `delete_document` | Delete document (owner only) |
| `get_blocks`      | Retrieve blocks at timestamp |
| `update_block`    | Modify block content         |

### Real-Time Collaboration

| Command                | Description                  |
| ---------------------- | ---------------------------- |
| `request_lock`         | Lock block for editing       |
| `release_lock_focused` | Release with focus indicator |
| `update_live_block`    | Stream content while typing  |

### Document Sharing

| Command            | Description                                 |
| ------------------ | ------------------------------------------- |
| `create_share`     | Generate share code                         |
| `join_share`       | Join document with code                     |
| `update_user_role` | Change collaborator role                    |
| `remove_user`      | Remove collaborator (triggers key rotation) |

### Blob Storage

| Command                 | Description                  |
| ----------------------- | ---------------------------- |
| `upload_blob_from_path` | Upload file to storage       |
| `get_blob_url`          | Get HTTP URL for cached blob |
| `prefetch_blob`         | Download blob to cache       |
| `get_storage_quota`     | Get quota info               |

### Workspaces

| Command            | Description                      |
| ------------------ | -------------------------------- |
| `create_workspace` | Create new workspace with config |
| `get_workspace`    | Get workspace config by ID       |
| `list_workspaces`  | List all workspaces              |
| `update_workspace` | Update workspace config          |
| `delete_workspace` | Delete workspace by ID           |

## Encryption Architecture

```
User Password/Mnemonic
    ↓ (Argon2id - 19 rounds, 2GB memory)
User Private Keys (Ed25519 + X25519)
    ↓ (X25519 key exchange)
Document Keys (per-document, rotated on user removal)
    ↓ (ChaCha20-Poly1305)
Document Content (batches, metadata, live blocks)
```

### Key Types

| Key        | Algorithm | Purpose                          |
| ---------- | --------- | -------------------------------- |
| Signing    | Ed25519   | Authentication & integrity       |
| Encryption | X25519    | Asymmetric key exchange          |
| Document   | ChaCha20  | Symmetric content encryption     |
| Recovery   | BIP-39    | Mnemonic phrase for key recovery |

## Database Schema (SQLCipher)

| Table        | Purpose                         |
| ------------ | ------------------------------- |
| `profiles`   | User accounts with OAuth tokens |
| `keys`       | Encrypted user keypairs         |
| `batches`    | Document patches (pending sync) |
| `snapshots`  | Key rotation snapshots          |
| `blob_cache` | Cached blob metadata            |
| `workspaces` | UI workspace layouts (Dockview) |

## Events Emitted

The backend emits Tauri events to the frontend:

| Category          | Events                                              |
| ----------------- | --------------------------------------------------- |
| **Connection**    | `stdb-connected`, `stdb-disconnected`, `stdb-error` |
| **Document**      | `document-created`, `document-deleted`              |
| **Collaboration** | `collaborator-added`, `collaborator-removed`        |
| **Sharing**       | `share-code-ready`, `share-user-added`              |
| **Locks**         | `block-locked`, `block-lock-released`               |

## Development

### Prerequisites

- Rust toolchain (2021 edition)
- Tauri CLI
- Node.js/Bun (for frontend)

### Commands

```bash
# Start development server
axogen run dev

# Run tests
axogen run test

# Format code
axogen run fmt
```

## Tech Stack

- **Framework:** [Tauri](https://tauri.app/) 2.x
- **Database:** SQLCipher (encrypted SQLite)
- **Real-Time:** [SpaceTimeDB SDK](https://spacetimedb.com/) 1.11
- **Authentication:** WorkOS OAuth 2.0 + PKCE
- **Cryptography:**
  - `argon2` - Password hashing
  - `chacha20poly1305` - Symmetric encryption
  - `ed25519-dalek` - Digital signatures
  - `x25519-dalek` - Key exchange
  - `blake3` - Hashing
- **Async:** Tokio runtime
- **Caching:** Moka, DashMap
- **Tauri Plugins:**
  - `tauri-plugin-window-state` - Persistent window position/size
  - `tauri-plugin-store` - Key-value storage
  - `tauri-plugin-decorum` - Custom titlebar
