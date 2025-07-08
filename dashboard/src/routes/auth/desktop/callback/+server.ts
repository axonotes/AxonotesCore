import {redirect} from "@sveltejs/kit";
import {workos, workosClientId, workos_jwks} from "$lib/server/workos";
import {desktopAuthManager} from "$lib/server/desktop-auth";
import {type UserTokenPayload} from "$lib/server/jwt";
import {JWT_ISSUER, WORKOS_SESSION_ID_COOKIE_NAME} from "$env/static/private";
import {jwtVerify} from "jose";
import {IN_DEVELOPMENT} from "$lib/utils";

export async function GET({url, cookies}) {
    const code = url.searchParams.get("code");
    const state = url.searchParams.get("state"); // This is our session ID

    if (!code || !state) {
        throw redirect(302, "/auth/desktop/error?error=invalid_request");
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

        // Store WorkOS session ID in cookie (needed for future revocation)
        cookies.set(WORKOS_SESSION_ID_COOKIE_NAME, payload.sid as string, {
            path: "/",
            httpOnly: true,
            secure: !IN_DEVELOPMENT,
            maxAge: 60 * 60 * 24 * 7, // 1 week, should match token expiry
            sameSite: "lax",
        });

        // Create token payload for our own tokens
        const tokenPayload: UserTokenPayload = {
            sub: user.id,
            iss: JWT_ISSUER || "https://app.axonotes.ch",
            email: user.email,
            firstName: user.firstName ?? "",
            lastName: user.lastName ?? "",
        };

        // Complete the desktop auth session
        await desktopAuthManager.completeAuth(state, tokenPayload);
    } catch (error) {
        console.error("Desktop auth callback failed:", error);
        throw redirect(302, "/auth/desktop/error?error=auth_failed");
    }

    // Generate custom URL for desktop app
    const desktopUrl = `axonotes://auth/success?session_id=${state}`;

    // Redirect to success page that will attempt to open the desktop app
    throw redirect(
        302,
        `/auth/desktop/success?session_id=${state}&desktop_url=${encodeURIComponent(desktopUrl)}`
    );
}
