import {cmd, liveExec} from "@axonotes/axogen";
import {generateRustBindings} from "./shared/stdb";
import {throwIfToolMissing} from "../utils/tool-detection";

export const dev2Cmd = cmd({
  help: "Start a second Axonotes instance with separate data directory (for multi-user testing)",
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
    const command = `${isLinux ? "WEBKIT_DISABLE_DMABUF_RENDERER=1 " : ""}bun run tauri dev --config "src-tauri/tauri.dev2.conf.json"`;
    await liveExec(command, {
      cwd: "./axonotes-app",
      outputPrefix: "DEV2",
    });
  },
});
