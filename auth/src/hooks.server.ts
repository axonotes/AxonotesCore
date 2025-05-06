import {handle as authHandle, jwtCache, generateSpacetimeDBToken} from './auth';
import {sequence} from '@sveltejs/kit/hooks';
import {error, type Handle, redirect} from '@sveltejs/kit';
import {dev} from '$app/environment';
import {JWT_CACHE_TTL} from '$env/static/private';

// Hook 1: Handle initial request_id and set cookie
const handleInitiation: Handle = async ({event, resolve}) => {
    const {url, cookies} = event;

    if (url.pathname === '/auth/initiate') {
        const requestId = url.searchParams.get('request_id');
        if (requestId) {
            console.log(
                `[Hook handleInitiation] /auth/initiate: Setting app.request-id cookie for ${requestId}`,
            );
            cookies.set('app.request-id', requestId, {
                path: '/',
                httpOnly: true,
                secure: !dev,
                sameSite: 'lax',
                maxAge: parseInt(JWT_CACHE_TTL || '300'),
            });

            throw redirect(302, '/signin');
        } else {
            console.warn('[Hook handleInitiation] /auth/initiate called without request_id.');
        }
    }
    return resolve(event); // Pass to next handler
};

// Hook 2: Handle the linking after Auth.js authentication
const handleCompleteLink: Handle = async ({event, resolve}) => {
    if (event.url.pathname === '/auth/complete-link') {
        console.log('[Hook handleCompleteLink] Intercepted /auth/complete-link');

        // 1. Check Auth.js session (populated by authHandle which runs before this)
        const session = await event.locals.auth();
        if (!session?.user) {
            console.warn('[Hook handleCompleteLink] Unauthorized: No active session.');
            // Redirect to sign-in or an error page
            throw redirect(302, '/signin?error=SessionExpiredForLink');
        }

        // 2. Get the request_id from the cookie
        const requestId = event.cookies.get('app.request-id');
        if (!requestId) {
            console.warn(
                `[Hook handleCompleteLink] Missing app.request-id cookie for user ${session.user.email}.`,
            );
            // Redirect to an error page or inform the user
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
            const cacheSuccess = jwtCache.set(requestId, spacetimeToken);

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
}

// IMPORTANT: Hook Order
// 1. handleInitiation: Sets the cookie if it's the /auth/initiate path.
// 2. authHandle: Handles all Auth.js specific routes (/signin, /auth/callback/*, etc.)
//    AND populates `event.locals.getSession()` for subsequent handlers.
// 3. handleCompleteLink: Acts on /auth/complete-link, relying on the session from authHandle
//    and the cookie from handleInitiation.
export const handle = sequence(handleInitiation, authHandle, handleCompleteLink, notFoundHandler);