import { json } from "@sveltejs/kit";
import { importSPKI, exportJWK } from "jose";
import {JWT_KEY_ID, JWT_PUBLIC_KEY_BASE64} from "$env/static/private";

const publicKeyPem = Buffer.from(
    JWT_PUBLIC_KEY_BASE64,
    "base64"
).toString("ascii");
const keyId = JWT_KEY_ID;

/**
 * This endpoint serves the public key in the standard JWKS format.
 * It allows services like SpaceTimeDB to fetch the key for JWT verification.
 */
export async function GET({ setHeaders }) {
    if (!publicKeyPem || !keyId) {
        return json(
            { error: "Server configuration error" },
            { status: 500 }
        );
    }

    const publicKey = await importSPKI(publicKeyPem, "ES256");
    const jwk = await exportJWK(publicKey);

    const jwks = {
        keys: [
            {
                ...jwk,
                kid: keyId, // The unique identifier for this key
                use: "sig", // Indicates the key is used for signing/verification
                alg: "ES256", // The algorithm used with this key
            },
        ],
    };

    // Set cache headers. Public keys rarely change, so they can be cached
    // aggressively by clients and CDNs to reduce requests to your server.
    setHeaders({
        "Cache-Control": "public, max-age=3600, s-maxage=86400", // Cache for 1hr (client) / 24hr (proxy)
    });

    console.log("jwk accessed");

    return json(jwks);
}