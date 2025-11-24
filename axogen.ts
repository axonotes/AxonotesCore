import {cmd, defineConfig, liveExec} from "@axonotes/axogen";

export default defineConfig({
  commands: {
    install: cmd({
      help: "Install Axonotes dependencies",
      exec: async () => {
        await liveExec("bun install", {cwd: "./axonotes-app"});
      },
    }),
    dev: cmd({
      help: "Start the Axonotes development server",
      exec: async () => {
        const isLinux = process.platform === "linux";
        const command = `${isLinux ? "__NV_DISABLE_EXPLICIT_SYNC=1 " : ""}bun run tauri dev`;
        await liveExec(command, {
          cwd: "./axonotes-app",
          outputPrefix: "DEV",
        });
      },
    }),
    format: cmd({
      help: "Format the Axonotes codebase",
      exec: async () => {
        await liveExec("eslint --fix .", {
          outputPrefix: "ES-LINT",
        });
        await liveExec("prettier -w .", {
          outputPrefix: "PRETTIER",
        });
      },
    }),
  },
});
