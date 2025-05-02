import { handle as authHandle } from "./auth"
import {sequence} from "@sveltejs/kit/hooks";
import {type Handle, redirect} from "@sveltejs/kit";

const siteRestrictionHandle: Handle = async ({ event, resolve }) => {
    const session = await event.locals.auth();
    const {pathname} = event.url;

    if (pathname.startsWith('/profile') && !session) {
        // User is not logged in, redirect to auth page
        throw redirect(303, '/auth');
    }

    if (pathname.startsWith('/auth') && session) {
        // User is logged in, redirect to profile page
        throw redirect(303, '/profile')
    }

    return resolve(event);
}

export const handle = sequence(authHandle, siteRestrictionHandle);