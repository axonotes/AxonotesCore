import {WorkOS} from "@workos-inc/node";
import {WORKOS_API_KEY, WORKOS_CLIENT_ID} from "$env/static/private";
import {createRemoteJWKSet} from "jose";

export const workos = new WorkOS(WORKOS_API_KEY);
export const workosClientId = WORKOS_CLIENT_ID;
export const workos_jwks = createRemoteJWKSet(
    new URL(workos.userManagement.getJwksUrl(WORKOS_CLIENT_ID))
);