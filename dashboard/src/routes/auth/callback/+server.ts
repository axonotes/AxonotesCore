import {redirect} from "@sveltejs/kit";
import {workos, workos_jwks, workosClientId} from "$lib/server/workos";
import {
    generateTokens,
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

/**
 * Handle the WorkOS OAuth callback: exchange the authorization code, create app tokens, set session cookies, and redirect to the app gate.
 *
 * Exchanges the `code` query parameter for a WorkOS user and access token, verifies the WorkOS token to extract a session id, builds an internal user token payload, generates internal access and refresh tokens, and sets three cookies:
 * - PUBLIC_ACCESS_TOKEN_COOKIE_NAME: public (non-httpOnly) access token for the client
 * - REFRESH_TOKEN_COOKIE_NAME: httpOnly refresh token
 * - WORKOS_SESSION_ID_COOKIE_NAME: httpOnly WorkOS session id
 *
 * On success the handler redirects (302) to "/auth/gate". If the `code` parameter is missing the handler redirects to "/?error=no_code". Any failure during the exchange/verification flow logs the error and redirects to "/?error=auth_failed".
 */
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
            client_type: "web", // Always web for WorkOS auth
        };

        const {
            accessToken: axonotesAccessToken,
            refreshToken: axonotesRefreshToken,
        } = generateTokens(tokenPayload);
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
