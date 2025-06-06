import type {Handle} from "@sveltejs/kit";
import {JWT_COOKIE_NAME} from "$env/static/private";
import {verifyToken} from "$lib/server/jwt";

export const handle: Handle = async ({event, resolve}) => {
    const token = event.cookies.get(JWT_COOKIE_NAME);

    event.locals.user = null;

    if (token) {
        const userPayload = verifyToken(token);

        if (userPayload) {
            event.locals.user = userPayload;
        } else {
            event.cookies.delete(JWT_COOKIE_NAME, {path: "/"});
        }
    }

    return resolve(event);
}