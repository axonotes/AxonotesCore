import type {Handle} from "@sveltejs/kit";
import {REFRESH_TOKEN_COOKIE_NAME} from "$env/static/private";
import {verifyToken} from "$lib/server/jwt";

export const handle: Handle = async ({event, resolve}) => {
    const token = event.cookies.get(REFRESH_TOKEN_COOKIE_NAME);

    event.locals.user = null;

    if (token) {
        const userPayload = verifyToken(token);

        if (userPayload) {
            event.locals.user = userPayload;
        } else {
            event.cookies.delete(REFRESH_TOKEN_COOKIE_NAME, {path: "/"});
        }
    }

    return resolve(event);
};
