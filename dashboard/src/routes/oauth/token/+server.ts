import {json} from "@sveltejs/kit";
import {verifyToken, getTokenLifetimes, generateTokens} from "$lib/server/jwt";
import {rateLimiter} from "$lib/server/rate-limiter";
import {
    refreshTokenRequestSchema,
    parseAndValidateForm,
} from "$lib/server/validation-schemas";

const noStore = {"Cache-Control": "no-store", Pragma: "no-cache"};

/**
 * Handle OAuth2 refresh-token requests (POST).
 *
 * Validates the incoming form for a refresh token, enforces per-client rate limits,
 * verifies the provided refresh token (must be a desktop client token), and
 * issues a new access token and refresh token on success.
 *
 * Returns JSON responses using OAuth2-style error fields for failure cases:
 * - 400 when the request is invalid or the refresh token is invalid/expired or client_type mismatches
 * - 429 when the client is rate-limited (includes `Retry-After` header)
 * - 500 on internal server errors
 *
 * On success returns 200 with the access token payload:
 * {
 *   access_token: string,
 *   token_type: "Bearer",
 *   expires_in: number,        // seconds until access token expiry
 *   refresh_token: string,     // newly issued refresh token
 *   scope: "openid profile email"
 * }
 *
 * Responses are returned with no-store caching headers to prevent token caching.
 *
 * @returns A SvelteKit JSON response containing either the token response or an OAuth2-style error object.
 */
export async function POST({request}) {
    try {
        // 1. Parse and validate form data with Zod
        const validation = await parseAndValidateForm(
            request,
            refreshTokenRequestSchema
        );
        if (!validation.success) {
            // Map Zod errors to OAuth2 errors
            let oauthError = "invalid_request";
            if (validation.error.includes("Grant type")) {
                oauthError = "unsupported_grant_type";
            }

            return json(
                {
                    error: oauthError,
                    error_description: validation.error,
                },
                {status: 400, headers: noStore}
            );
        }

        const {refresh_token} = validation.data;

        // 2. Rate limiting
        const clientId = rateLimiter.getClientIdentifier(request);
        const rateLimit = await rateLimiter.checkLimit(
            clientId,
            "token_refresh"
        );

        if (!rateLimit.allowed) {
            return json(
                {
                    error: "rate_limit_exceeded",
                    error_description:
                        "Too many requests. Please try again later.",
                },
                {
                    status: 429,
                    headers: {
                        ...noStore,
                        "Retry-After": String(rateLimit.retryAfter ?? 60),
                    },
                }
            );
        }

        // 3. Verify refresh token
        const tokenPayload = verifyToken(refresh_token);

        if (!tokenPayload) {
            return json(
                {
                    error: "invalid_grant",
                    error_description: "Invalid or expired refresh token",
                },
                {status: 400, headers: noStore}
            );
        }

        // Enforce desktop-only refresh tokens
        if (tokenPayload.client_type !== "desktop") {
            return json(
                {
                    error: "invalid_grant",
                    error_description: "Refresh token client_type mismatch",
                },
                {status: 400, headers: noStore}
            );
        }

        // 4. Generate new tokens
        const {accessToken, refreshToken: newRefreshToken} =
            generateTokens(tokenPayload);
        const {accessTokenMs} = getTokenLifetimes(true);

        return json(
            {
                access_token: accessToken,
                token_type: "Bearer",
                expires_in: Math.floor(accessTokenMs / 1000),
                refresh_token: newRefreshToken,
                scope: "openid profile email",
            },
            {status: 200, headers: noStore}
        );
    } catch (error) {
        console.error("Token refresh error:", error);
        return json(
            {
                error: "server_error",
                error_description: "Internal server error",
            },
            {status: 500, headers: noStore}
        );
    }
}
