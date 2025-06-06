import {WorkOS} from "@workos-inc/node";
import {WORKOS_API_KEY, WORKOS_CLIENT_ID} from "$env/static/private";

export const workos = new WorkOS(WORKOS_API_KEY);
export const workosClientId = WORKOS_CLIENT_ID;