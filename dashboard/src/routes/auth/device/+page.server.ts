import {redirect} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {verifyToken} from "$lib/server/jwt";
import {workos, workosClientId} from "$lib/server/workos";
import {
    WORKOS_SESSION_ID_COOKIE_NAME,
    REFRESH_TOKEN_COOKIE_NAME,
    JWT_ISSUER,
} from "$env/static/private";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

/**
 * Validates a device `user_code`, determines whether the requester is already authenticated, and returns the code plus current user info when available.
 *
 * If `user_code` is missing or invalid this function throws a 302 redirect to an error page.
 *
 * @returns An object containing:
 * - `user_code` — the validated device user code from the URL query string.
 * - `currentUser` — `null` if no valid access token is present; otherwise an object with `id`, `email`, `firstName`, `lastName`, and `fullName`.
 */
export async function load({url, cookies}) {
    const user_code = url.searchParams.get("user_code");

    if (!user_code) {
        throw redirect(302, "/auth/device/error?error=missing_user_code");
    }

    // Validate user code
    const isValidUserCode =
        await desktopAuthManager.validateUserCode(user_code);
    if (!isValidUserCode) {
        throw redirect(302, "/auth/device/error?error=invalid_user_code");
    }

    // Check if user is already logged in
    const accessToken = cookies.get(PUBLIC_ACCESS_TOKEN_COOKIE_NAME);
    let currentUser = null;

    if (accessToken) {
        const tokenPayload = verifyToken(accessToken);
        if (tokenPayload) {
            currentUser = {
                id: tokenPayload.sub,
                email: tokenPayload.email,
                firstName: tokenPayload.firstName,
                lastName: tokenPayload.lastName,
                fullName:
                    `${tokenPayload.firstName} ${tokenPayload.lastName}`.trim(),
            };
        }
    }

    return {
        user_code,
        currentUser,
    };
}

export const actions = {
    useCurrentAccount: async ({request, cookies}) => {
        const formData = await request.formData();
        const user_code = formData.get("user_code")?.toString();

        if (!user_code) {
            throw redirect(302, "/auth/device/error?error=missing_user_code");
        }

        // Verify user is still logged in
        const accessToken = cookies.get(PUBLIC_ACCESS_TOKEN_COOKIE_NAME);
        if (!accessToken) {
            throw redirect(302, "/auth/device/error?error=not_authenticated");
        }

        const tokenPayload = verifyToken(accessToken);
        if (!tokenPayload) {
            throw redirect(302, "/auth/device/error?error=invalid_token");
        }

        try {
            // Complete the device flow with current user
            await desktopAuthManager.completeDeviceAuth(
                user_code,
                tokenPayload
            );
        } catch (error) {
            console.error("Device auth completion error:", error);
            throw redirect(302, "/auth/device/error?error=completion_failed");
        }

        throw redirect(302, "/auth/device/success");
    },

    useOtherAccount: async ({request, cookies}) => {
        const formData = await request.formData();
        const user_code = formData.get("user_code")?.toString();

        if (!user_code) {
            throw redirect(302, "/auth/device/error?error=missing_user_code");
        }

        // Revoke current WorkOS session
        const workosSessionId = cookies.get(WORKOS_SESSION_ID_COOKIE_NAME);
        if (workosSessionId) {
            try {
                await workos.userManagement.revokeSession({
                    sessionId: workosSessionId,
                });
            } catch (error) {
                console.error("Failed to revoke WorkOS session:", error);
                // Continue anyway - session might already be expired
            }
        }

        // Clear all auth cookies
        cookies.delete(PUBLIC_ACCESS_TOKEN_COOKIE_NAME, {path: "/"});
        cookies.delete(REFRESH_TOKEN_COOKIE_NAME, {path: "/"});
        cookies.delete(WORKOS_SESSION_ID_COOKIE_NAME, {path: "/"});

        // Redirect to WorkOS auth with user_code in state
        const authorizationURL = workos.userManagement.getAuthorizationUrl({
            provider: "authkit",
            redirectUri: `${JWT_ISSUER}/auth/device/callback`,
            clientId: workosClientId,
            state: user_code,
        });

        throw redirect(302, authorizationURL);
    },

    loginWithUserCode: async ({request}) => {
        const formData = await request.formData();
        const user_code = formData.get("user_code")?.toString();

        if (!user_code) {
            throw redirect(302, "/auth/device/error?error=missing_user_code");
        }

        // Redirect to WorkOS auth (user not logged in)
        const authorizationURL = workos.userManagement.getAuthorizationUrl({
            provider: "authkit",
            redirectUri: `${JWT_ISSUER}/auth/device/callback`,
            clientId: workosClientId,
            state: user_code,
        });

        throw redirect(302, authorizationURL);
    },
};
