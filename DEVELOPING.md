Of course! Thank you for the corrections and additional details. Here is the updated developer setup guide incorporating your feedback.

---

# AxonotesCore: Developer Setup Guide

This guide provides instructions for developers to set up the environment, run the project, and contribute to the `AxonotesCore` monorepo.

## 1. Prerequisites

Before you begin, ensure you have the following tools installed on your system.

- **Rust Toolchain:** Required for the SpaceTimeDB backend. You can install it using `rustup`.
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
    Clone the `AxonotesCore` repository to your local machine.

    ```sh
    git clone https://github.com/axonotes/AxonotesCore.git
    cd AxonotesCore
    ```

2.  **Install Root Dependencies**
    Install the root-level development dependencies, which include tools like `prettier`, `eslint`, and `husky`.

    ```sh
    bun install
    ```

3.  **Run the Project Setup Script**
    This repository includes a convenient setup script that handles several one-time setup tasks.

    ```sh
    bun run setup
    ```

    This command will:

    - Install dependencies for the utility scripts located in `/scripts`.
    - Generate necessary local keys for development.
    - Install all dependencies for the SvelteKit frontend located in `/dashboard`.

4.  **Log in to SpaceTimeDB (First-Time Users)**
    Before you can publish the backend modules, you need to authenticate with SpaceTimeDB. This is a one-time action.

    ```sh
    bun run sdb:login
    ```

    This will open a browser window for you to log in or create a SpaceTimeDB account.

## 3. Running the Development Environment

The development workflow involves running a local server, publishing your modules to generate client code, and then running the frontend application.

### Step 1: Start the Backend Server

In your first terminal window, start the local SpaceTimeDB server. This server runs in-memory and is used for local development.

```sh
# In Terminal 1
bun run srv:dev
```

### Step 2: Publish Backend Modules

In a second terminal, publish the backend modules from the `/server` directory. This step is crucial as it also **generates the TypeScript client bindings** that the frontend needs to communicate with the backend.

```sh
# In Terminal 2
bun run srv:publish
```

### Step 3: Start the Frontend

Once the modules are published and bindings are generated, you can start the SvelteKit frontend development server in the same terminal.

```sh
# In Terminal 2
bun run dash:dev
```

This will launch the SvelteKit application, typically available at `http://localhost:5173`.

### Making Changes

- **When you change backend code (`/server` directory):** You must re-run the publish command to regenerate the TypeScript bindings.
    ```sh
    bun run srv:publish
    ```
- **When you change the core SpaceTimeDB engine code (`/SpacetimeDB` subtree):** You must stop and restart the local server to rebuild the binaries.
    ```sh
    # In Terminal 1, press Ctrl+C, then:
    bun run srv:dev
    ```

## 4. Contribution Guidelines

We welcome contributions! To ensure code quality and consistency, please follow these guidelines.

- **Automated Checks:** This repository is equipped with `husky` and `lint-staged`. Before each commit, they will automatically:
    - Format your staged code with Prettier (`.js`, `.ts`, `.svelte`, `.json`, `.md`).
    - Lint your staged code with ESLint (`.js`, `.ts`, `.svelte`).
    - Format your staged Rust code with `cargo fmt`.
- **Branching:** Create a new branch for each feature or bug fix. Use a descriptive name (e.g., `feat/user-authentication` or `fix/login-button-bug`).
- **Commits:** Write clear and concise commit messages. We recommend following the [Conventional Commits](https://www.conventionalcommits.org/) specification.
- **Pull Requests:** Open a Pull Request (PR) with a clear description of the changes. If your PR addresses an open issue, please link to it in the description.

## 5. Useful Scripts

The `package.json` file contains several scripts to help with development.

| Script              | Command               | Description                                                      |
| :------------------ | :-------------------- | :--------------------------------------------------------------- |
| **Setup**           | `bun run setup`       | Runs the initial one-time setup for keys and dependencies.       |
| **Login to SDB**    | `bun run sdb:login`   | Authenticates your machine with SpaceTimeDB.                     |
| **Backend Dev**     | `bun run srv:dev`     | Builds and starts the SpaceTimeDB backend for local development. |
| **Publish Backend** | `bun run srv:publish` | Publishes backend modules and generates TS client bindings.      |
| **Frontend Dev**    | `bun run dash:dev`    | Starts the SvelteKit frontend development server.                |
| **Format Code**     | `bun run format`      | Manually formats all project files using Prettier.               |
| **Lint Code**       | `bun run lint:fix`    | Manually lints and fixes all project files.                      |

## 6. Project Structure

- **`/dashboard`**: The SvelteKit frontend application.
- **`/server`**: The Rust modules for the SpaceTimeDB backend.
- **`/scripts`**: Utility scripts for development tasks.
- **`/SpacetimeDB`**: A git subtree of the official SpaceTimeDB repository, used to build local binaries.
- **`/bin`**: Contains the compiled `spacetimedb-cli` and `spacetimedb-standalone` binaries (auto-generated).
