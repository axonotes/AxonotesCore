import {json, error} from "@sveltejs/kit";
import {
    verifyToken,
    generateDesktopTokens,
    getTokenLifetimes,
    type UserTokenPayload,
} from "$lib/server/jwt";

export async function POST({request}) {
    try {
        const {refreshToken} = await request.json();

        if (!refreshToken) {
            throw error(400, "Refresh token is required");
        }

        // Verify the refresh token
        const payload = verifyToken(refreshToken);
        if (!payload) {
            throw error(401, "Invalid or expired refresh token");
        }

        // Create new token payload
        const tokenPayload: UserTokenPayload = {
            sub: payload.sub,
            iss: payload.iss,
            email: payload.email,
            firstName: payload.firstName,
            lastName: payload.lastName,
        };

        // Generate new desktop tokens (longer lifetimes)
        const {accessToken, refreshToken: newRefreshToken} =
            generateDesktopTokens(tokenPayload);

        // Get desktop token lifetimes
        const {accessTokenMs} = getTokenLifetimes(true);

        return json({
            accessToken,
            refreshToken: newRefreshToken,
            expiresAt: Date.now() + accessTokenMs,
            user: {
                id: payload.sub,
                email: payload.email,
            },
        });
    } catch (err) {
        console.error("Desktop token refresh failed:", err);

        if (err instanceof Error) {
            if (err.message.includes("expired")) {
                throw error(401, "Refresh token has expired");
            }
            if (err.message.includes("invalid")) {
                throw error(401, "Invalid refresh token");
            }
        }

        throw error(500, "Internal server error");
    }
}
