/**
 * Exposes the authenticated user from server `locals` to the layout's load data.
 *
 * @param locals - Server-side `locals` object containing a `user` property
 * @returns An object with `user` set to `locals.user`
 */
export async function load({locals}) {
    return {
        user: locals.user,
    };
}
