import {
    defineConfig,
    createTypedEnv,
    command,
    liveExec,
} from "@axonotes/axogen";
import {z} from "zod";
import {generateKeyPairSync} from "node:crypto";
import {readFileSync, writeFileSync, existsSync} from "node:fs";
import {resolve} from "node:path";

// Generate JWT keys if missing
const jwtKeys = generateJwtKeys();

// Environment configuration
const env = createTypedEnv({
    // Environment selection
    AXONOTES_ENV: z.enum(["local", "staging", "production"]).default("local"),

    // WorkOS Authentication
    WORKOS_CLIENT_ID: z.string(),
    WORKOS_API_KEY: z.string(),

    // JWT Configuration (auto-generated)
    JWT_PRIVATE_KEY_BASE64: z.string(),
    JWT_PUBLIC_KEY_BASE64: z.string(),
    JWT_KEY_ID: z.string(),
});

// Environment-specific configuration
const getConfig = () => {
    const baseConfig = {
        dashboardPort: "5173",
        spacetimePort: "3000",
        moduleName: "axonotes",
    };

    switch (env.AXONOTES_ENV) {
        case "local":
            return {
                ...baseConfig,
                dashboardOrigin: `http://localhost:${baseConfig.dashboardPort}`,
                spacetimeEndpoint: `ws://localhost:${baseConfig.spacetimePort}`,
            };
        case "staging":
            return {
                ...baseConfig,
                dashboardOrigin: "https://staging.axonotes.com",
                spacetimeEndpoint: "wss://staging-api.axonotes.com",
            };
        case "production":
            return {
                ...baseConfig,
                dashboardOrigin: "https://axonotes.com",
                spacetimeEndpoint: "wss://api.axonotes.com",
            };
        default:
            throw new Error(`Unknown environment: ${env.AXONOTES_ENV}`);
    }
};

const config = getConfig();

