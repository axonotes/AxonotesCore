import {WorkOS} from "@workos-inc/node";
import {WORKOS_API_KEY, WORKOS_CLIENT_ID} from "$env/static/private";
import {createRemoteJWKSet} from "jose";

if (!WORKOS_API_KEY || !WORKOS_CLIENT_ID) {
    throw new Error("Missing required WorkOS environment variables: WORKOS_API_KEY and WORKOS_CLIENT_ID must be set");
}

export const workos = new WorkOS(WORKOS_API_KEY);
export const workosClientId = WORKOS_CLIENT_ID;
export const workos_jwks = (() => {
    try {
        const jwksUrl = workos.userManagement.getJwksUrl(WORKOS_CLIENT_ID);
        return createRemoteJWKSet(new URL(jwksUrl));
    } catch (error) {
        console.error("Failed to initialize WorkOS JWKS:", error);
        throw new Error("WorkOS JWKS initialization failed");
    }
})();