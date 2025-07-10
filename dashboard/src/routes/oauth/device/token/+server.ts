import {json} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {rateLimiter} from "$lib/server/rate-limiter";
import {InputValidator} from "$lib/server/input-validator";

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
        const device_code = formData.get("device_code")?.toString();
        const code_verifier = formData.get("code_verifier")?.toString();
        const client_id = formData.get("client_id")?.toString();

        // Validate field lengths
        const fieldValidations = [
            InputValidator.validateFieldLength(grant_type, "grant_type"),
            InputValidator.validateFieldLength(device_code, "device_code"),
            InputValidator.validateFieldLength(code_verifier, "code_verifier"),
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
        if (grant_type !== "urn:ietf:params:oauth:grant-type:device_code") {
            return json(
                {
                    error: "unsupported_grant_type",
                    error_description:
                        "Only device_code grant type is supported",
                },
                {status: 400}
            );
        }

        // Validate required parameters
        const requiredParams = {device_code, code_verifier};
        const paramValidation =
            InputValidator.validateRequiredParams(requiredParams);
        if (!paramValidation.valid) {
            return json(
                {
                    error: "invalid_request",
                    error_description: paramValidation.error,
                },
                {status: 400}
            );
        }

        // Exchange device code for tokens
        const tokenResponse = await desktopAuthManager.exchangeDeviceToken(
            device_code!,
            code_verifier!
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