export default defineConfig({
    targets: {
        dashboard: {
            path: "dashboard/.env",
            type: "env",
            variables: {
                // WorkOS Authentication
                WORKOS_CLIENT_ID: env.WORKOS_CLIENT_ID || "",
                WORKOS_API_KEY: env.WORKOS_API_KEY || "",
                WORKOS_REDIRECT_URI: `${config.dashboardOrigin}/auth/callback`,
                WORKOS_SESSION_ID_COOKIE_NAME: "workos_session_id",

                // JWT Configuration
                JWT_PRIVATE_KEY_BASE64:
                    env.JWT_PRIVATE_KEY_BASE64 || jwtKeys.privateKey,
                JWT_PUBLIC_KEY_BASE64:
                    env.JWT_PUBLIC_KEY_BASE64 || jwtKeys.publicKey,
                JWT_KEY_ID: env.JWT_KEY_ID || jwtKeys.keyId,
                JWT_ISSUER: config.dashboardOrigin,

                // Token Configuration
                REFRESH_TOKEN_COOKIE_NAME: "axonotes_auth_refresh_token",
                PUBLIC_ACCESS_TOKEN_COOKIE_NAME: "axonotes_auth_access_token",
                WEB_REFRESH_TOKEN_EXPIRES_IN: "7d",
                WEB_ACCESS_TOKEN_EXPIRES_IN: "15m",

                // SpacetimeDB Configuration
                PUBLIC_SPACETIME_WS: config.spacetimeEndpoint,
                PUBLIC_SPACETIME_MODULE_NAME: config.moduleName,

                // Desktop Auth Configuration
                DESKTOP_AUTH_SESSION_TTL: "300",
                DESKTOP_AUTH_CALLBACK_URL: `${config.dashboardOrigin}/auth/desktop/callback`,
                DESKTOP_REFRESH_TOKEN_EXPIRES_IN: "30d",
                DESKTOP_ACCESS_TOKEN_EXPIRES_IN: "1h",

                // Device Flow Configuration
                DEVICE_CODE_TTL_MS: (10 * 60 * 1000).toString(), // 10 minutes
                DEVICE_POLL_INTERVAL: "5", // 5 seconds

                // Rate Limiting Configuration
                RATE_LIMIT_CACHE_TTL: (60 * 60 * 1000).toString(), // 1 hour
                RATE_LIMIT_CHECK_PERIOD: "60", // 60 seconds

                // Device Flow Rate Limits
                DEVICE_FLOW_WINDOW_MS: (60 * 1000).toString(), // 1 minute
                DEVICE_FLOW_MAX_REQUESTS: "15",
                DEVICE_FLOW_WEIGHT: "3",

                // Device Poll Rate Limits
                DEVICE_POLL_WINDOW_MS: (60 * 1000).toString(), // 1 minute
                DEVICE_POLL_MAX_REQUESTS: "15",
                DEVICE_POLL_WEIGHT: "1",

                // Token Exchange Rate Limits
                TOKEN_EXCHANGE_WINDOW_MS: (60 * 1000).toString(), // 1 minute
                TOKEN_EXCHANGE_MAX_REQUESTS: "15",
                TOKEN_EXCHANGE_WEIGHT: "2",

                // Token Refresh Rate Limits
                TOKEN_REFRESH_WINDOW_MS: (60 * 1000).toString(), // 1 minute
                TOKEN_REFRESH_MAX_REQUESTS: "30",
                TOKEN_REFRESH_WEIGHT: "1",
            },
        },
    },

    commands: {
        format: command.string(
            "prettier --write .",
            "Format all code using Prettier"
        ),
        "format:check": command.string(
            "prettier --check .",
            "Check code formatting with Prettier"
        ),
        lint: command.string("eslint .", "Lint code with ESLint"),
        "lint:fix": command.string(
            "eslint . --fix",
            "Lint and fix code with ESLint"
        ),

        // Configuration Management
        "config:generate": command.define({
            help: "Generate configuration files",
            exec: async () => {
                console.log("🔧 Generating configuration files...");
                // Axogen handles this automatically when this command runs
            },
        }),

        "config:validate": command.define({
            help: "Validate configuration files",
            exec: async () => {
                console.log("🔍 Validating configuration...");
                // Axogen handles validation automatically
                console.log("✅ Configuration is valid");
            },
        }),

        "config:init": command.define({
            help: "Initialize configuration setup",
            exec: async () => {
                console.log("🎯 Initializing configuration...");
                // Generate keys and initial setup
                generateJwtKeys();
                console.log("✅ Configuration initialized");
            },
        }),

        // SpacetimeDB Server Operations
        "srv:dev": command.define({
            help: "Start SpacetimeDB server in development mode",
            exec: async () => {
                // Build CLI first (equivalent to presrv:dev)
                console.log("🔨 Building CLI first...");
                const buildResult = await liveExec("axogen run sdb:build:cli", {
                    outputPrefix: "BUILD",
                });

                if (buildResult.exitCode !== 0) {
                    console.error("❌ Build CLI failed");
                    return;
                }

                console.log("🚀 Starting SpacetimeDB server...");
                const devCommand =
                    env.AXONOTES_ENV === "production"
                        ? "./bin/spacetimedb-cli start --in-memory --allowed-oidc-issuer http://localhost:5173 --allowed-oidc-issuer https://auth.spacetimedb.com --auth-required"
                        : "./bin/spacetimedb-cli start --in-memory --allowed-oidc-issuer http://localhost:5173 --allowed-oidc-issuer https://auth.spacetimedb.com";

                const result = await liveExec(devCommand, {
                    outputPrefix: "SDB",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Server dev failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Server was stopped by user");
                } else {
                    console.log("✅ Server completed successfully");
                }
            },
        }),

        "srv:publish": command.define({
            help: "Publish SpacetimeDB module and generate TypeScript bindings",
            exec: async () => {
                const publishCmd = `./bin/spacetimedb-cli publish --project-path server -c ${config.moduleName}`;

                console.log("📦 Publishing SpacetimeDB module...");
                const publishResult = await liveExec(publishCmd, {
                    outputPrefix: "PUBLISH",
                });

                if (publishResult.exitCode !== 0) {
                    console.error("❌ Publish failed");
                    return;
                }

                console.log("🔧 Generating TypeScript bindings...");
                const generateResult = await liveExec(
                    "./bin/spacetimedb-cli generate --lang typescript --out-dir dashboard/src/lib/module_bindings --project-path server",
                    {
                        outputPrefix: "GENERATE",
                    }
                );

                if (generateResult.exitCode !== 0) {
                    console.error("❌ Generate bindings failed");
                } else {
                    console.log(
                        "✅ Module published and bindings generated successfully"
                    );
                }
            },
        }),

        "srv:format": command.define({
            help: "Format server Rust code",
            exec: async () => {
                console.log("🎨 Formatting server Rust code...");
                const result = await liveExec("cd server && cargo fmt --all", {
                    outputPrefix: "FORMAT",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Server format failed");
                } else {
                    console.log("✅ Server code formatted successfully");
                }
            },
        }),

        "srv:clear": command.define({
            help: "Build CLI and clear SpacetimeDB server",
            exec: async () => {
                // Build CLI first (equivalent to presrv:clear)
                console.log("🔨 Building CLI first...");
                const buildResult = await liveExec("axogen run sdb:build:cli", {
                    outputPrefix: "BUILD",
                });

                if (buildResult.exitCode !== 0) {
                    console.error("❌ Build CLI failed");
                    return;
                }

                console.log("🧹 Clearing SpacetimeDB server...");
                const result = await liveExec(
                    "./bin/spacetimedb-cli server clear",
                    {
                        outputPrefix: "CLEAR",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Server clear failed");
                } else {
                    console.log("✅ Server cleared successfully");
                }
            },
        }),

        "srv:logs": command.define({
            help: "Show SpacetimeDB logs",
            exec: async (ctx) => {
                console.log("📋 Showing SpacetimeDB logs...");
                const result = await liveExec(
                    `./bin/spacetimedb-cli logs ${config.moduleName}`,
                    {
                        outputPrefix: "LOGS",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Failed to fetch logs");
                } else if (result.wasTerminated) {
                    console.log("🔵 Log viewing stopped by user");
                }
            },
        }),

        // SpacetimeDB CLI Management
        "sdb:build:cli": command.define({
            help: "Build SpacetimeDB CLI",
            exec: async () => {
                console.log("🔨 Building SpacetimeDB CLI...");
                const result = await liveExec(
                    "cd SpacetimeDB && cargo build --release --bin spacetimedb-cli",
                    {
                        outputPrefix: "CARGO",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ SDB CLI build failed");
                } else {
                    console.log("✅ SDB CLI built successfully");
                }
            },
        }),

        "sdb:logout": command.define({
            help: "Logout from SpacetimeDB CLI",
            exec: async () => {
                console.log("🔐 Logging out from SpacetimeDB CLI...");
                const result = await liveExec("./bin/spacetimedb-cli logout", {
                    outputPrefix: "LOGOUT",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ SDB logout failed");
                } else {
                    console.log("✅ Successfully logged out from SDB CLI");
                }
            },
        }),

        "sdb:format": command.define({
            help: "Format SpacetimeDB source code",
            exec: async () => {
                console.log("🎨 Formatting SpacetimeDB source code...");
                const result = await liveExec(
                    "cd SpacetimeDB && cargo fmt --all",
                    {
                        outputPrefix: "FORMAT",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ SDB format failed");
                } else {
                    console.log("✅ SDB source code formatted successfully");
                }
            },
        }),

        // Git Subtree Management
        "sdb:pull-upstream": command.define({
            help: "Pull upstream SpacetimeDB changes",
            exec: async () => {
                console.log("⬇️ Pulling upstream SpacetimeDB changes...");
                const result = await liveExec(
                    "git subtree pull --prefix=SpacetimeDB https://github.com/clockworklabs/SpacetimeDB.git master --squash",
                    {
                        outputPrefix: "GIT",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Pull upstream failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Pull operation stopped by user");
                } else {
                    console.log("✅ Successfully pulled upstream changes");
                }
            },
        }),

        "sdb:push-fork": command.define({
            help: "Push changes to SpacetimeDB fork",
            exec: async () => {
                console.log("⬆️ Pushing changes to SpacetimeDB fork...");
                const result = await liveExec(
                    "git subtree push --prefix=SpacetimeDB https://github.com/axonotes/SpacetimeDB.git master",
                    {
                        outputPrefix: "GIT",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Push fork failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Push operation stopped by user");
                } else {
                    console.log("✅ Successfully pushed to fork");
                }
            },
        }),

        "sdb:pull-fork": command.define({
            help: "Pull changes from SpacetimeDB fork",
            exec: async () => {
                console.log("⬇️ Pulling changes from SpacetimeDB fork...");
                const result = await liveExec(
                    "git subtree pull --prefix=SpacetimeDB https://github.com/axonotes/SpacetimeDB.git master --squash",
                    {
                        outputPrefix: "GIT",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Pull fork failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Pull operation stopped by user");
                } else {
                    console.log("✅ Successfully pulled from fork");
                }
            },
        }),

        // SvelteKit Dashboard
        "dash:install": command.define({
            help: "Install dashboard dependencies",
            exec: async () => {
                console.log("📦 Installing dashboard dependencies...");
                const result = await liveExec("cd dashboard && bun install", {
                    outputPrefix: "BUN",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Dashboard install failed");
                } else {
                    console.log(
                        "✅ Dashboard dependencies installed successfully"
                    );
                }
            },
        }),

        "dash:dev": command.define({
            help: "Start dashboard development server",
            exec: async () => {
                console.log("🚀 Starting dashboard development server...");
                const result = await liveExec("cd dashboard && bun run dev", {
                    outputPrefix: "SVELTE",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Dashboard dev failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Dashboard dev server stopped by user");
                } else {
                    console.log("✅ Dashboard dev server completed");
                }
            },
        }),

        "dash:build": command.define({
            help: "Build dashboard for production",
            exec: async () => {
                console.log("🏗️ Building dashboard for production...");
                const result = await liveExec("cd dashboard && bun run build", {
                    outputPrefix: "BUILD",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Dashboard build failed");
                } else {
                    console.log("✅ Dashboard built successfully");
                }
            },
        }),

        "dash:preview": command.define({
            help: "Preview dashboard build",
            exec: async () => {
                console.log("👀 Starting dashboard preview server...");
                const result = await liveExec(
                    "cd dashboard && bun run preview",
                    {
                        outputPrefix: "PREVIEW",
                    }
                );

                if (result.exitCode !== 0) {
                    console.error("❌ Dashboard preview failed");
                } else if (result.wasTerminated) {
                    console.log("🔵 Dashboard preview stopped by user");
                } else {
                    console.log("✅ Dashboard preview completed");
                }
            },
        }),

        "app:install": command.define({
            help: "Install Axonotes application dependencies",
            exec: async () => {
                console.log(
                    "📦 Installing Axonotes application dependencies..."
                );
                const result = await liveExec("cd app && bun install", {
                    outputPrefix: "BUN",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Application install failed");
                } else {
                    console.log(
                        "✅ Application dependencies installed successfully"
                    );
                }
            },
        }),

        "app:dev": command.define({
            help: "Start Axonotes application in development mode",
            exec: async () => {
                console.log(
                    "🚀 Starting Axonotes application in development mode..."
                );
                const result = await liveExec("cd app && bun run tauri dev", {
                    outputPrefix: "AXONOTES",
                });

                if (result.exitCode !== 0) {
                    console.error("❌ Application dev failed");
                } else if (result.wasTerminated) {
                    console.log(
                        "🔵 Application development server stopped by user"
                    );
                } else {
                    console.log("✅ Application development server completed");
                }
            },
        }),
    },
});

interface JWTKeys {
    privateKey: string;
    publicKey: string;
    keyId: string;
}

function generateJwtKeys(): JWTKeys {
    const envAxogenPath = resolve(".env.axogen");

    // First, check if keys already exist in .env.axogen
    if (existsSync(envAxogenPath)) {
        const envContent = readFileSync(envAxogenPath, "utf-8");

        const privateKeyMatch = envContent.match(
            /^JWT_PRIVATE_KEY_BASE64\s*=\s*"?([^"\n\r]+)"?\s*$/m
        );
        const publicKeyMatch = envContent.match(
            /^JWT_PUBLIC_KEY_BASE64\s*=\s*"?([^"\n\r]+)"?\s*$/m
        );
        const keyIdMatch = envContent.match(
            /^JWT_KEY_ID\s*=\s*"?([^"\n\r]+)"?\s*$/m
        );

        if (privateKeyMatch && publicKeyMatch && keyIdMatch) {
            return {
                privateKey: privateKeyMatch[1],
                publicKey: publicKeyMatch[1],
                keyId: keyIdMatch[1],
            };
        }
    }

    // Generate new keys if they don't exist
    console.log("🔑 Generating new ECDSA (ES256) key pair...");

    const {privateKey, publicKey} = generateKeyPairSync("ec", {
        namedCurve: "prime256v1",
        publicKeyEncoding: {type: "spki", format: "pem"},
        privateKeyEncoding: {type: "pkcs8", format: "pem"},
    });

    const privateKeyBase64 = Buffer.from(privateKey).toString("base64");
    const publicKeyBase64 = Buffer.from(publicKey).toString("base64");
    const keyId = `axonotes-key-${new Date().toISOString().slice(0, 10)}`;

    const keys = {
        privateKey: privateKeyBase64,
        publicKey: publicKeyBase64,
        keyId,
    };

    // Update .env.axogen file
    updateEnvAxogenFile(keys);

    console.log("   ✅ JWT keys generated and saved to .env.axogen");

    return keys;
}

function updateEnvAxogenFile(keys: JWTKeys): void {
    const envAxogenPath = resolve(".env.axogen");

    if (existsSync(envAxogenPath)) {
        let envContent = readFileSync(envAxogenPath, "utf-8");

        const keyEntries = [
            {key: "JWT_PRIVATE_KEY_BASE64", value: keys.privateKey},
            {key: "JWT_PUBLIC_KEY_BASE64", value: keys.publicKey},
            {key: "JWT_KEY_ID", value: keys.keyId},
        ];

        let hasUpdates = false;

        for (const {key, value} of keyEntries) {
            const regex = new RegExp(`^(${key}=)(.*)$`, "m");
            if (regex.test(envContent)) {
                envContent = envContent.replace(regex, `$1"${value}"`);
                hasUpdates = true;
            } else {
                if (!hasUpdates) {
                    envContent += "\n\n# JWT Keys (auto-generated)\n";
                }
                envContent += `${key}="${value}"\n`;
                hasUpdates = true;
            }
        }

        if (hasUpdates) {
            writeFileSync(envAxogenPath, envContent);
        }
    } else {
        // Create new .env.axogen file
        const envContent = `# .env.axogen - Add this to .gitignore!

# Environment Configuration
AXONOTES_ENV=local

# WorkOS Configuration
WORKOS_CLIENT_ID=""
WORKOS_API_KEY=""

# JWT Keys (auto-generated)
JWT_PRIVATE_KEY_BASE64="${keys.privateKey}"
JWT_PUBLIC_KEY_BASE64="${keys.publicKey}"
JWT_KEY_ID="${keys.keyId}"
`;

        writeFileSync(envAxogenPath, envContent);
        console.log("📄 Created new .env.axogen file with JWT keys");
    }
}
