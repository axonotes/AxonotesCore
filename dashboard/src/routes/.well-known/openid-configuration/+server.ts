import {json} from "@sveltejs/kit";
import {JWT_ISSUER} from "$env/static/private";

/**
 * OpenID Connect discovery endpoint that returns the provider configuration.
 *
 * Returns a JSON OpenID Provider Configuration containing `issuer`, `jwks_uri`,
 * supported `subject_types`, and `id_token_signing_alg_values_supported`.
 * Sets a public Cache-Control header ("public, max-age=3600, s-maxage=86400").
 *
 * @returns A JSON response with the OIDC discovery configuration
 */
export async function GET({setHeaders}) {
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
