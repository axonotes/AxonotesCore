import {cmd, exec, group, liveExec} from "@axonotes/axogen";
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
            exec: async () => {},
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
        app: cmd({
            help: "Generate server types to app",
            exec: generateServerTypesForApp,
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
            },
            exec: async (context) => {
                const commandOptions = [
                    context.options["in-memory"] ? "--in-memory" : "",
                ].join(" ");

                await liveExec(`spacetime start ${commandOptions}`);
            },
        }),
        generate: sdb_server_generate,
        publish: cmd({
            help: "Publish server to SpacetimeDB and generate types",
            exec: async () => {
                await liveExec(
                    `spacetime publish --project-path server ${envVars.SPACETIME_MODULE_NAME}`
                );
                await generateServerTypesForAll();
                process.exit();
            },
        }),
        logs: cmd({
            help: "Show the servers logs",
            exec: async () => {
                await liveExec(
                    `spacetime logs ${envVars.SPACETIME_MODULE_NAME}`
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
                await liveExec("spacetime login");
            },
        }),
        logout: cmd({
            help: "Logout from Spacetime",
            exec: async () => {
                await liveExec("spacetime logout");
            },
        }),
        server: sdb_server,
    },
});

const app = group({
    help: "Tauri Desktop App commands",
    commands: {
        install: "cd app && bun install",
        dev: cmd({
            help: "Run the Tauri app in development mode",
            exec: async () => {
                // check if user is on linux and set the environment variable
                const isLinux = process.platform === "linux";
                const command = `${isLinux ? "__NV_DISABLE_EXPLICIT_SYNC=1 " : ""}bun run tauri dev`;
                await liveExec(command, {
                    cwd: "./app",
                    outputPrefix: "Tauri Dev",
                });
            },
        }),
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
            // Check if tools are installed
            await checkVersion(
                "Rust",
                "1.88.x",
                "https://www.rust-lang.org/tools/install",
                "rustc --version",
                "rustc 1.88"
            );
            await checkVersion(
                "Node.js",
                "v22",
                "https://nodejs.org/en/download",
                "node --version",
                "v22"
            );
            await checkVersion(
                "Bun",
                "1.2",
                "https://bun.sh/docs/installation",
                "bun --version",
                "1.2"
            );
            await checkVersion(
                "SpacetimeDB CLI",
                "1.2",
                "https://spacetimedb.com/install",
                "spacetime -V",
                "spacetime 1.2"
            );
            await checkVersion(
                "WasmOpt",
                "123",
                "https://github.com/WebAssembly/binaryen/releases",
                "wasm-opt --version",
                "wasm-opt version 123",
                false
            );

            await liveExec("cd dashboard && bun install");
            await liveExec("cd app && bun install");
        },
    }),
};

// ---- Helper Functions ----

async function checkVersion(
    name: string,
    version: string,
    link: string,
    command: string,
    startsWith: string,
    required = true
) {
    const result = await exec(command);
    if (result.exitCode !== 0) {
        if (!required) {
            console.log(
                `Its recommended to install ${name} from ${link}. But you can continue without it.`
            );
            return;
        }
        throw Error(`Make sure to install ${name} from ${link}`);
    } else if (!result.stdout.startsWith(startsWith)) {
        throw Error(
            `Make sure to update ${name} to version ${version} from ${link}`
        );
    }
}

async function generateServerTypesForAll() {
    await generateServerTypesForDashboard();
    await generateServerTypesForApp();
}

async function generateServerTypesForDashboard() {
    await liveExec(
        "spacetime generate --lang typescript --out-dir dashboard/src/lib/module_bindings --project-path server"
    );
}

async function generateServerTypesForApp() {
    await liveExec(
        "spacetime generate --lang rust --out-dir app/src-tauri/src/module_bindings --project-path server"
    );
}
