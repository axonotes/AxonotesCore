import {env} from "@axonotes/axogen";
import * as url from "node:url";
import {envVars} from "../env";

export const dashboard = env({
    path: "./dashboard/.env",
    variables: {
        // JWT
        JWT_PRIVATE_KEY_BASE64: envVars.JWT_PRIVATE_KEY_BASE64,
        JWT_PUBLIC_KEY_BASE64: envVars.JWT_PUBLIC_KEY_BASE64,
        JWT_KEY_ID: envVars.JWT_KEY_ID,
        JWT_ISSUER: envVars.DASHBOARD_ORIGIN,

        // Cookie Names
        PUBLIC_ACCESS_TOKEN_COOKIE_NAME: "axonotes_auth_access_token",
        REFRESH_TOKEN_COOKIE_NAME: "axonotes_auth_refresh_token",
        WORKOS_SESSION_ID_COOKIE_NAME: "workos_session_id",

        // Token Limits
        TOKEN_EXCHANGE_MAX_REQUESTS: 15,
        TOKEN_EXCHANGE_WEIGHT: 2,
        TOKEN_EXCHANGE_WINDOW_MS: 60000,
        TOKEN_REFRESH_MAX_REQUESTS: 30,
        TOKEN_REFRESH_WEIGHT: 1,
        TOKEN_REFRESH_WINDOW_MS: 60000,
        WEB_ACCESS_TOKEN_EXPIRES_IN: "15m",
        WEB_REFRESH_TOKEN_EXPIRES_IN: "7d",

        // WorkOS
        WORKOS_API_KEY: envVars.WORKOS_API_KEY,
        WORKOS_CLIENT_ID: envVars.WORKOS_CLIENT_ID,
        WORKOS_REDIRECT_URI: url.resolve(
            envVars.DASHBOARD_ORIGIN,
            envVars.WORKOS_REDIRECT_PATHNAME
        ),

        // Desktop
        DESKTOP_ACCESS_TOKEN_EXPIRES_IN: "1h",
        DESKTOP_AUTH_CALLBACK_URL: url.resolve(
            envVars.DASHBOARD_ORIGIN,
            envVars.DESKTOP_AUTH_CALLBACK_PATHNAME
        ),
        DESKTOP_AUTH_SESSION_TTL: "300",
        DESKTOP_REFRESH_TOKEN_EXPIRES_IN: "30d",

        // Rate-Limits
        RATE_LIMIT_CACHE_TTL: 3600000,
        RATE_LIMIT_CHECK_PERIOD: 60,
        DEVICE_CODE_TTL_MS: 600000,
        DEVICE_FLOW_MAX_REQUESTS: 15,
        DEVICE_FLOW_WEIGHT: 3,
        DEVICE_FLOW_WINDOW_MS: 60000,
        DEVICE_POLL_INTERVAL: 5,
        DEVICE_POLL_MAX_REQUESTS: 15,
        DEVICE_POLL_WEIGHT: 1,
        DEVICE_POLL_WINDOW_MS: 60000,

        // Spacetime Server
        PUBLIC_SPACETIME_MODULE_NAME: envVars.SPACETIME_MODULE_NAME,
        PUBLIC_SPACETIME_WS: envVars.PUBLIC_SPACETIME_WS,
    },
});
