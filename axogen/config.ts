import * as z from "zod";
import {loadFile} from "@axonotes/axogen";

// Schema for OAuth configuration validation
export const OAuthSchema = z.object({
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
  stdb: z.object({
    default_host_uri: z.url().min(1),
    default_module_name: z.string().min(1),
  }),
});

export const config = loadFile("config.toml", "toml", OAuthSchema);
