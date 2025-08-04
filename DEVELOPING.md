# axonotes Core: Developer Setup Guide

This guide provides instructions for developers to set up the environment, run the project, and contribute to the
`axonotes Core` monorepo.

## 1. Prerequisites

Before you begin, ensure you have the following tools installed on your system.

- **Rust Toolchain:** Required for the SpacetimeDB backend and CLI. You can install it using `rustup`.
    - [Install Rust](https://www.rust-lang.org/tools/install)
- **Node.js and Bun:** Required for the SvelteKit frontend, Tauri desktop app, scripts, and dependency management. We use `bun` as the
  package manager and runtime.
    - [Install Node.js](https://nodejs.org/)
    - [Install Bun](https://bun.sh/docs/installation)
- **Tauri Prerequisites:** Required for the desktop application. Follow the official Tauri v2 setup guide for your operating system.
    - [Tauri v2 Prerequisites](https://v2.tauri.app/start/prerequisites/)

## 2. Initial Project Setup

Follow these steps to get the project configured on your local machine.

1. **Clone the Repository**
   Clone the `axonotesCore` repository to your local machine.

    ```sh
    git clone https://github.com/axonotes/AxonotesCore.git
    cd AxonotesCore
    ```

2. **Install Root Dependencies**
   Install the root-level development dependencies, which include tools like `prettier`, `eslint`, `husky`, and most importantly `@axonotes/axogen`.

    ```sh
    bun install
    ```

3. **Create Environment Configuration**
   Create a `.env.axogen` file in your project root. This file will contain your local environment variables:

    ```sh
    touch .env.axogen
    ```

    **Important:** Add `.env.axogen` to your `.gitignore` file if it's not already there! It's just like any other `.env` file - you don't want to push your secrets to git.

4. **Configure WorkOS**

    **WorkOS Setup for Testing:**

    1. Create a free WorkOS account: https://workos.com/signin
    2. Get your credentials from the dashboard:
        - Navigate to "API Keys" section
        - Copy your Client ID and API Key (starts with `sk_`)
    3. Configure redirect URI:
        - Go to "Redirects" section in dashboard
        - Add: `http://localhost:5173/auth/callback`
    4. Fill out your `.env.axogen` file with your credentials:
        ```env
        WORKOS_CLIENT_ID="your_client_id_here"
        WORKOS_API_KEY="your_api_key_here"
        ```

    **Note:** The setup process will generate other required keys automatically. Only WorkOS credentials need manual setup.

5. **Setup The Project**
   Run the automated setup process, which will install dependencies for all sub-projects, build required tools, and generate configuration files:

    ```sh
    bunx @axonotes/axogen run setup
    ```

6. **Log in to SpacetimeDB**
   Before you can publish backend modules, authenticate with SpacetimeDB:

    ```sh
    bunx @axonotes/axogen run sdb login
    ```

    This will open a browser window for you to log in or create a SpacetimeDB account.

7. **Generate Required Files**
   As a final step, generate all files with axogen:

    ```sh
    bunx @axonotes/axogen generate
    ```

## 3. Running the Development Environment

The development workflow involves running a local server, publishing your modules to generate client code, and then
running the frontend applications.

### Step 1: Start the Backend Server

In your first terminal window, start the local SpacetimeDB server:

```sh
# Terminal 1
bunx @axonotes/axogen run sdb server dev --in-memory
```

### Step 2: Publish Backend Modules

In a second terminal, publish the backend modules. This generates the TypeScript client bindings that the frontend
needs:

```sh
# Terminal 2
bunx @axonotes/axogen run sdb server publish
```

### Step 3: Start the Frontend Applications

Once the modules are published and bindings are generated, you can start either or both frontend applications:

**Dashboard (SvelteKit web app):**

```sh
# Terminal 2 or 3
bunx @axonotes/axogen run dash dev
```

**Desktop App (Tauri):**

```sh
# Terminal 3 or 4
bunx @axonotes/axogen run app dev
```

### Making Changes

- **When you change backend code (`/server` directory):** Re-run the publish command to regenerate TypeScript bindings:
    ```sh
    bunx @axonotes/axogen run sdb server publish
    ```
- **When you change the core SpacetimeDB engine code (`/SpacetimeDB` subtree):** Restart the local server to rebuild the
  binaries:
    ```sh
    # In Terminal 1, press Ctrl+C, then:
    bunx @axonotes/axogen run sdb server dev --in-memory
    ```

You can modify these settings as needed for your development environment.

### Axogen

If you need help understanding axogen, look at the [docs](https://axonotes.github.io/axogen/docs/intro). Axogen is our configuration management tool that handles environment variables, builds, and common development tasks.

## 4. Project Structure

- **`/dashboard`**: SvelteKit web application for the dashboard interface
- **`/app`**: Tauri desktop application built with SvelteKit frontend
- **`/server`**: SpacetimeDB backend modules written in Rust
- **`/SpacetimeDB`**: Git subtree of the SpacetimeDB engine (for custom modifications)
