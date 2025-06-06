import {redirect} from "@sveltejs/kit";
import {workos, workosClientId} from "$lib/server/workos";

export async function GET({ url, cookies }) {
    const code = url.searchParams.get("code");

    if (!code) {
        throw redirect(302, "/?error=no_code");
    }

    try {
        const { user } =
            await workos.userManagement.authenticateWithCode({
                code,
                clientId: workosClientId
            })

        cookies.set("axonotes_session", JSON.stringify(user), {
            path: "/",
            httpOnly: true,
            secure: process.env.NODE_ENV === "production",
            maxAge: 60 * 60 * 24 * 7, // 1 week
        })
    } catch (error) {
        console.error("WorkOS auth failed:", error);
        throw redirect(302, "/?error=auth_failed");
    }

    throw redirect(302, "/dashboard");
}