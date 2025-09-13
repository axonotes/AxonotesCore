import {generateKeyPairSync} from "node:crypto";

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
