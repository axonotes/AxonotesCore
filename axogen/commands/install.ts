import {cmd, liveExec} from "@axonotes/axogen";
import {throwIfToolMissing} from "../utils/tool-detection";

export const installCmd = cmd({
  help: "Install Axonotes dependencies",
  exec: async () => {
    await throwIfToolMissing("Bun", "bun", "--version", "https://bun.sh/");

    await liveExec("bun install", {cwd: "./axonotes-app"});
  },
});
