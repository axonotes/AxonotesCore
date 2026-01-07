# Axogen

**Code generation and build automation configuration for the Axonotes monorepo.**

---

## About

This directory contains the Axonotes-specific configuration for [Axogen](https://axonotes.github.io/axogen/), a TypeScript-native configuration system that unifies typed environment variables, code generation, and task management. Axogen allows defining configuration once and generating it across multiple formats and languages.

## Features

- **Template-based Code Generation:** Generates Rust configuration files and Docker Compose manifests from Nunjucks templates
- **Multi-service Orchestration:** Manages the Tauri app, SpaceTimeDB module, and Storage service
- **Tool Detection:** Validates required tools (Bun, Cargo, Docker, etc.) before executing commands
- **Configuration Validation:** Uses Zod schemas to validate `config.toml`

## Directory Structure

```
axogen/
├── config.ts              # Zod schema for config.toml validation
├── commands/              # CLI command definitions
│   ├── dev.ts             # Development server
│   ├── fmt.ts             # Code formatting
│   ├── install.ts         # Dependency installation
│   ├── test.ts            # Test runner
│   ├── stdb.ts            # SpaceTimeDB commands
│   ├── storage.ts         # Storage service commands
│   └── shared/            # Shared command utilities
├── targets/               # Template output definitions
│   ├── app/               # Tauri app targets
│   └── storage/           # Storage service targets
├── templates/             # Nunjucks templates
│   ├── app/               # Rust config templates
│   └── storage/           # Docker Compose templates
└── utils/
    └── tool-detection.ts  # Tool availability checker
```

## Commands

Run commands from the repository root using `axogen run <command>`:

| Command              | Description                                     |
| -------------------- | ----------------------------------------------- |
| `axogen run install` | Install all dependencies using Bun              |
| `axogen run dev`     | Start the Tauri development server              |
| `axogen run fmt`     | Format codebase (Prettier + Cargo fmt + ESLint) |
| `axogen run test`    | Run all unit tests                              |

### SpaceTimeDB Commands

| Command                    | Description                                    |
| -------------------------- | ---------------------------------------------- |
| `axogen run stdb generate` | Generate Rust bindings from SpaceTimeDB module |
| `axogen run stdb dev`      | Start SpaceTimeDB server and publish module    |
| `axogen run stdb dev -c`   | Clear data before publishing                   |

### Storage Commands

| Command                      | Description                               |
| ---------------------------- | ----------------------------------------- |
| `axogen run storage dev`     | Start storage service with Docker Compose |
| `axogen run storage stop`    | Stop the storage service                  |
| `axogen run storage restart` | Restart the service                       |
| `axogen run storage logs`    | View service logs                         |
| `axogen run storage status`  | Show container status                     |
| `axogen run storage build`   | Build Docker images                       |
| `axogen run storage test`    | Run storage tests                         |
| `axogen run storage clean`   | Remove containers and volumes             |

### Code Generation

| Command           | Description                                     |
| ----------------- | ----------------------------------------------- |
| `axogen generate` | Generate all configuration files from templates |

## Generated Files

Axogen generates the following files from templates:

| Template                                   | Output                                 | Purpose                 |
| ------------------------------------------ | -------------------------------------- | ----------------------- |
| `templates/app/config.rs.njk`              | `axonotes-app/src-tauri/src/config.rs` | Rust app configuration  |
| `templates/storage/docker-compose.yml.njk` | `axonotes-storage/docker-compose.yml`  | Docker Compose manifest |

## Configuration

Axogen reads from `config.toml` in the repository root. See the root README for configuration options.

## Requirements

- **Bun** - JavaScript runtime and package manager
- **Cargo** - Rust toolchain
- **Docker** - Container runtime (for storage service)
- **SpaceTime CLI** - SpaceTimeDB command-line tool

## Documentation

For more information about Axogen, see the [official documentation](https://axonotes.github.io/axogen/docs/intro/).
