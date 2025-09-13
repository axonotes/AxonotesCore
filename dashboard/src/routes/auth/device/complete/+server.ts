import {redirect} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {verifyToken} from "$lib/server/jwt";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

/**
 * Completes a device authentication flow and redirects to success or an error page.
 *
 * Reads `user_code` from the request query and requires a valid access token cookie (named by
 * `PUBLIC_ACCESS_TOKEN_COOKIE_NAME`). If the code or token is missing/invalid, or if completion
 * fails, the handler issues a 302 redirect to `/auth/device/error` with an appropriate `error`
 * query parameter (`missing_user_code`, `not_authenticated`, `invalid_token`, or `completion_failed`).
 *
 * On successful completion it issues a 302 redirect to `/auth/device/success`.
 */
export async function GET({url, cookies}) {
    const user_code = url.searchParams.get("user_code");

    if (!user_code) {
        throw redirect(302, "/auth/device/error?error=missing_user_code");
    }

    // Verify user is authenticated
    const accessToken = cookies.get(PUBLIC_ACCESS_TOKEN_COOKIE_NAME);
    if (!accessToken) {
        throw redirect(302, "/auth/device/error?error=not_authenticated");
    }

    const tokenPayload = verifyToken(accessToken);
    if (!tokenPayload) {
        throw redirect(302, "/auth/device/error?error=invalid_token");
    }

    try {
        // Complete the device flow
        await desktopAuthManager.completeDeviceAuth(user_code, tokenPayload);
    } catch (error) {
        console.error("Device auth completion error:", error);
        throw redirect(302, "/auth/device/error?error=completion_failed");
    }

    throw redirect(302, "/auth/device/success");
}
