import {generateKeyPairSync} from "node:crypto";

/**
 * Generates a new EC key pair for JWT use and returns both PEM and Base64 forms plus a date-based key ID.
 *
 * Creates an EC (prime256v1) key pair encoded as PEM (public: SPKI, private: PKCS8), derives Base64-encoded
 * versions of each PEM string, and builds a keyId in the form `axonotes-key-YYYY-MM-DD` using the current UTC date.
 *
 * @returns An object with:
 * - `privateKey` — private key PEM (PKCS8)
 * - `publicKey` — public key PEM (SPKI)
 * - `privateKeyBase64` — Base64 encoding of the private PEM
 * - `publicKeyBase64` — Base64 encoding of the public PEM
 * - `keyId` — identifier `axonotes-key-YYYY-MM-DD` based on the current UTC date
 */
function generateJWTKeys() {
    const {privateKey, publicKey} = generateKeyPairSync("ec", {
        namedCurve: "prime256v1",
        publicKeyEncoding: {type: "spki", format: "pem"},
        privateKeyEncoding: {type: "pkcs8", format: "pem"},
    });

    const privateKeyBase64 = Buffer.from(privateKey).toString("base64");
    const publicKeyBase64 = Buffer.from(publicKey).toString("base64");
    const keyId = `axonotes-key-${new Date().toISOString().slice(0, 10)}`;

    return {
        privateKey,
        publicKey,
        privateKeyBase64,
        publicKeyBase64,
        keyId,
    };
}

export const defaultKeys = generateJWTKeys();
