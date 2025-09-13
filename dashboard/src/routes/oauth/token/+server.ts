import {json} from "@sveltejs/kit";
import {verifyToken, getTokenLifetimes, generateTokens} from "$lib/server/jwt";
import {rateLimiter} from "$lib/server/rate-limiter";
import {
    refreshTokenRequestSchema,
    parseAndValidateForm,
} from "$lib/server/validation-schemas";

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
                {status: 400}
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
                {status: 429}
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
                {status: 400}
            );
        }

        // Ensure client type is set to desktop!!
        tokenPayload.client_type = "desktop";

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
            {status: 200}
        );
    } catch (error) {
        console.error("Token refresh error:", error);
        return json(
            {
                error: "server_error",
                error_description: "Internal server error",
            },
            {status: 500}
        );
    }
}
