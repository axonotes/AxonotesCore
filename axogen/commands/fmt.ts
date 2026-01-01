import {cmd, liveExec} from "@axonotes/axogen";
import {throwIfToolMissing} from "../utils/tool-detection";

export const fmtCmd = cmd({
  help: "Format the Axonotes codebase",
  exec: async () => {
    await throwIfToolMissing(
      "ESLint",
      "eslint",
      "--version",
      "https://eslint.org/docs/latest/use/getting-started"
    );
    await throwIfToolMissing(
      "Prettier",
      "prettier",
      "--version",
      "https://prettier.io/docs/install.html"
    );
    await throwIfToolMissing(
      "Cargo",
      "cargo",
      "--version",
      "https://rustup.rs/"
    );

    await liveExec("eslint --fix .", {
      outputPrefix: "ES-LINT",
    });
    await liveExec("prettier -w .", {
      outputPrefix: "PRETTIER",
    });
    await liveExec("cargo fmt", {
      cwd: "./axonotes-app/src-tauri",
      outputPrefix: "CARGO",
    });
    await liveExec("cargo fmt", {
      cwd: "./axonotes-stdb",
      outputPrefix: "CARGO-STDB",
    });
  },
});
