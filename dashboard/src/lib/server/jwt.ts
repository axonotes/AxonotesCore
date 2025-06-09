import jwt from "jsonwebtoken";
import {
    ACCESS_TOKEN_EXPIRES_IN,
    JWT_KEY_ID,
    JWT_PRIVATE_KEY_BASE64,
    JWT_PUBLIC_KEY_BASE64,
    REFRESH_TOKEN_EXPIRES_IN,
} from "$env/static/private";
import ms from "ms";

export interface UserTokenPayload {
    sub: string;
    iss: string;
    email: string;
    firstName: string;
    lastName: string;
}

const privateKey = Buffer.from(JWT_PRIVATE_KEY_BASE64, "base64").toString(
    "ascii"
);

const publicKey = Buffer.from(JWT_PUBLIC_KEY_BASE64, "base64").toString(
    "ascii"
);

const keyId = JWT_KEY_ID;
const ALGORITHM = "ES256"; // Elliptic Curve algorithm

const refreshTokenExpiresIn = REFRESH_TOKEN_EXPIRES_IN as ms.StringValue;
const accessTokenExpiresIn = ACCESS_TOKEN_EXPIRES_IN as ms.StringValue;

if (
    !privateKey ||
    !publicKey ||
    !keyId ||
    !refreshTokenExpiresIn ||
    !accessTokenExpiresIn
) {
    throw new Error(
        "JWT keys are not set correctly. Please generate them with `bun run generate` and copy them into the .env file"
    );
}

/**
 * Signs a payload to generate a short-lived ACCESS token.
 */
export function generateAccessToken(payload: UserTokenPayload): string {
    return jwt.sign(payload, privateKey, {
        algorithm: ALGORITHM,
        expiresIn: accessTokenExpiresIn, // Use short lifetime
        keyid: keyId,
    });
}

/**
 * Signs a payload to generate a long-lived REFRESH token.
 */
export function generateRefreshToken(payload: UserTokenPayload): string {
    return jwt.sign(payload, privateKey, {
        algorithm: ALGORITHM,
        expiresIn: refreshTokenExpiresIn, // Use long lifetime
        keyid: keyId,
    });
}

/**
 * Verifies any JWT (access or refresh) using the PUBLIC key.
 * @returns The decoded payload, or null if verification fails.
 */
export function verifyToken(token: string): UserTokenPayload | null {
    try {
        const decoded = jwt.verify(token, publicKey, {
            algorithms: [ALGORITHM],
        });
        return decoded as UserTokenPayload;
    } catch (error) {
        // This will catch expired tokens, invalid signatures, etc.
        // console.error("JWT Verification failed:", error.name);
        return null;
    }
}
