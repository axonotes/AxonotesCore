import {json} from "@sveltejs/kit";
import {
    verifyToken,
    getTokenLifetimes, generateTokens,
} from "$lib/server/jwt";
import {rateLimiter} from "$lib/server/rate-limiter";
import {InputValidator} from "$lib/server/input-validator";

export async function POST({request}) {
    try {
        // 1. Input validation
        const validation = await InputValidator.validateFormData(request);
        if (!validation.valid) {
            return json(
                {
                    error: "invalid_request",
                    error_description: validation.error,
                },
                {status: 400}
            );
        }

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

        // 3. Parse form data with error handling
        let formData: FormData;
        try {
            formData = await request.formData();
        } catch (error) {
            return json(
                {
                    error: "invalid_request",
                    error_description: "Invalid form data",
                },
                {status: 400}
            );
        }

        const grant_type = formData.get("grant_type")?.toString();
        const refresh_token = formData.get("refresh_token")?.toString();
        const client_id = formData.get("client_id")?.toString();

        // Validate field lengths
        const fieldValidations = [
            InputValidator.validateFieldLength(grant_type, "grant_type"),
            InputValidator.validateFieldLength(refresh_token, "refresh_token"),
            InputValidator.validateFieldLength(client_id, "client_id"),
        ];

        for (const validation of fieldValidations) {
            if (!validation.valid) {
                return json(
                    {
                        error: "invalid_request",
                        error_description: validation.error,
                    },
                    {status: 400}
                );
            }
        }

        // Validate grant type
        if (grant_type !== "refresh_token") {
            return json(
                {
                    error: "unsupported_grant_type",
                    error_description:
                        "Only refresh_token grant type is supported",
                },
                {status: 400}
            );
        }

        // Validate required parameters
        if (!refresh_token) {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "Missing required parameter: refresh_token",
                },
                {status: 400}
            );
        }

        // Verify refresh token
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

        // Generate new tokens
        const {accessToken, refreshToken: newRefreshToken} = generateTokens(tokenPayload);
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
