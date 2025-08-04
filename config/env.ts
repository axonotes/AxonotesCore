import {loadEnv} from "@axonotes/axogen";
import * as z from "zod";
import {defaultKeys} from "./jwt";

export const envVars = loadEnv(
    z.object({
        DASHBOARD_ORIGIN: z.url().default("http://localhost:5173"),

        // JWT
        JWT_PRIVATE_KEY_BASE64: z
            .base64()
            .default(defaultKeys.privateKeyBase64)
            .describe("Base64 encoded private key for JWT"),
        JWT_PUBLIC_KEY_BASE64: z
            .base64()
            .default(defaultKeys.publicKeyBase64)
            .describe("Base64 encoded public key for JWT"),
        JWT_KEY_ID: z.string().default(defaultKeys.keyId),

        // WorkOS
        WORKOS_API_KEY: z.string().startsWith("sk_test_"),
        WORKOS_CLIENT_ID: z.string().startsWith("client_"),
        WORKOS_REDIRECT_PATHNAME: z.string().default("/auth/callback"),

        // Desktop
        DESKTOP_AUTH_CALLBACK_PATHNAME: z
            .string()
            .default("/auth/desktop/callback"),

        // Server
        PUBLIC_SPACETIME_WS: z.string().default("ws://localhost:3000"),
        SPACETIME_MODULE_NAME: z.string().default("axonotes"),
    })
);
