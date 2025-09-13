import {json} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {rateLimiter} from "$lib/server/rate-limiter";
import {
    deviceTokenSchema,
    parseAndValidateForm,
} from "$lib/server/validation-schemas";

export async function GET({url, request}) {
    try {
        // 1. Rate limiting for polling
        const clientId = rateLimiter.getClientIdentifier(request);
        const rateLimit = await rateLimiter.checkLimit(clientId, "device_poll");

        if (!rateLimit.allowed) {
            if (rateLimit.shouldSlowDown) {
                return json(
                    {
                        error: "slow_down",
                        error_description:
                            "You are polling too frequently. Please slow down.",
                    },
                    {status: 400}
                );
            }

            return json(
                {
                    error: "rate_limit_exceeded",
                    error_description:
                        "Too many requests. Please try again later.",
                },
                {status: 429}
            );
        }

        const device_code = url.searchParams.get("device_code");

        if (!device_code) {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "Missing required parameter: device_code",
                },
                {status: 400}
            );
        }

        // Validate device code format
        if (
            !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
                device_code
            )
        ) {
            return json(
                {
                    error: "invalid_request",
                    error_description: "Invalid device_code format",
                },
                {status: 400}
            );
        }

        // Poll device status
        const response = await desktopAuthManager.pollDeviceStatus(device_code);

        if (response.error) {
            return json(response, {status: 400});
        }

        return json(
            {
                status: "complete",
                message:
                    "Authorization complete. Use POST to exchange for tokens.",
            },
            {status: 200}
        );
    } catch (error) {
        console.error("Device token polling error:", error);
        return json(
            {
                error: "server_error",
                error_description: "Internal server error",
            },
            {status: 500}
        );
    }
}

export async function POST({request}) {
    try {
        // 1. Parse and validate form data with Zod
        const validation = await parseAndValidateForm(
            request,
            deviceTokenSchema
        );
        if (!validation.success) {
            // Map Zod errors to OAuth2 errors
            let oauthError = "invalid_request";
            if (validation.error.includes("grant type")) {
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

        const {grant_type, device_code, code_verifier, client_id} =
            validation.data;

        // 2. Rate limiting
        const clientId = rateLimiter.getClientIdentifier(request);
        const rateLimit = await rateLimiter.checkLimit(
            clientId,
            "token_exchange"
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

        // 3. Exchange device code for tokens
        const tokenResponse = await desktopAuthManager.exchangeDeviceToken(
            device_code,
            code_verifier
        );

        return json(tokenResponse, {status: 200});
    } catch (error) {
        console.error("Device token exchange error:", error);

        // Map internal errors to OAuth2 errors
        let oauthError = "server_error";
        let errorDescription = "Internal server error";

        if (error instanceof Error) {
            switch (true) {
                case error.message.includes("Invalid or expired device code"):
                    oauthError = "invalid_grant";
                    errorDescription = "Invalid or expired device code";
                    break;
                case error.message.includes("expired"):
                    oauthError = "expired_token";
                    errorDescription = "The device code has expired";
                    break;
                case error.message.includes("not yet complete"):
                    oauthError = "authorization_pending";
                    errorDescription =
                        "The authorization request is still pending";
                    break;
                case error.message.includes("Invalid code verifier"):
                    oauthError = "invalid_grant";
                    errorDescription = "Invalid code verifier";
                    break;
            }
        }

        return json(
            {
                error: oauthError,
                error_description: errorDescription,
            },
            {status: 400}
        );
    }
}
