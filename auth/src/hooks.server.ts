import {handle as authHandle, jwtCache, generateSpacetimeDBToken} from './auth';
import {sequence} from '@sveltejs/kit/hooks';
import {error, type Handle, redirect} from '@sveltejs/kit';
import {dev} from '$app/environment';
import {
    JWT_CACHE_TTL,
    CLIENT_ED25519_PUBLIC_KEY_B64URL,
} from '$env/static/private';
import nacl from 'tweetnacl';
import {Buffer} from 'buffer';

export interface CachedState {
    type: 'PENDING' | 'USER_AUTH_COMPLETED';
    codeChallenge: string;
    jwtToken?: string; // Only present in USER_AUTH_COMPLETED
}

// Helper for TextEncoder
const textEncoder = new TextEncoder();

function importClientPublicKeyTweetNaCl(base64UrlKey: string): Uint8Array {
    if (!base64UrlKey) {
        throw new Error('Client Ed25519 public key (B64URL) is not configured.');
    }
    try {
        const keyBytes = Buffer.from(base64UrlKey, 'base64url');
        if (keyBytes.length !== nacl.sign.publicKeyLength) {
            throw new Error(
                `Invalid public key length. Expected ${nacl.sign.publicKeyLength}, got ${keyBytes.length}`,
            );
        }
        return new Uint8Array(keyBytes); // tweetnacl expects Uint8Array
    } catch (e: any) {
        console.error(
            '[Hook importClientPublicKeyTweetNaCl] Error decoding public key:',
            e.message,
        );
        throw new Error('Failed to decode client public key.');
    }
}

// Hook 1: Handle initial request_id, code_challenge, client_signature and set cookie
const handleInitiation: Handle = async ({event, resolve}) => {
    const {url, cookies} = event;

    if (url.pathname === '/auth/initiate') {
        const requestId = url.searchParams.get('rid');
        const codeChallenge = url.searchParams.get('ch');
        const clientSignatureBase64Url = url.searchParams.get('cs');

        if (!requestId || !codeChallenge || !clientSignatureBase64Url) {
            console.warn(
                '[Hook handleInitiation] /auth/initiate called with missing parameters (rid, ch, or cs).',
            );
            throw error(400, 'Missing required authentication parameters.');
        }

        // Verify client_signature using tweetnacl
        try {
            if (!CLIENT_ED25519_PUBLIC_KEY_B64URL) {
                console.error(
                    '[Hook handleInitiation] Server configuration error: CLIENT_ED25519_PUBLIC_KEY_B64URL is not set.',
                );
                throw error(500, 'Server configuration error.');
            }

            const clientPublicKeyBytes = importClientPublicKeyTweetNaCl(CLIENT_ED25519_PUBLIC_KEY_B64URL);
            const signatureBytes = Buffer.from(clientSignatureBase64Url, 'base64url');
            const dataToVerifyBytes = textEncoder.encode(codeChallenge);

            if (signatureBytes.length !== nacl.sign.signatureLength) {
                console.warn(
                    `[Hook handleInitiation] Invalid signature length for request_id ${requestId}. Expected ${nacl.sign.signatureLength}, got ${signatureBytes.length}`,
                );
                throw error(400, 'Invalid signature format.');
            }

            const isValid = nacl.sign.detached.verify(
                dataToVerifyBytes,
                new Uint8Array(signatureBytes), // Ensure signature is Uint8Array
                clientPublicKeyBytes,
            );

            if (!isValid) {
                console.warn(
                    `[Hook handleInitiation] Invalid client signature for request_id ${requestId} (tweetnacl).`,
                );
                throw error(401, 'Invalid client signature.');
            }
            console.log(
                `[Hook handleInitiation] Client signature verified for request_id ${requestId} (tweetnacl).`,
            );

        } catch (err: any) {
            if (err.status) throw err; // Re-throw SvelteKit errors
            console.error(
                `[Hook handleInitiation] Error during signature verification for ${requestId} (tweetnacl):`,
                err.message,
            );
            throw error(500, 'Signature verification failed.');
        }

        // Store state if signature is valid
        const ttlSeconds = parseInt(JWT_CACHE_TTL || '300');
        const cacheEntry: CachedState = {
            type: 'PENDING',
            codeChallenge: codeChallenge,
        };

        const cacheSuccess = jwtCache.set(requestId, cacheEntry, ttlSeconds);
        if (!cacheSuccess) {
            console.error(
                `[Hook handleInitiation] Failed to cache initiated state for request_id ${requestId}.`,
            );
            throw error(500, 'Failed to store request state.');
        }
        console.log(
            `[Hook handleInitiation] /auth/initiate: Stored initiated state for ${requestId}. Setting cookie.`,
        );

        cookies.set('app.request-id', requestId, {
            path: '/',
            httpOnly: true,
            secure: !dev,
            sameSite: 'lax',
            maxAge: ttlSeconds,
        });

        throw redirect(302, '/signin'); // Proceed to user login
    }
    return resolve(event);
};

