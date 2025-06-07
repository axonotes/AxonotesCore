import jwt from "jsonwebtoken";
import {
    JWT_KEY_ID,
    JWT_PRIVATE_KEY_BASE64,
    JWT_PUBLIC_KEY_BASE64,
} from "$env/static/private";

export interface UserTokenPayload {
    sub: string;
    iss: string;
    email: string;
    firstName: string;
    lastName: string;
}

const privateKey = Buffer.from(JWT_PRIVATE_KEY_BASE64, "base64").toString(
    "ascii",
);

const publicKey = Buffer.from(JWT_PUBLIC_KEY_BASE64, "base64").toString(
    "ascii",
);

const keyId = JWT_KEY_ID;

const EXPIRES_IN = "7d"; // How long the token is valid for
const ALGORITHM = "ES256"; // Elliptic Curve algorithm

if (!privateKey || !publicKey || !keyId) {
    throw new Error(
        "JWT keys are not set correctly. Please generate them with `bun run generate` and copy them into the .env file",
    );
}

/**
 * Signs a payload to generate a JWT using the PRIVATE key.
 * @param payload The user data to encode in the token.
 * @returns The generated JWT string.
 */
export function generateToken(payload: UserTokenPayload): string {
    return jwt.sign(payload, privateKey, {
        algorithm: ALGORITHM,
        expiresIn: EXPIRES_IN,
        keyid: keyId,
    });
}

/**
 * Verifies a JWT using the PUBLIC key and returns its payload if valid.
 * @param token The JWT string to verify.
 * @returns The decoded payload, or null if verification fails.
 */
export function verifyToken(token: string): UserTokenPayload | null {
    try {
        const decoded = jwt.verify(token, publicKey, {
            algorithms: [ALGORITHM], // Important to specify the allowed algorithm
        });
        return decoded as UserTokenPayload;
    } catch (error) {
        // Catches expired tokens, invalid signatures, etc.
        console.error("JWT Verification failed:", error);
        return null;
    }
}