import {cmd, defineConfig, env, group, liveExec} from "@axonotes/axogen";

const DEV = true;

const LOCAL_SERVER_URI = "ws://localhost:3000";
const MAINCLOUD_SERVER_URI = "https://maincloud.spacetimedb.com"

const SERVER_MODULE_NAME = "axonotes-demo-simple-write-sync";
const SERVER_URI = DEV ? LOCAL_SERVER_URI : MAINCLOUD_SERVER_URI;

export default defineConfig({
    targets: {
        app_env: env({
            path: "./app/.env",
            variables: {
                PUBLIC_SPACETIME_SERVER_URI: SERVER_URI,
                PUBLIC_SPACETIME_SERVER_MODULE_NAME: SERVER_MODULE_NAME,
            }
        })
    },
    commands: {
        server: group({
            help: "SpacetimeDB Server commands",
            commands: {
                start: "spacetime start",
                publish: cmd({
                    help: "Publish and Generate",
                    exec: async () => {
                        if (DEV) {
                            await liveExec(`spacetime publish --project-path server ${SERVER_MODULE_NAME}`)
                        } else {
                            await liveExec(`spacetime publish -s maincloud --project-path server ${SERVER_MODULE_NAME}`)
                        }
                        await liveExec("spacetime generate --lang typescript --out-dir app/src/lib/module_bindings --project-path server")
                    },
                }),
            }
        }),
        app: group({
            help: "App commands",
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
            }
        })
    }
})