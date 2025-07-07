# axonotes Core: Developer Setup Guide

This guide provides instructions for developers to set up the environment, run the project, and contribute to the `axonotes Core` monorepo.

## 1. Prerequisites

Before you begin, ensure you have the following tools installed on your system.

- **Rust Toolchain:** Required for the SpacetimeDB backend and CLI. You can install it using `rustup`.
    - [Install Rust](https://www.rust-lang.org/tools/install)
- **Node.js and Bun:** Required for the SvelteKit frontend, scripts, and dependency management. We use `bun` as the package manager and runtime.
    - [Install Node.js](https://nodejs.org/)
    - [Install Bun](https://bun.sh/docs/installation)

### Future Requirements: Tauri

The desktop application will be built with Tauri. While not required for developing the dashboard at this moment, you can prepare your system by following the official Tauri v2 setup guide.

- [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

## 2. Initial Project Setup

Follow these steps to get the project configured on your local machine.

1.  **Clone the Repository**
    Clone the `axonotesCore` repository to your local machine.

    ```sh
    git clone https://github.com/axonotes/AxonotesCore.git
    cd AxonotesCore
    ```

2.  **Install Root Dependencies**
    Install the root-level development dependencies, which include tools like `prettier`, `eslint`, and `husky`.

    ```sh
    bun install
    ```

3.  **Build and Install the CLI**
    The project uses a smart CLI tool that handles most development tasks. Build and install it globally:

    ```sh
    bun run cli:install
    ```

    This makes the `axonotes` command available system-wide.

4.  **Run the Project Setup**
    Use the smart CLI to set up your development environment:

    ```sh
    axonotes setup
    ```

    This command will:
    - Generate `axonotes.toml` configuration file with sensible defaults
    - Check and generate cryptographic keys for JWT authentication
    - Install dashboard dependencies automatically
    - Build SpacetimeDB CLI tools
    - Provide a health report of your setup

5.  **Configure WorkOS (Required for Authentication)**

    **WorkOS Setup for Testing:**
    1. Create a free WorkOS account: https://workos.com/signin
    2. Get your credentials from the dashboard:
        - Navigate to "API Keys" section
        - Copy your Client ID and API Key (starts with `sk_`)
    3. Configure redirect URI:
        - Go to "Redirects" section in dashboard
        - Add: `http://localhost:5173/auth/callback`
    4. Fill out your `dashboard/.env` file with your credentials:
       ```env
       WORKOS_CLIENT_ID="your_client_id_here"
       WORKOS_API_KEY="your_api_key_here"
       ```

    **Note:** The CLI will have generated other required keys automatically. Only WorkOS credentials need manual setup.

6.  **Log in to SpacetimeDB**
    Before you can publish backend modules, authenticate with SpacetimeDB:

    ```sh
    axonotes sdb login
    ```

    This will open a browser window for you to log in or create a SpacetimeDB account.

## 3. Running the Development Environment

The development workflow involves running a local server, publishing your modules to generate client code, and then running the frontend application.

### Step 1: Start the Backend Server

In your first terminal window, start the local SpacetimeDB server:

```sh
# Terminal 1
axonotes server dev
```

**Smart Features:**
- Auto-builds SpacetimeDB CLI if missing
- Auto-generates keys if missing
- Runs in-memory by default (use `--persist` for persistent database)
- Use `--auth` to force authentication (you can't publish modules when authentication is enabled)

### Step 2: Publish Backend Modules

In a second terminal, publish the backend modules. This generates the TypeScript client bindings that the frontend needs:

```sh
# Terminal 2
axonotes server publish
```

**Smart Features:**
- Auto-builds CLI if missing
- Checks server compilation before publishing
- Auto-generates TypeScript bindings
- Provides clear error messages if compilation fails

### Step 3: Start the Frontend

Once the modules are published and bindings are generated, start the SvelteKit frontend:

```sh
# Terminal 2
axonotes dashboard dev
```

**Smart Features:**
- Auto-installs dependencies if missing
- Auto-generates missing environment keys
- Auto-generates TypeScript bindings if missing
- Uses configured port from `axonotes.toml`

This will launch the SvelteKit application, typically available at `http://localhost:5173`.

### Making Changes

- **When you change backend code (`/server` directory):** Re-run the publish command to regenerate TypeScript bindings:
    ```sh
    axonotes server publish
    ```
- **When you change the core SpacetimeDB engine code (`/SpacetimeDB` subtree):** Restart the local server to rebuild the binaries:
    ```sh
    # In Terminal 1, press Ctrl+C, then:
    axonotes server dev
    ```

## 4. Configuration

The CLI uses a `axonotes.toml` configuration file in the project root. This file is automatically generated with sensible defaults:

```toml
[server]
default_mode = "memory"  # or "persistent"
auth_required = false
module_name = "axonotes"

[dashboard]
port = 5173
auto_install = true

[spacetimedb]
url = "https://testnet.spacetimedb.com"
```

You can modify these settings as needed for your development environment.

## 5. CLI Commands

The `axonotes` CLI provides smart, self-healing commands:

| Command                           | Description                                                |
| :-------------------------------- |:-----------------------------------------------------------|
| `axonotes setup`                  | Smart project setup with health checks and auto-fixes      |
| `axonotes server dev`             | Start development server (auto-builds CLI, generates keys) |
| `axonotes server dev --persist`   | Start with persistent database                             |
| `axonotes server dev --auth`      | Start with forced authentication                           |
| `axonotes server publish`         | Publish server and generate TypeScript bindings            |
| `axonotes dashboard dev`          | Start dashboard (auto-installs deps, generates bindings)   |
| `axonotes sdb build`              | Build SpacetimeDB CLI tools                                |
| `axonotes sdb login`              | Login to SpacetimeDB                                       |
| `axonotes sdb logout`             | Logout from SpacetimeDB                                    |
| `axonotes format`                 | Format all code (TypeScript, Rust)                         |
| `axonotes format --check`         | Check code formatting without applying changes             |
| `axonotes dev clean`              | Clean all build artifacts (with confirmation)              |

## 6. Legacy Package.json Scripts

Some legacy scripts are still available but will redirect you to use the CLI:

| Script              | Command               | Description                                                      |
| :------------------ | :-------------------- | :--------------------------------------------------------------- |
| **CLI Management**  | `bun run cli:install`| Build and install the CLI globally                              |
| **CLI Development** | `bun run cli:dev`     | Run CLI in development mode                                      |
| **Setup**           | `bun run setup`       | Redirects to `axonotes setup`                                    |
| **Format**          | `bun run format`      | Redirects to `axonotes format`                                   |
| **Server Logs**     | `bun run srv:logs`    | View server logs                                                 |
| **Dashboard Build** | `bun run dash:build`  | Build dashboard for production                                   |

## 7. Troubleshooting

The CLI is designed to be smart and self-healing. If you encounter issues:

1. **Run `axonotes setup`** - This will check your environment and offer to fix common issues
2. **Check the error messages** - The CLI provides clear guidance on how to fix problems
3. **Use `--verbose` flag** - Add `-v` to any command for detailed output
4. **Missing dependencies** - The CLI will detect and offer to install missing dependencies
5. **Port conflicts** - The CLI will detect port mismatches and offer to fix them

## 8. Contribution Guidelines

We welcome contributions! To ensure code quality and consistency, please follow these guidelines.

- **Automated Checks:** This repository is equipped with `husky` and `lint-staged`. Before each commit, they will automatically:
    - Format your staged code with Prettier (`.js`, `.ts`, `.svelte`, `.json`, `.md`)
    - Lint your staged code with ESLint (`.js`, `.ts`, `.svelte`)
    - Format your staged Rust code with `cargo fmt`
- **Branching:** Create a new branch for each feature or bug fix. Use a descriptive name (e.g., `feat/user-authentication` or `fix/login-button-bug`)
- **Commits:** Write clear and concise commit messages. We recommend following the [Conventional Commits](https://www.conventionalcommits.org/) specification
- **Pull Requests:** Open a Pull Request (PR) with a clear description of the changes. If your PR addresses an open issue, please link to it in the description

## 9. Project Structure

- **`/cli`**: The smart CLI tool for development tasks
- **`/dashboard`**: The SvelteKit frontend application
- **`/server`**: The Rust modules for the SpacetimeDB backend
- **`/scripts`**: Utility scripts for development tasks
- **`/SpacetimeDB`**: A git subtree of the official SpacetimeDB repository, used to build local binaries
- **`/bin`**: Contains the compiled `spacetimedb-cli` and `spacetimedb-standalone` binaries (auto-generated)
- **`axonotes.toml`**: Project configuration file (auto-generated)

---

**Happy coding! 🚀**