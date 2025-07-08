import {redirect} from "@sveltejs/kit";
import {workos, workos_jwks, workosClientId} from "$lib/server/workos";
import {
    generateWebTokens,
    getTokenLifetimes,
    type UserTokenPayload,
} from "$lib/server/jwt";
import {
    REFRESH_TOKEN_COOKIE_NAME,
    JWT_ISSUER,
    WORKOS_SESSION_ID_COOKIE_NAME,
} from "$env/static/private";
import {jwtVerify} from "jose";
import {IN_DEVELOPMENT} from "$lib/utils";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

export async function GET({url, cookies}) {
    const code = url.searchParams.get("code");

    if (!code) {
        throw redirect(302, "/?error=no_code");
    }

    try {
        const {user, accessToken} =
            await workos.userManagement.authenticateWithCode({
                code,
                clientId: workosClientId,
            });

        if (!user.email) {
            throw new Error("User email is required");
        }

        const tokenPayload: UserTokenPayload = {
            sub: user.id,
            iss: JWT_ISSUER || "https://app.axonotes.ch",
            email: user.email,
            firstName: user.firstName ?? "",
            lastName: user.lastName ?? "",
        };

        const {
            accessToken: axonotesAccessToken,
            refreshToken: axonotesRefreshToken,
        } = generateWebTokens(tokenPayload);
        const {accessTokenMs, refreshTokenMs} = getTokenLifetimes(false);

        const {payload} = await jwtVerify(accessToken, workos_jwks);

        if (!payload.sid || typeof payload.sid !== "string") {
            throw new Error("Invalid session ID in WorkOS token");
        }

        cookies.set(PUBLIC_ACCESS_TOKEN_COOKIE_NAME, axonotesAccessToken, {
            path: "/",
            httpOnly: false,
            secure: !IN_DEVELOPMENT,
            maxAge: Math.floor(accessTokenMs / 1000), // should match token expiry
            sameSite: "lax",
        });

        cookies.set(REFRESH_TOKEN_COOKIE_NAME, axonotesRefreshToken, {
            path: "/",
            httpOnly: true,
            secure: !IN_DEVELOPMENT,
            maxAge: Math.floor(refreshTokenMs / 1000), // should match token expiry
            sameSite: "strict",
        });

        cookies.set(WORKOS_SESSION_ID_COOKIE_NAME, payload.sid as string, {
            path: "/",
            httpOnly: true,
            secure: !IN_DEVELOPMENT,
            maxAge: 60 * 60 * 24 * 7, // 1 week, should match token expiry
            sameSite: "lax",
        });
    } catch (error) {
        console.error("WorkOS auth failed:", error);
        throw redirect(302, "/?error=auth_failed");
    }

    throw redirect(302, "/auth/gate");
}
