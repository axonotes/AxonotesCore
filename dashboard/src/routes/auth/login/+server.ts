import {workos, workosClientId} from "$lib/server/workos";
import {WORKOS_REDIRECT_URI} from "$env/static/private";
import {redirect} from "@sveltejs/kit";

/**
 * Redirects the client (HTTP 302) to a WorkOS User Management authorization URL.
 *
 * Builds an authorization URL for provider `"authkit"` using the configured client ID and redirect URI,
 * then issues a SvelteKit 302 redirect to that URL by throwing `redirect(302, authorizationURL)`.
 */
export async function GET() {
    const authorizationURL = workos.userManagement.getAuthorizationUrl({
        provider: "authkit",
        redirectUri: WORKOS_REDIRECT_URI,
        clientId: workosClientId,
    });

    throw redirect(302, authorizationURL);
}
