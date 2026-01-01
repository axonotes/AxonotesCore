import {cmd, liveExec} from "@axonotes/axogen";
import {generateRustBindings, publishSpacetimeModule} from "./shared/stdb";
import {throwIfToolMissing} from "../utils/tool-detection";

export const devCmd = cmd({
  help: "Start the Axonotes development server",
  exec: async () => {
    await throwIfToolMissing("Bun", "bun", "--version", "https://bun.sh/");
    await throwIfToolMissing(
      "Cargo",
      "cargo",
      "--version",
      "https://rustup.rs/"
    );

    // Re-generate Rust bindings before starting dev server
    await generateRustBindings();

    const isLinux = process.platform === "linux";
    const command = `${isLinux ? "WEBKIT_DISABLE_DMABUF_RENDERER=1 " : ""}bun run tauri dev`;
    await liveExec(command, {
      cwd: "./axonotes-app",
      outputPrefix: "DEV",
    });
  },
});
