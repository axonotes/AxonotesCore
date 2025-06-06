import {redirect} from "@sveltejs/kit";
import {workos, workos_jwks, workosClientId} from "$lib/server/workos";
import {generateToken, type UserTokenPayload} from "$lib/server/jwt";
import {JWT_COOKIE_NAME, WORKOS_SESSION_ID_COOKIE_NAME} from "$env/static/private";
import {jwtVerify} from "jose";

export async function GET({url, cookies}) {
    const code = url.searchParams.get("code");

    if (!code) {
        throw redirect(302, "/?error=no_code");
    }

    try {
        const {user, accessToken} =
            await workos.userManagement.authenticateWithCode({
                code,
                clientId: workosClientId
            })

        const tokenPayload: UserTokenPayload = {
            sub: user.id,
            email: user.email,
            firstName: user.firstName ?? "",
            lastName: user.lastName ?? ""
        };

        const authToken = generateToken(tokenPayload);

        cookies.set(JWT_COOKIE_NAME, authToken, {
            path: "/",
            httpOnly: true,
            secure: process.env.NODE_ENV === "production",
            maxAge: 60 * 60 * 24 * 7, // 1 week, should match token expiry
            sameSite: "lax",
        })

        const {payload} = await jwtVerify(accessToken, workos_jwks);

        cookies.set(WORKOS_SESSION_ID_COOKIE_NAME, payload.sid as string, {
            path: "/",
            httpOnly: true,
            secure: process.env.NODE_ENV === "production",
            maxAge: 60 * 60 * 24 * 7, // 1 week, should match token expiry
            sameSite: "lax",
        })
    } catch (error) {
        console.error("WorkOS auth failed:", error);
        throw redirect(302, "/?error=auth_failed");
    }

    throw redirect(302, "/dashboard");
}