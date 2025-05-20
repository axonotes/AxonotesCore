import { json, error, type RequestHandler } from '@sveltejs/kit';
import { jwtCache } from '../../../../../auth';
import { createHash } from 'crypto';
import { Buffer } from 'buffer';
import type {CachedState} from "../../../../../hooks.server";

// Helper for TextEncoder
const textEncoder = new TextEncoder();

// Helper for Base64URL encoding (Node.js)
function base64UrlEncodeNode(input: Uint8Array | Buffer): string {
    return Buffer.from(input).toString('base64url');
}

// GET Handler: Poll for authentication status
export const GET: RequestHandler = async ({ params }) => {
    const { requestId } = params;

    if (!requestId) {
        console.warn('[API GET /api/auth/token] Request ID missing in path.');
        throw error(400, 'Request ID is required.');
    }

    console.log(
        `[API GET /api/auth/token] Checking status for request_id: ${requestId}`,
    );

    const cachedValue = jwtCache.get(requestId) as CachedState | undefined;

    if (!cachedValue) {
        console.warn(
            `[API GET /api/auth/token] No cached value found for request_id ${requestId}.`,
        );
        return json({
            status: 'pending',
            message: 'Request ID not found or expired.',
        });
    }

    if (cachedValue.type === 'PENDING') {
        console.log(
            `[API GET /api/auth/token] Status for request_id ${requestId} is PENDING_USER_AUTHENTICATION.`,
        );
        return json({
            status: 'pending_user_authentication',
            message: 'User authentication is pending. Please continue login process.',
        });
    }

    if (cachedValue.type === 'USER_AUTH_COMPLETED') {
        console.log(
            `[API GET /api/auth/token] Status for request_id ${requestId} is READY_FOR_TOKEN_EXCHANGE.`,
        );
        return json({
            status: 'ready_for_token_exchange',
            message: 'User authentication complete. Client can now request token via POST.',
        });
    }

    console.error(
        `[API GET /api/auth/token] Unexpected cache state for request_id ${requestId}:`,
        cachedValue,
    );
    return json(
        { status: 'error', message: 'Unexpected request state.' },
        { status: 500 },
    );
};

// POST Handler: Exchange code_verifier for JWT
export const POST: RequestHandler = async ({ params, request }) => {
    const { requestId } = params;

    if (!requestId) {
        console.warn('[API POST /api/auth/token] Request ID missing in path.');
        throw error(400, 'Request ID is required.');
    }

    let requestBody;
    try {
        requestBody = await request.json();
    } catch (e) {
        console.warn(`[API POST /api/auth/token] Invalid JSON body for ${requestId}.`);
        throw error(400, 'Invalid request body: Must be JSON.');
    }

    const { code_verifier } = requestBody;

    if (!code_verifier) {
        console.warn(`[API POST /api/auth/token] Missing code_verifier for ${requestId}.`);
        throw error(400, 'code_verifier is required.');
    }

    console.log(
        `[API POST /api/auth/token] Attempting token exchange for request_id: ${requestId}`,
    );

    const cachedValue = jwtCache.get(requestId) as CachedState | undefined;

    if (
        !cachedValue ||
        cachedValue.type !== 'USER_AUTH_COMPLETED' ||
        !cachedValue.jwtToken
    ) {
        let message = 'Request ID not found, expired, or not in a valid state for token exchange.';
        if (cachedValue && cachedValue.type !== 'USER_AUTH_COMPLETED') message = 'User authentication not completed for this request ID.';

        console.warn(`[API POST /api/auth/token] Invalid state for ${requestId}: ${message}`);
        if (cachedValue) jwtCache.del(requestId); // Clean up
        throw error(403, message); // Forbidden or Bad Request
    }

    // Verify code_verifier (PKCE Check)
    try {
        const receivedCodeVerifierBytes = textEncoder.encode(code_verifier);
        const hash = createHash('sha256').update(receivedCodeVerifierBytes).digest();
        const expectedCodeChallenge = base64UrlEncodeNode(hash);

        if (expectedCodeChallenge !== cachedValue.codeChallenge) {
            console.warn(
                `[API POST /api/auth/token] PKCE verification failed for ${requestId}. code_verifier did not match stored code_challenge.`,
            );
            throw error(400, 'Invalid code_verifier.');
        }
        console.log(`[API POST /api/auth/token] PKCE verification successful for ${requestId}.`);

    } catch (err: any) {
        if (err.status) throw err; // Re-throw SvelteKit errors
        console.error(
            `[API POST /api/auth/token] Error during PKCE verification for ${requestId}:`,
            err.message,
        );
        throw error(500, 'PKCE verification failed.');
    }

    // Issue JWT
    try {
        jwtCache.del(requestId); // Invalidate request_id after successful token issuance
        console.log(
            `[API POST /api/auth/token] request_id ${requestId} invalidated from cache.`,
        );

        return json({
            status: 'success',
            token_type: 'Bearer',
            access_token: cachedValue.jwtToken,
        });
    } catch (err: any) {
        console.error(
            `[API POST /api/auth/token] Error generating JWT (${requestId}):`,
            err,
        );
        throw error(500, 'Failed to generate authentication token.');
    }
};
