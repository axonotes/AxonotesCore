import {redirect} from "@sveltejs/kit";
import {JWT_COOKIE_NAME, WORKOS_SESSION_ID_COOKIE_NAME} from "$env/static/private";
import {workos} from "$lib/server/workos";

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

    cookies.delete(JWT_COOKIE_NAME, {path: "/"});
    cookies.delete(WORKOS_SESSION_ID_COOKIE_NAME, {path: "/"});

    throw redirect(302, "/");
}