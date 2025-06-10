import {json, error} from "@sveltejs/kit";
import {
    verifyToken,
    generateAccessToken,
    UserTokenPayload,
} from "$lib/server/jwt";
import {REFRESH_TOKEN_COOKIE_NAME} from "$env/static/private";

export async function POST({cookies}) {
    const refreshToken = cookies.get(REFRESH_TOKEN_COOKIE_NAME);

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
    };

    // If refresh token is valid, issue a new access token
    const newAccessToken = generateAccessToken(accessTokenPayload);

    return json({accessToken: newAccessToken});
}
