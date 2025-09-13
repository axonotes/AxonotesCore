import {json} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {rateLimiter} from "$lib/server/rate-limiter";
import {
    deviceAuthorizeSchema,
    parseAndValidateForm,
} from "$lib/server/validation-schemas";

export async function POST({request}) {
    try {
        // 1. Parse and validate form data with Zod
        const validation = await parseAndValidateForm(
            request,
            deviceAuthorizeSchema
        );
        if (!validation.success) {
            return json(
                {
                    error: "invalid_request",
                    error_description: validation.error,
                },
                {status: 400}
            );
        }

        const {client_id, scope, code_challenge} = validation.data;

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

        // 3. Initiate device flow
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
