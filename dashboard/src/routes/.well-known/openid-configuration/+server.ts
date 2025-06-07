import { json } from "@sveltejs/kit";
import { JWT_ISSUER } from "$env/static/private";

/**
 * OIDC Discovery Endpoint.
 * This provides configuration metadata to clients, telling them where to
 * find the public keys (JWKS).
 */
export async function GET({ setHeaders }) {
    const configuration = {
        issuer: JWT_ISSUER,
        jwks_uri: `${JWT_ISSUER}/.well-known/jwks.json`,
        subject_types_supported: ["public"],
        id_token_signing_alg_values_supported: ["ES256"],
    };

    setHeaders({
        "Cache-Control": "public, max-age=3600, s-maxage=86400",
    });

    return json(configuration);
}