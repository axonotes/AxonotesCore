import {redirect} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/desktop-auth";
import {workos, workosClientId} from "$lib/server/workos";
import {
    DESKTOP_AUTH_CALLBACK_URL,
    WORKOS_SESSION_ID_COOKIE_NAME,
} from "$env/static/private";

export async function load({url, cookies}) {
    const forceNewLogin = url.searchParams.get("force_new_login") === "true";

    // If force_new_login is true, revoke the current WorkOS session
    if (forceNewLogin) {
        const sessionId = cookies.get(WORKOS_SESSION_ID_COOKIE_NAME);
        if (sessionId) {
            try {
                await workos.userManagement.revokeSession({
                    sessionId: sessionId,
                });
                console.log("WorkOS session revoked for desktop auth");
            } catch (error) {
                console.error("Failed to revoke WorkOS session:", error);
                // Continue anyway - the session might already be expired
            }
        }

        // Clear the session cookie
        cookies.delete(WORKOS_SESSION_ID_COOKIE_NAME, {path: "/"});
    }

    // Generate session ID for desktop auth
    const sessionId = await desktopAuthManager.createSession();

    // Create WorkOS authorization URL with desktop callback
    const authorizationURL = workos.userManagement.getAuthorizationUrl({
        provider: "authkit",
        redirectUri: DESKTOP_AUTH_CALLBACK_URL,
        clientId: workosClientId,
        state: sessionId, // Pass session ID as state
    });

    throw redirect(302, authorizationURL);
}
