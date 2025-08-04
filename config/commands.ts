import {cmd, group, liveExec} from "@axonotes/axogen";
import * as fs from "fs/promises";
import * as z from "zod";
import {envVars} from "./env";

const dash = group({
    help: "Dashboard commands",
    commands: {
        dev: "cd dashboard && bun run dev",
        build: "cd dashboard && bun run build",
        preview: "cd dashboard && bun run preview",
        install: "cd dashboard && bun install",
    },
});

const sdb_cli = group({
    help: "SpacetimeDB CLI commands",
    commands: {
        build: cmd({
            help: "Build the SpacetimeDB CLI",
            exec: async () => {
                await buildSpacetimeDBCLI();
            },
        }),
    },
});

const sdb_server_generate = group({
    help: "Generate commands",
    commands: {
        all: cmd({
            help: "Generate all project types from the server",
            exec: generateServerTypesForAll,
        }),
        dash: cmd({
            help: "Generate server types to dashboard",
            exec: generateServerTypesForDashboard,
        }),
    },
});

const sdb_server = group({
    help: "SpacetimeDB Server commands",
    commands: {
        dev: cmd({
            help: "Runs the SpacetimeDB Server",
            options: {
                "in-memory": z
                    .boolean()
                    .default(false)
                    .optional()
                    .describe(
                        "Run the database in memory and dont write to disk"
                    ),
                "auth-required": z
                    .boolean()
                    .default(false)
                    .optional()
                    .describe("Only users with existing JWTs can connect"),
            },
            exec: async (context) => {
                await checkAndBuildSpacetimeDBCLI();

                const commandOptions = [
                    `--allowed-oidc-issuer ${envVars.DASHBOARD_ORIGIN}`,
                    "--allowed-oidc-issuer https://auth.spacetimedb.com",
                    context.options["in-memory"] ? "--in-memory" : "",
                    context.options["auth-required"] ? "--auth-required" : "",
                ].join(" ");

                await liveExec(`./bin/spacetimedb-cli start ${commandOptions}`);
            },
        }),
        generate: sdb_server_generate,
        publish: cmd({
            help: "Publish server to SpacetimeDB and generate types",
            exec: async () => {
                await checkAndBuildSpacetimeDBCLI();
                await liveExec(
                    `./bin/spacetimedb-cli publish --project-path server ${envVars.SPACETIME_MODULE_NAME}`
                );
                await generateServerTypesForAll();
                process.exit();
            },
        }),
        logs: cmd({
            help: "Show the servers logs",
            exec: async () => {
                await checkAndBuildSpacetimeDBCLI();
                await liveExec(
                    `./bin/spacetimedb-cli logs ${envVars.SPACETIME_MODULE_NAME}`
                );
            },
        }),
    },
});

const sdb = group({
    help: "SpacetimeDB commands",
    commands: {
        cli: sdb_cli,
        login: cmd({
            help: "Login to Spacetime",
            exec: async () => {
                await checkAndBuildSpacetimeDBCLI();
                await liveExec("./bin/spacetimedb-cli login");
            },
        }),
        logout: cmd({
            help: "Logout from Spacetime",
            exec: async () => {
                await checkAndBuildSpacetimeDBCLI();
                await liveExec("./bin/spacetimedb-cli logout");
            },
        }),
        server: sdb_server,
    },
});

const app = group({
    help: "Tauri Desktop App commands",
    commands: {
        install: "cd app && bun install",
        dev: "cd app && bun run tauri dev",
    },
});

export const commands = {
    format: cmd({
        help: "Format everything",
        exec: async () => {
            await liveExec("bunx prettier --write .", {
                outputPrefix: "Prettier",
            });
            await liveExec("cargo fmt --all", {
                cwd: "./SpacetimeDB",
                outputPrefix: "Cargo",
            });
            await liveExec("cargo fmt --all", {
                cwd: "./server",
                outputPrefix: "Cargo",
            });
        },
    }),
    lint: "bunx eslint .",
    dash,
    sdb,
    app,
    setup: cmd({
        help: "Setup everything",
        exec: async () => {
            await checkAndBuildSpacetimeDBCLI();
            await liveExec("cd dashboard && bun install");
            await liveExec("cd app && bun install");
        },
    }),
};

// ---- Helper Functions ----

async function buildSpacetimeDBCLI() {
    try {
        await fs.mkdir("./bin");
    } catch (_e) {
        // Folder already exists
    }

    await liveExec(
        "cargo build --release -p spacetimedb-cli -p spacetimedb-standalone",
        {
            cwd: "./SpacetimeDB",
            outputPrefix: "Build",
        }
    );
    await fs.rename(
        "./SpacetimeDB/target/release/spacetimedb-cli",
        "./bin/spacetimedb-cli"
    );
    await fs.rename(
        "./SpacetimeDB/target/release/spacetimedb-standalone",
        "./bin/spacetimedb-standalone"
    );
}

async function checkAndBuildSpacetimeDBCLI() {
    // check if the SpacetimeDB CLI binary exists
    await buildSpacetimeDBCLI();
}

async function generateServerTypesForAll() {
    await generateServerTypesForDashboard();
}

async function generateServerTypesForDashboard() {
    await checkAndBuildSpacetimeDBCLI();
    await liveExec(
        "./bin/spacetimedb-cli generate --lang typescript --out-dir dashboard/src/lib/module_bindings --project-path server"
    );
}
