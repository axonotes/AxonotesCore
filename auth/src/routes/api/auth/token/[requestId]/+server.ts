import { json, error, type RequestHandler } from '@sveltejs/kit';
import {jwtCache} from "../../../../../auth";
import * as jose from 'jose';

let jwksClient: ReturnType<typeof jose.createRemoteJWKSet> | null = null;

function getJWKSClient(origin: string): ReturnType<typeof jose.createRemoteJWKSet> {
    if (!jwksClient) {
        const jwksUrl = new URL('/.well-known/jwks.json', origin);
        console.log(`[API /api/auth/token] Initializing JWKS client for: ${jwksUrl.href}`);
        jwksClient = jose.createRemoteJWKSet(jwksUrl, {
            cacheMaxAge: 600000, // 10 minutes in ms
            timeoutDuration: 5000, // 5 seconds timeout for fetching
        });
    }
    return jwksClient;
}

export const GET: RequestHandler = async ({ params, url }) => {
    const { requestId } = params;

    if (!requestId) {
        console.warn('[API /api/auth/token] Request ID missing in path.');
        throw error(400, 'Request ID is required.');
    }

    console.log(
        `[API /api/auth/token] Checking token for request_id: ${requestId}`,
    );

    const cachedValue = jwtCache.get(requestId);

    if (cachedValue === undefined) {
        // The requestId was not found in the cache.
        // This could mean it's invalid, expired, or the initiation step failed.
        console.log(
            `[API /api/auth/token] No cache entry found for request_id: ${requestId}. Might be expired or invalid.`,
        );
        return json(
            { status: 'not_found', message: 'Request ID not found or expired.' },
            { status: 404 },
        );
    }

    if (cachedValue === 'PENDING') {
        // The authentication process is still ongoing for this requestId.
        console.log(
            `[API /api/auth/token] Token for request_id ${requestId} is still PENDING.`,
        );
        return json({
            status: 'pending',
            message: 'Authentication is pending. Please try again shortly.',
        });
    }

    // --- Verification Step using jose and JWKS ---
    if (typeof cachedValue === 'string') {
        const token = cachedValue;
        console.log(
            `[API /api/auth/token] Token string found for request_id ${requestId}. Verifying with jose via JWKS...`,
        );

        try {
            // Get the JWKS client, ensuring it's initialized with the correct origin
            const currentJwksClient = getJWKSClient(url.origin);

            const { payload, protectedHeader } = await jose.jwtVerify(
                token,
                currentJwksClient, // Pass the remote JWKSet function
                {
                    // Optional: If you want to enforce a specific algorithm beyond what the key in JWKS specifies
                    // algorithms: SPACETIME_JWT_ALGORITHM ? [SPACETIME_JWT_ALGORITHM] : undefined,
                    //
                    // Optional: If your token has specific issuer/audience, add them here for validation:
                    // issuer: 'YOUR_EXPECTED_ISSUER',
                    // audience: 'YOUR_EXPECTED_AUDIENCE',
                },
            );

            // `jose` will automatically use the `kid` from the JWT header to select the correct key from the JWKS.
            // It also verifies the `alg` from the JWT header against the `alg` of the selected key.

            console.log(
                `[API /api/auth/token] Token for request_id ${requestId} verified successfully via JWKS. Subject:`,
                payload.sub,
                `Algorithm: ${protectedHeader.alg}, Key ID (kid): ${protectedHeader.kid}`,
            );

            jwtCache.del(requestId);

            return json({ status: 'success', token: token });
        } catch (err: any) {
            console.error(
                `[API /api/auth/token] jose token verification via JWKS failed for request_id ${requestId}: [${err.code || 'UNKNOWN_JOSE_ERROR'}] ${err.message}`,
            );

            // Common jose error codes for JWKS/verification:
            // ERR_JWKS_NO_MATCHING_KEY, ERR_JWT_SIGNATURE_VERIFICATION_FAILED, ERR_JWT_EXPIRED,
            // ERR_JOSE_ALG_NOT_WHITELISTED (if algorithms option is used and doesn't match)
            // ERR_JWKS_TIMEOUT, ERR_JWKS_FETCH_FAILED
            throw error(
                500,
                `Token verification failed: ${err.code || 'VerificationError'}`,
            );
        }
    }
    // --- End Verification Step ---

    // This case should ideally not be reached if your cache only stores
    // "PENDING" or the actual token string for a given request_id.
    console.error(
        `[API /api/auth/token] Unexpected value type in cache for request_id ${requestId}:`,
        typeof cachedValue,
    );
    throw error(500, 'Internal server error: Unexpected token state in cache.');
};
