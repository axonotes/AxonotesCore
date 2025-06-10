import {workos, workosClientId} from "$lib/server/workos";
import {WORKOS_REDIRECT_URI} from "$env/static/private";
import {redirect} from "@sveltejs/kit";

export async function GET() {
    const authorizationURL = workos.userManagement.getAuthorizationUrl({
        provider: "authkit",
        redirectUri: WORKOS_REDIRECT_URI,
        clientId: workosClientId,
    });

    throw redirect(302, authorizationURL);
}