// Hook 2: Handle the linking after Auth.js authentication
const handleCompleteLink: Handle = async ({event, resolve}) => {
    if (event.url.pathname === '/auth/complete-link') {
        console.log('[Hook handleCompleteLink] Intercepted /auth/complete-link');

        const session = await event.locals.auth();
        if (!session?.user) {
            console.warn('[Hook handleCompleteLink] Unauthorized: No active session.');
            throw redirect(302, '/signin?error=SessionExpiredForLink');
        }

        const requestId = event.cookies.get('app.request-id');
        if (!requestId) {
            console.warn(
                `[Hook handleCompleteLink] Missing app.request-id cookie for user ${session.user.email}.`,
            );
            throw redirect(302, '/error?code=LINK_ID_MISSING');
        }

        console.log(
            `[Hook handleCompleteLink] User ${session.user.email} authenticated. Found request_id: ${requestId}`,
        );

        try {
            const userIdForSpacetime = session.user.id || session.user.email;
            if (!userIdForSpacetime) {
                console.error(
                    `[Hook handleCompleteLink] User ID/email missing in session for user ${session.user.email}.`,
                );
                throw error(500, 'User identifier not found in session.');
            }

            const spacetimeToken = await generateSpacetimeDBToken(userIdForSpacetime);
            const currentState = jwtCache.get(requestId) as CachedState | undefined;
            if (!currentState) {
                console.error(
                    `[Hook handleCompleteLink] No cache entry found for request_id ${requestId}.`,
                );
                throw error(404, 'Request ID not found.');
            }

            const cacheEntry: CachedState = {
                type: 'USER_AUTH_COMPLETED',
                codeChallenge: currentState.codeChallenge,
                jwtToken: spacetimeToken,
            }

            const cacheSuccess = jwtCache.set(requestId, cacheEntry);

            if (!cacheSuccess) {
                console.error(
                    `[Hook handleCompleteLink] Failed to cache SpacetimeDB token for request_id ${requestId}.`,
                );
                throw error(500, 'Failed to store authentication token.');
            }
            console.log(
                `[Hook handleCompleteLink] SpacetimeDB token cached for request_id ${requestId}.`,
            );

            event.cookies.delete('app.request-id', {path: '/', httpOnly: true, secure: !dev, sameSite: 'lax'});
            console.log(
                `[Hook handleCompleteLink] Deleted app.request-id cookie for ${requestId}.`,
            );
        } catch (err: any) {
            console.error(
                `[Hook handleCompleteLink] Error processing link for request_id ${requestId}:`,
                err,
            );
            // Redirect to a generic error page
            const errorCode = err.status || 500;
            throw redirect(302, `/error?code=${errorCode}&message=${encodeURIComponent(err.body?.message || err.message || 'Unknown error')}`);
        }

        // Redirect to the final success page
        console.log('[Hook handleCompleteLink] Linking successful. Redirecting to /success.');
        throw redirect(302, '/success');
    }
    // If not /auth/complete-link, pass to the next handler
    return resolve(event);
};

const notFoundHandler: Handle = async ({event, resolve}) => {
    const response = await resolve(event);

    if (response.status === 404 || event.url.pathname === '/') {
        console.error(`[Hook notFoundHandler] 404 Not Found: ${event.url.pathname}`);
        throw redirect(302, '/error?code=NOT_FOUND');
    }

    return response;
};

export const handle = sequence(handleInitiation, authHandle, handleCompleteLink, notFoundHandler);