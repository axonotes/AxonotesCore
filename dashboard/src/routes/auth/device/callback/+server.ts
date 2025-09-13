import {redirect} from "@sveltejs/kit";
import {workos, workosClientId, workos_jwks} from "$lib/server/workos";
import {generateTokens, type UserTokenPayload} from "$lib/server/jwt";
import {
    JWT_ISSUER,
    WORKOS_SESSION_ID_COOKIE_NAME,
    REFRESH_TOKEN_COOKIE_NAME,
} from "$env/static/private";
import {jwtVerify} from "jose";
import {IN_DEVELOPMENT} from "$lib/utils";
import {getTokenLifetimes} from "$lib/server/jwt";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

/**
 * SvelteKit GET route handler for the WorkOS device flow callback.
 *
 * Exchanges the WorkOS authorization `code` (from the query) for a user and WorkOS access token,
 * validates required fields (user.email and a string `sid` in the WorkOS token), generates
 * temporary web access and refresh tokens, sets three cookies (public access token, HttpOnly refresh
 * token, and WorkOS session id), and then redirects the client to the device gate flow with the
 * original `state` (used as `user_code`).
 *
 * On missing `code` or `state`, or on any authentication/validation failure, the handler redirects
 * to /auth/device/error with an appropriate error query (`invalid_request` or `auth_failed`).
 *
 * Cookies set:
 * - PUBLIC_ACCESS_TOKEN_COOKIE_NAME: non-HttpOnly access token for client-side use (SameSite=lax).
 * - REFRESH_TOKEN_COOKIE_NAME: HttpOnly refresh token (SameSite=strict).
 * - WORKOS_SESSION_ID_COOKIE_NAME: HttpOnly WorkOS session id (`sid`) (SameSite=lax).
 *
 * Cookie `secure` flags depend on the IN_DEVELOPMENT flag; cookie lifetimes use token lifetimes or a
 * one-week TTL for the WorkOS session id.
 */
export async function GET({url, cookies}) {
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state"); // This is our user_code

    if (!code || !state) {
        throw redirect(302, "/auth/device/error?error=invalid_request");
    }

    try {
        // Authenticate with WorkOS
        const {user, accessToken} =
            await workos.userManagement.authenticateWithCode({
                code,
                clientId: workosClientId,
            });

        if (!user.email) {
            throw new Error("User email is required");
        }

        // Extract WorkOS session ID from access token
        const {payload} = await jwtVerify(accessToken, workos_jwks);

        if (!payload.sid || typeof payload.sid !== "string") {
            throw new Error("Invalid session ID in WorkOS token");
        }

        // Create token payload
        const tokenPayload: UserTokenPayload = {
            sub: user.id,
            iss: JWT_ISSUER || "https://app.axonotes.ch",
            email: user.email,
            firstName: user.firstName ?? "",
            lastName: user.lastName ?? "",
            client_type: "web", // Always web for WorkOS auth
        };

        // Generate temporary web tokens for the setup/gate flow
        const {accessToken: tempAccessToken, refreshToken: tempRefreshToken} =
            generateTokens(tokenPayload);
        const {accessTokenMs, refreshTokenMs} = getTokenLifetimes(false);

        // Store temporary auth cookies
        cookies.set(PUBLIC_ACCESS_TOKEN_COOKIE_NAME, tempAccessToken, {
            path: "/",
            httpOnly: false,
            secure: !IN_DEVELOPMENT,
            maxAge: Math.floor(accessTokenMs / 1000),
            sameSite: "lax",
        });

        cookies.set(REFRESH_TOKEN_COOKIE_NAME, tempRefreshToken, {
            path: "/",
            httpOnly: true,
            secure: !IN_DEVELOPMENT,
            maxAge: Math.floor(refreshTokenMs / 1000),
            sameSite: "strict",
        });

        cookies.set(WORKOS_SESSION_ID_COOKIE_NAME, payload.sid as string, {
            path: "/",
            httpOnly: true,
            secure: !IN_DEVELOPMENT,
            maxAge: 60 * 60 * 24 * 7, // 1 week
            sameSite: "lax",
        });
    } catch (error) {
        console.error("Device auth callback failed:", error);
        throw redirect(302, "/auth/device/error?error=auth_failed");
    }

    // Redirect to device gate - this will check if setup is needed
    throw redirect(302, `/auth/gate?user_code=${state}`);
}
