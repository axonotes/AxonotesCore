# Axonotes Desktop Application

**Cross-platform desktop application built with Tauri and SvelteKit.**

---

## About

This is the Axonotes desktop application that provides the user interface for the note-taking and collaboration platform. It combines a SvelteKit frontend with a Rust backend powered by Tauri.

## Directory Structure

```
axonotes-app/
├── src/                   # SvelteKit frontend (work in progress)
│   ├── routes/            # Page routes
│   └── lib/               # Shared components and utilities
├── src-tauri/             # Rust backend (see README)
│   └── src/               # Tauri commands and core logic
├── static/                # Static assets
├── package.json
├── svelte.config.js
├── vite.config.js
└── tsconfig.json
```

## Status

| Component                                                | Status           |
| -------------------------------------------------------- | ---------------- |
| **Rust Backend** ([`src-tauri/`](./src-tauri/README.md)) | Mostly complete  |
| **SvelteKit Frontend** (`src/`)                          | Work in progress |

## Development

```bash
# From repository root
axogen run dev
```

## Tech Stack

- **Framework:** [Tauri](https://tauri.app/) 2.x
- **Frontend:** [SvelteKit](https://kit.svelte.dev/) with TypeScript
- **Backend:** Rust (see [`src-tauri/README.md`](./src-tauri/README.md))
- **Styling:** Tailwind CSS

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) extension
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) extension
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) extension
