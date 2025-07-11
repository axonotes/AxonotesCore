import {
    defineConfig,
    env,
    generateJwtKeys,
    minutes,
    hours,
} from "./cli/src/config/functions.js";

// Generate JWT keys if they don't exist
const keys = generateJwtKeys();

// Flexible endpoint handling
const dashboardOrigin = "http://localhost:5173";
const spacetimeEndpoint = "ws://localhost:3000";
const dashboardPort = "5173";
const spacetimePort = "3000";

export default defineConfig({
    watch: [".env", "axonotes.config.ts"],

    globals: {
        dashboardOrigin,
        spacetimeEndpoint,
        dashboardPort,
        spacetimePort,
        moduleName: "axonotes",
        keys,
    },

    targets: {
        // Dashboard SvelteKit application
        dashboard: {
            type: "env",
            path: "dashboard/.env",
            variables: {
                // WorkOS Authentication
                WORKOS_CLIENT_ID: env.WORKOS_CLIENT_ID || "",
                WORKOS_API_KEY: env.WORKOS_API_KEY || "",
                WORKOS_REDIRECT_URI: `${dashboardOrigin}/auth/callback`,
                WORKOS_SESSION_ID_COOKIE_NAME: "workos_session_id",

                // JWT Configuration
                JWT_PRIVATE_KEY_BASE64: keys.privateKey,
                JWT_PUBLIC_KEY_BASE64: keys.publicKey,
                JWT_KEY_ID: keys.keyId,
                JWT_ISSUER: dashboardOrigin,

                // Token Configuration
                REFRESH_TOKEN_COOKIE_NAME: "axonotes_auth_refresh_token",
                PUBLIC_ACCESS_TOKEN_COOKIE_NAME: "axonotes_auth_access_token",
                WEB_REFRESH_TOKEN_EXPIRES_IN: "7d",
                WEB_ACCESS_TOKEN_EXPIRES_IN: "15m",

                // SpacetimeDB Configuration
                PUBLIC_SPACETIME_WS: spacetimeEndpoint,
                PUBLIC_SPACETIME_MODULE_NAME: "axonotes",

                // Desktop Auth Configuration
                DESKTOP_AUTH_SESSION_TTL: "300",
                DESKTOP_AUTH_CALLBACK_URL: `${dashboardOrigin}/auth/desktop/callback`,
                DESKTOP_REFRESH_TOKEN_EXPIRES_IN: "30d",
                DESKTOP_ACCESS_TOKEN_EXPIRES_IN: "1h",

                // Device Flow Configuration (using time utilities)
                DEVICE_CODE_TTL_MS: minutes(10), // 10 minutes
                DEVICE_POLL_INTERVAL: "5", // 5 seconds

                // Rate Limiting Configuration
                RATE_LIMIT_CACHE_TTL: hours(1), // 1 hour
                RATE_LIMIT_CHECK_PERIOD: "60", // 60 seconds

                // Device Flow Rate Limits
                DEVICE_FLOW_WINDOW_MS: minutes(1), // 1 minute
                DEVICE_FLOW_MAX_REQUESTS: "15",
                DEVICE_FLOW_WEIGHT: "3",

                // Device Poll Rate Limits
                DEVICE_POLL_WINDOW_MS: minutes(1), // 1 minute
                DEVICE_POLL_MAX_REQUESTS: "15",
                DEVICE_POLL_WEIGHT: "1",

                // Token Exchange Rate Limits
                TOKEN_EXCHANGE_WINDOW_MS: minutes(1), // 1 minute
                TOKEN_EXCHANGE_MAX_REQUESTS: "15",
                TOKEN_EXCHANGE_WEIGHT: "2",

                // Token Refresh Rate Limits
                TOKEN_REFRESH_WINDOW_MS: minutes(1), // 1 minute
                TOKEN_REFRESH_MAX_REQUESTS: "30",
                TOKEN_REFRESH_WEIGHT: "1",
            },
        },
    },
});
