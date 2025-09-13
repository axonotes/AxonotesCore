import type {Handle} from "@sveltejs/kit";
import {REFRESH_TOKEN_COOKIE_NAME} from "$env/static/private";
import {verifyToken} from "$lib/server/jwt";

import {error, json, text} from "@sveltejs/kit";
import {sequence} from "@sveltejs/kit/hooks";

/**
 * CSRF protection copied from sveltekit but with the ability to turn it off for specific routes.
 */
const csrf =
    (allowedPaths: string[]): Handle =>
    async ({event, resolve}) => {
        const forbidden =
            event.request.method === "POST" &&
            event.request.headers.get("origin") !== event.url.origin &&
            isFormContentType(event.request) &&
            !allowedPaths.includes(event.url.pathname);

        if (forbidden) {
            const csrfError = {
                status: 403,
                body: {
                    message: `Cross-site ${event.request.method} form submissions are forbidden`,
                },
            };
            if (event.request.headers.get("accept") === "application/json") {
                return json(csrfError.body, {status: csrfError.status});
            }
            return text(csrfError.body.message, {status: csrfError.status});
        }

        return resolve(event);
    };

/**
 * Checks whether the request's Content-Type (base media type) matches any of the provided types.
 *
 * The header is normalized by removing any parameters (for example `; charset=utf-8`) before comparison.
 *
 * @param request - The incoming Request whose Content-Type will be checked
 * @param types - One or more media types to match against (e.g., `"application/json"`, `"multipart/form-data"`)
 * @returns `true` if the request's base Content-Type equals any of the provided types
 */
function isContentType(request: Request, ...types: string[]) {
    const type =
        request.headers.get("content-type")?.split(";", 1)[0].trim() ?? "";
    return types.includes(type);
}

/**
 * Returns true if the request's Content-Type indicates an HTML form submission.
 *
 * Checks whether the request's Content-Type header is either
 * `application/x-www-form-urlencoded` or `multipart/form-data`.
 *
 * @returns `true` when the request is a form submission content type, otherwise `false`.
 */
function isFormContentType(request: Request) {
    return isContentType(
        request,
        "application/x-www-form-urlencoded",
        "multipart/form-data"
    );
}

const authHandle: Handle = async ({event, resolve}) => {
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

// Combine handles
export const handle: Handle = sequence(
    csrf(["/oauth/device/authorize", "/oauth/device/token", "/oauth/token"]),
    authHandle
);
