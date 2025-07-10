import {json} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
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
        const rateLimit = await rateLimiter.checkLimit(clientId, "device_flow");

        if (!rateLimit.allowed) {
            return json(
                {
                    error: "rate_limit_exceeded",
                    error_description:
                        "Too many requests. Please try again later.",
                    retry_after: rateLimit.retryAfter,
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

        // 4. Extract and validate parameters
        const client_id = formData.get("client_id")?.toString();
        const scope =
            formData.get("scope")?.toString() || "openid profile email";
        const code_challenge = formData.get("code_challenge")?.toString();
        const code_challenge_method = formData
            .get("code_challenge_method")
            ?.toString();

        // Validate field lengths
        const fieldValidations = [
            InputValidator.validateFieldLength(client_id, "client_id"),
            InputValidator.validateFieldLength(scope, "scope"),
            InputValidator.validateFieldLength(
                code_challenge,
                "code_challenge"
            ),
            InputValidator.validateFieldLength(
                code_challenge_method,
                "code_challenge_method"
            ),
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

        // 5. Validate required parameters - Both must be present, or both missing
        const hasChallenge = !!code_challenge;
        const hasMethod = !!code_challenge_method;

        if (hasChallenge !== hasMethod) {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "code_challenge and code_challenge_method must both be present or both be absent",
                },
                {status: 400}
            );
        }

        if (!code_challenge || !code_challenge_method) {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "Missing required parameters: code_challenge, code_challenge_method",
                },
                {status: 400}
            );
        }

        if (code_challenge_method !== "S256") {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "Only S256 code_challenge_method is supported",
                },
                {status: 400}
            );
        }

        // 6. Validate code_challenge format (base64url)
        if (!/^[A-Za-z0-9_-]{43,128}$/.test(code_challenge)) {
            return json(
                {
                    error: "invalid_request",
                    error_description:
                        "Invalid code_challenge format. Must be 43-128 characters, base64url encoded.",
                },
                {status: 400}
            );
        }

        // 7. Initiate device flow
        const response = await desktopAuthManager.initiateDeviceFlow(
            code_challenge,
            client_id,
            scope
        );

        return json(response, {status: 200});
    } catch (error) {
        console.error("Device authorization error:", error);

        // Don't leak error details to the client
        return json(
            {
                error: "server_error",
                error_description: "Internal server error",
            },
            {status: 500}
        );
    }
}

export async function OPTIONS() {
    return new Response(null, {
        status: 200,
        headers: {
            "Access-Control-Allow-Origin": "*",
            "Access-Control-Allow-Methods": "POST, OPTIONS",
            "Access-Control-Allow-Headers": "Content-Type, Authorization",
        },
    });
}
