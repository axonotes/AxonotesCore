import {redirect} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/oauth";
import {verifyToken} from "$lib/server/jwt";
import {PUBLIC_ACCESS_TOKEN_COOKIE_NAME} from "$env/static/public";

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
