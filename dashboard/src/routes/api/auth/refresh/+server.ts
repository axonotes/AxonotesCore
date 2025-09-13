import {json, error} from "@sveltejs/kit";
import {
    verifyToken,
    UserTokenPayload,
    generateTokens,
    getTokenLifetimes,
} from "$lib/server/jwt";
import {REFRESH_TOKEN_COOKIE_NAME} from "$env/static/private";

/**
 * Refreshes authentication tokens using a provided refresh token.
 *
 * Verifies a refresh token taken first from a secure cookie (REFRESH_TOKEN_COOKIE_NAME) or, as a fallback, from the request JSON body (`refresh_token`). If the token is missing the handler responds with 401. If verification fails the refresh cookie is cleared (path "/") and a 401 is returned. On success, new tokens are generated.
 *
 * - For payloads with `client_type === "web"`: returns the raw refreshed token pair as JSON.
 * - For other client types: returns an OpenID-style token response with `access_token`, `token_type`, `expires_in` (seconds), `refresh_token`, and `scope` (200).
 *
 * @throws {HttpError} 401 if the refresh token is not provided or is invalid/expired.
 * @returns A SvelteKit JSON response containing either the refreshed tokens (web clients) or an OpenID-like token object (non-web clients).
 */
export async function POST({cookies, request}) {
    let refreshToken = cookies.get(REFRESH_TOKEN_COOKIE_NAME);

    const body = await request.json();
    if (!refreshToken) {
        // Check if the request body contains a refresh token
        refreshToken = body.refresh_token || null;
    }

    console.log("Token refresh");

    if (!refreshToken) {
        throw error(401, "Refresh token not found.");
    }

    const payload = verifyToken(refreshToken);
    if (!payload) {
        // If refresh token is invalid/expired, clear it to force re-login
        cookies.delete(REFRESH_TOKEN_COOKIE_NAME, {path: "/"});
        throw error(401, "Invalid or expired refresh token.");
    }

    const accessTokenPayload: UserTokenPayload = {...payload};

    const refreshedTokens = generateTokens(accessTokenPayload);
    if (payload.client_type === "web") {
        return json(refreshedTokens);
    }

    const {accessTokenMs} = getTokenLifetimes(true);
    return json(
        {
            access_token: refreshedTokens.accessToken,
            token_type: "Bearer",
            expires_in: Math.floor(accessTokenMs / 1000),
            refresh_token: refreshedTokens.refreshToken,
            scope: "openid profile email",
        },
        {status: 200}
    );
}
