import { json, error, type RequestHandler } from '@sveltejs/kit';
import {jwtCache} from "../../../../../auth";

export const GET: RequestHandler = async ({ params }) => {
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

    // If the cachedValue is not undefined and not "PENDING",
    // it should be the actual token string.
    if (typeof cachedValue === 'string') {
        console.log(
            `[API /api/auth/token] Token found for request_id ${requestId}.`,
        );

        jwtCache.del(requestId);
        console.log(`[API /api/auth/token] Token for request_id ${requestId} deleted from cache after retrieval.`);

        return json({ status: 'success', token: cachedValue });
    }

    // This case should ideally not be reached if your cache only stores
    // "PENDING" or the actual token string for a given request_id.
    console.error(
        `[API /api/auth/token] Unexpected value type in cache for request_id ${requestId}:`,
        typeof cachedValue,
    );
    throw error(500, 'Internal server error: Unexpected token state in cache.');
};
