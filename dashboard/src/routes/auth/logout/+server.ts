import {redirect} from "@sveltejs/kit";
import {
    REFRESH_TOKEN_COOKIE_NAME,
    WORKOS_SESSION_ID_COOKIE_NAME,
} from "$env/static/private";
import {workos} from "$lib/server/workos";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

/**
 * Logs the user out: revokes the WorkOS session (if present), clears auth cookies, and redirects to "/".
 *
 * If a WorkOS session ID cookie is present the handler attempts to revoke that session; errors during revocation
 * are caught and logged but do not prevent cookie deletion or the redirect. The following cookies are deleted
 * with path "/": the public access token, refresh token, and WorkOS session ID. Finally the handler issues a
 * 302 redirect to the root path.
 */
export async function POST({cookies}) {
    // revoke WorkOS session
    const sessionId = cookies.get(WORKOS_SESSION_ID_COOKIE_NAME);
    if (sessionId) {
        try {
            await workos.userManagement.revokeSession({
                sessionId: sessionId,
            });
        } catch (error) {
            console.error("Failed to revoke WorkOS session: ", error);
        }
    }

    cookies.delete(PUBLIC_ACCESS_TOKEN_COOKIE_NAME, {path: "/"});
    cookies.delete(REFRESH_TOKEN_COOKIE_NAME, {path: "/"});
    cookies.delete(WORKOS_SESSION_ID_COOKIE_NAME, {path: "/"});

    throw redirect(302, "/");
}
