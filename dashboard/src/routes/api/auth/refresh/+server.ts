import {json, error} from "@sveltejs/kit";
import {
    verifyToken,
    UserTokenPayload, generateTokens,
} from "$lib/server/jwt";
import {REFRESH_TOKEN_COOKIE_NAME} from "$env/static/private";

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

    const accessTokenPayload: UserTokenPayload = {
        sub: payload.sub,
        iss: payload.iss,
        email: payload.email,
        firstName: payload.firstName,
        lastName: payload.lastName,
        client_type: payload.client_type,
    };

    return json(generateTokens(accessTokenPayload));
}
