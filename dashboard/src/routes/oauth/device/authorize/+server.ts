import {json} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {rateLimiter} from "$lib/server/rate-limiter";
import {
    deviceAuthorizeSchema,
    parseAndValidateForm,
} from "$lib/server/validation-schemas";

/**
 * Handles POST requests for the OAuth 2.0 device authorization endpoint.
 *
 * Validates the incoming form against `deviceAuthorizeSchema`, enforces a per-client
 * rate limit for the device flow, and on success initiates a device authorization
 * flow via the desktop auth manager. Responds with a JSON payload and appropriate
 * HTTP status code:
 *
 * - 200: device flow initiation succeeded; body is the authorization response from
 *   the desktop auth manager.
 * - 400: request validation failed; body contains `error: "invalid_request"` and
 *   `error_description`.
 * - 429: client exceeded the "device_flow" rate limit; body contains
 *   `error: "rate_limit_exceeded"`, `error_description`, and `retry_after`.
 * - 500: internal server error; body contains `error: "server_error"` and a
 *   non-revealing `error_description`.
 *
 * @returns A SvelteKit JSON response containing either the device authorization
 * result or a structured OAuth2 error object with an appropriate HTTP status.
 */
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

/**
 * Responds to CORS preflight requests for the device authorization endpoint.
 *
 * Returns a 200 response with CORS headers that allow POST and OPTIONS methods and the
 * Content-Type and Authorization request headers from any origin.
 *
 * @returns A Response with status 200 and the following headers:
 * - Access-Control-Allow-Origin: "*"
 * - Access-Control-Allow-Methods: "POST, OPTIONS"
 * - Access-Control-Allow-Headers: "Content-Type, Authorization"
 */
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
