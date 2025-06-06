import type {Handle} from "@sveltejs/kit";

export const handle: Handle = async ({ event, resolve }) => {
    const sessionCookie = event.cookies.get("axonotes_session");

    if (!sessionCookie) {
        event.locals.user = null;
        return resolve(event);
    }

    try {
        event.locals.user = JSON.parse(sessionCookie);
    } catch (error) {
        event.locals.user = null;
    }

    return resolve(event);
}