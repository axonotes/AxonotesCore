import {cmd, liveExec} from "@axonotes/axogen";

export const testCmd = cmd({
  help: "Run all unit tests",
  exec: async () => {
    await liveExec("cargo test --all", {
      outputPrefix: "TAURI",
      cwd: "axonotes-app/src-tauri",
    });
  },
});
