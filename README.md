<p align="center">
  <a href="./">
    <img src="assets/logo_no_text.png" alt="Axonotes Logo" width="150"/>
  </a>
</p>

<h1 align="center">AxonotesCore 🐙</h1>

<p align="center">
  <strong>Core monorepo for the Axonotes Desktop application (Tauri/SvelteKit) and its SpaceTimeDB Rust backend.</strong>
  <br />
  <em>Currently in Beta-1.</em>
</p>

<p align="center">
  <a href="#about-axonotes">About Axonotes</a> •
  <a href="#whats-inside-this-monorepo">What's Inside?</a> •
  <a href="#current-stage">Current Stage</a> •
  <a href="#tech-stack">Tech Stack</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#stay-connected-with-axonotes">Stay Connected with Axonotes</a> •
  <a href="#star-history">⭐ Star History</a> •
  <a href="#license-overview">License Overview</a>
</p>

[![Status](https://img.shields.io/badge/status-beta--1-blue)](https://github.com/axonotes/AxonotesCore)

---

## 🎯 About Axonotes

Axonotes is envisioned as the ultimate command center for students and educators, designed to end the chaos of juggling
multiple applications. Our goal is to create a single, streamlined platform where notes, collaboration, planning,
learning tools (like flashcards and interactive exercises), and communication live together seamlessly.

We're building an all-in-one academic suite focused on:

- **Unified Workflow:** Notes, tasks, chat, and learning tools in one place.
- **Effortless Collaboration:** Real-time co-authoring with features like line-level locking.
- **Powerful Knowledge Creation:** Flexible note-taking (Markdown, rich text, infinite canvases, pen support), `LaTeX`
  support, and more.
- **Smart Organization:** Integrated planning, powerful global search, and knowledge graphs.
- **Offline-First & Cross-Platform:** Work anywhere, anytime, on any device.
- **Revolutionary Version History:** Every change saved, powered by SpaceTimeDB.
- **Secure & Private by Design:** Built with Swiss precision.

## 📦 What's Inside This Monorepo?

This `AxonotesCore` repository is a monorepo that houses the foundational code for Axonotes:

- **[`/axonotes-app`](./axonotes-app)**:
  - The Axonotes desktop application.
  - Built with **[Tauri](https://tauri.app/)** (using **[SvelteKit](https://kit.svelte.dev/)** for the frontend).
  - Provides the cross-platform user interface (Windows, macOS, Linux).
  - **[`/axonotes-app/src-tauri`](./axonotes-app/src-tauri)**: Rust backend with end-to-end encryption, offline-first SQLCipher database, and SpaceTimeDB synchronization.

- **[`/axonotes-stdb`](./axonotes-stdb)**:
  - The real-time backend module running on **[SpaceTimeDB](https://spacetimedb.com/)**.
  - Written in **Rust**, compiles to WebAssembly.
  - Manages real-time collaboration, access control, key distribution, and version history.
  - All data encrypted client-side; server never sees plaintext content.

- **[`/axonotes-storage`](./axonotes-storage)**:
  - Blob storage microservice for files and media.
  - Built with **[Axum](https://github.com/tokio-rs/axum)** and **S3-compatible storage** (MinIO).
  - Features streaming uploads, Ed25519 signature verification, and dynamic quota management.

- **[`/axogen`](./axogen)**:
  - Build automation and code generation configuration using **[Axogen](https://axonotes.github.io/axogen/)**.
  - Generates Rust configuration files and Docker Compose manifests from templates.
  - Provides unified CLI commands for development, testing, and service orchestration.

## ⏳ Current Stage

Axonotes and this `AxonotesCore` repository are currently in **Beta-1**.

| Component                                      | Status           | Notes                                                                   |
| ---------------------------------------------- | ---------------- | ----------------------------------------------------------------------- |
| **Rust Backend** (Tauri, SpaceTimeDB, Storage) | ~90%             | Core features implemented. Some nice-to-haves and improvements pending. |
| **SvelteKit Frontend**                         | Work in Progress | Needs to integrate most backend APIs.                                   |

We're actively developing the frontend and refining the user experience based on community feedback.

## ⭐ Star History

[![Star History Chart](https://api.star-history.com/svg?repos=axonotes/AxonotesCore&type=Date)](https://www.star-history.com/#axonotes/AxonotesCore&Date)

## 🛠️ Tech Stack

- **Desktop Application ([`/axonotes-app`](./axonotes-app)):**
  - Framework: [Tauri](https://tauri.app/) 2.x
  - Frontend: [SvelteKit](https://kit.svelte.dev/) with TypeScript
  - Backend: Rust with [SpaceTimeDB SDK](https://spacetimedb.com/)
  - Database: SQLCipher (encrypted SQLite)
  - Authentication: WorkOS OAuth 2.0 + PKCE

- **Real-time Backend ([`/axonotes-stdb`](./axonotes-stdb)):**
  - Platform: [SpaceTimeDB](https://spacetimedb.com/) 1.11+
  - Language: Rust (compiles to WebAssembly)

- **Storage Service ([`/axonotes-storage`](./axonotes-storage)):**
  - Framework: [Axum](https://github.com/tokio-rs/axum)
  - Storage: S3-compatible (MinIO / AWS S3)
  - Database: SQLite

- **Cryptography:**
  - Symmetric: ChaCha20-Poly1305
  - Asymmetric: X25519 (key exchange), Ed25519 (signatures)
  - Hashing: BLAKE3, Argon2id (password derivation)
  - Recovery: BIP-39 mnemonic phrases

- **Build Tools ([`/axogen`](./axogen)):**
  - [Axogen](https://axonotes.github.io/axogen/) - Configuration and task management
  - [Bun](https://bun.sh/) - JavaScript runtime
  - Docker Compose - Service orchestration

## 🚀 Getting Started

### Prerequisites

- **Rust Toolchain:** For Tauri, SpaceTimeDB modules, and storage service
- **Bun:** JavaScript runtime and package manager ([bun.sh](https://bun.sh/))
- **SpaceTime CLI:** For SpaceTimeDB development (`spacetime`)
- **Docker:** For running the storage service with MinIO
- **Tauri Prerequisites:** Follow the [Tauri setup guide](https://tauri.app/start/prerequisites/) for your OS

### Quick Start

```bash
# Install dependencies
axogen run install

# Start SpaceTimeDB server (in separate terminal)
axogen run stdb dev

# Start storage service (in separate terminal)
axogen run storage dev

# Start the Tauri development server
axogen run dev
```

### Configuration

Copy and configure the settings in `config.toml`:

```toml
[app]
name = "AxonotesApp"
version = "0.0.1"
environment = "development"

[oauth]
client_id = "your-workos-client-id"
# ... see config.toml for full options

[stdb]
default_host_uri = "http://localhost:3000"
default_module_name = "axonotes"

[storage]
base_url = "http://localhost:8081"
```

### Common Commands

| Command                  | Description                    |
| ------------------------ | ------------------------------ |
| `axogen run install`     | Install all dependencies       |
| `axogen run dev`         | Start Tauri development server |
| `axogen run fmt`         | Format all code                |
| `axogen run test`        | Run all tests                  |
| `axogen run stdb dev`    | Start SpaceTimeDB server       |
| `axogen run storage dev` | Start storage service          |
| `axogen generate`        | Regenerate configuration files |

See the [axogen README](./axogen/README.md) for more commands.

## 🤝 Contributing

Your insights, experiences, and ideas are critical at this early stage! While direct code contributions to
`AxonotesCore` will become more streamlined as the project matures, here's how you can help shape Axonotes right now:

- 📧 **Share Your Thoughts via Email:** Send your ideas, your biggest frustrations with current tools, and your dream features to:
  `oliver@axonotes.ch`
- 📝 **Fill Out Our Quick Survey:** [https://forms.gle/N2qFoXn4PonD6EnA9](https://forms.gle/N2qFoXn4PonD6EnA9)
- ⭐ **Watch this Repository:** Stay updated on our progress.
- 💡 **Open Issues:** Feel free to open issues in this repository for specific bugs you anticipate or features directly
  related to the core application's structure or functionality.
- 🗣️ **Spread the Word:** Sharing Axonotes with friends, classmates, and colleagues helps immensely!

We plan to be open to direct Pull Request suggestions for features and improvements that may be accepted into the core
product. Formal contribution guidelines (`CONTRIBUTING.md`) will be added as the codebase stabilizes.

## 🌐 Stay Connected with Axonotes

Follow the overall Axonotes project for updates, announcements, and community discussion:

- **Website:** [axonotes.ch](https://axonotes.ch) (Coming Soon!)
- **Discord Server:** [https://discord.gg/myBMaaDeQu](https://discord.gg/myBMaaDeQu)
- **X (Twitter):** [@axonotes](https://twitter.com/axonotes)
- **YouTube:** [@axonotes](https://youtube.com/@axonotes)
- **Reddit:** [r/axonotes](https://www.reddit.com/r/Axonotes/)
- **BlueSky:** [@axonotes.bsky.social](https://bsky.app/profile/axonotes.bsky.social)

## 📜 License Overview

AxonotesCore (Version 0.0.0) is licensed under the **Business Source License 1.1 (BSL 1.1)**.

- **Until May 2, 2030 (the "Change Date"):**
  - You **CAN** copy, modify, create derivative works, and redistribute the software.
  - You **CAN** use it for non-production purposes.
  - For **production use**, you can self-host it for internal purposes for **up to 50 individual users**.
  - You **CANNOT** offer it as a commercial hosted service or exceed the 50-user limit in production without a separate commercial license from Axonotes.
- **On or After May 2, 2030:**
  - The license will automatically convert to the **GNU Affero General Public License v3.0 or later (AGPLv3+)**.
- **Important:**
  - The software is **not considered open-source** until the Change Date.
  - You must include the BSL 1.1 license text with any distribution.

This is a brief summary. For full terms and conditions, please see the [LICENSE](LICENSE) file.

---

Thank you for your interest in AxonotesCore! We're excited to build the future of academic software with you.

Best regards,
Oliver & the (future) Axonotes Team
(A Swiss-based initiative)
