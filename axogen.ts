import {
  cmd,
  defineConfig,
  json,
  liveExec,
  loadFile,
  template,
  unsafe,
} from "@axonotes/axogen";
import * as z from "zod";

// Schema for OAuth configuration validation
const OAuthSchema = z.object({
  oauth: z.object({
    client_id: z.string().min(1, "Client ID is required"),
    authorization_url: z.url("Must be a valid URL"),
    token_url: z.url("Must be a valid URL"),
    redirect_uri: z.url("Must be a valid URL"),
    callback_port: z
      .number()
      .min(1024)
      .max(65535, "Port must be between 1024-65535"),
    provider: z.string().min(1, "Provider is required"),
    scopes: z.array(z.string()).optional(),
  }),
  app: z
    .object({
      name: z.string().min(1),
      version: z.string().regex(/^\d+\.\d+\.\d+$/, "Must be semver format"),
      environment: z.enum(["development", "staging", "production"]).optional(),
    })
    .optional(),
});

const config = loadFile("config.toml", "toml", OAuthSchema);

export default defineConfig({
  targets: {
    rust_config: template({
      path: "axonotes-app/src-tauri/src/config.rs",
      template: "axonotes-app/src-tauri/templates/config.rs.njk",
      engine: "nunjucks",
      schema: OAuthSchema,
      variables: config,
      backup: true,
      generate_meta: true,
    }),
  },
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
        await liveExec("cargo fmt", {
          cwd: "./axonotes-app/src-tauri",
          outputPrefix: "CARGO",
        });
      },
    }),
  },
});
