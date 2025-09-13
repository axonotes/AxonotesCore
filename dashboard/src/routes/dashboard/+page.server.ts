import {redirect} from "@sveltejs/kit";

/**
 * Server load for the dashboard route.
 *
 * Redirects unauthenticated requests to "/" with a 302 status. When authenticated, exposes the current user to the page.
 *
 * @param locals - Request-local state; expected to contain `user` when the session is authenticated.
 * @returns An object with `user` (the authenticated user) to be merged into the page's load data.
 * @throws Redirect (302) to "/" if `locals.user` is falsy.
 */
export async function load({locals}) {
    if (!locals.user) {
        throw redirect(302, "/");
    }

    return {
        user: locals.user,
    };
}
