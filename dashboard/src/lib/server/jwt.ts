import jwt from "jsonwebtoken";
import {
    JWT_KEY_ID,
    JWT_PRIVATE_KEY_BASE64,
    JWT_PUBLIC_KEY_BASE64,
    WEB_ACCESS_TOKEN_EXPIRES_IN,
    WEB_REFRESH_TOKEN_EXPIRES_IN,
    DESKTOP_ACCESS_TOKEN_EXPIRES_IN,
    DESKTOP_REFRESH_TOKEN_EXPIRES_IN,
} from "$env/static/private";
import ms from "ms";

export interface UserTokenPayload {
    sub: string;
    iss: string;
    email: string;
    firstName: string;
    lastName: string;
}

export interface TokenOptions {
    accessTokenLifetime?: ms.StringValue;
    refreshTokenLifetime?: ms.StringValue;
}

// Web token lifetimes
const DEFAULT_WEB_ACCESS_LIFETIME =
    WEB_ACCESS_TOKEN_EXPIRES_IN as ms.StringValue;
const DEFAULT_WEB_REFRESH_LIFETIME =
    WEB_REFRESH_TOKEN_EXPIRES_IN as ms.StringValue;

// Desktop token lifetimes (longer for better UX)
const DEFAULT_DESKTOP_ACCESS_LIFETIME =
    DESKTOP_ACCESS_TOKEN_EXPIRES_IN as ms.StringValue;
const DEFAULT_DESKTOP_REFRESH_LIFETIME =
    DESKTOP_REFRESH_TOKEN_EXPIRES_IN as ms.StringValue;

const privateKey = Buffer.from(JWT_PRIVATE_KEY_BASE64, "base64").toString(
    "ascii"
);
const publicKey = Buffer.from(JWT_PUBLIC_KEY_BASE64, "base64").toString(
    "ascii"
);
const keyId = JWT_KEY_ID;
const ALGORITHM = "ES256";

if (
    !privateKey ||
    !publicKey ||
    !keyId ||
    !ALGORITHM ||
    !DEFAULT_WEB_ACCESS_LIFETIME ||
    !DEFAULT_WEB_REFRESH_LIFETIME ||
    !DEFAULT_DESKTOP_ACCESS_LIFETIME ||
    !DEFAULT_DESKTOP_REFRESH_LIFETIME
) {
    throw new Error(
        "JWT keys are not set correctly. Please generate them with `bun run generate` and copy them into the .env file"
    );
}

/**
 * Clean JWT payload by removing JWT-specific claims
 * This prevents conflicts when generating new tokens
 */
function cleanJWTPayload(payload: any): UserTokenPayload {
    const {exp, iat, nbf, jti, ...cleanPayload} = payload;
    return cleanPayload as UserTokenPayload;
}

/**
 * Signs a payload to generate a short-lived ACCESS token.
 */
export function generateAccessToken(
    payload: UserTokenPayload,
    options: TokenOptions = {}
): string {
    const expiresIn =
        options.accessTokenLifetime || DEFAULT_WEB_ACCESS_LIFETIME;

    // Clean the payload to remove any existing JWT claims
    const cleanPayload = cleanJWTPayload(payload);

    return jwt.sign(cleanPayload, privateKey, {
        algorithm: ALGORITHM,
        expiresIn,
        keyid: keyId,
    });
}

/**
 * Signs a payload to generate a long-lived REFRESH token.
 */
export function generateRefreshToken(
    payload: UserTokenPayload,
    options: TokenOptions = {}
): string {
    const expiresIn =
        options.refreshTokenLifetime || DEFAULT_WEB_REFRESH_LIFETIME;

    // Clean the payload to remove any existing JWT claims
    const cleanPayload = cleanJWTPayload(payload);

    return jwt.sign(cleanPayload, privateKey, {
        algorithm: ALGORITHM,
        expiresIn,
        keyid: keyId,
    });
}

/**
 * Generate tokens specifically for web clients (shorter lifetimes)
 */
export function generateWebTokens(payload: UserTokenPayload) {
    return {
        accessToken: generateAccessToken(payload, {
            accessTokenLifetime: DEFAULT_WEB_ACCESS_LIFETIME,
            refreshTokenLifetime: DEFAULT_WEB_REFRESH_LIFETIME,
        }),
        refreshToken: generateRefreshToken(payload, {
            accessTokenLifetime: DEFAULT_WEB_ACCESS_LIFETIME,
            refreshTokenLifetime: DEFAULT_WEB_REFRESH_LIFETIME,
        }),
    };
}

/**
 * Generate tokens specifically for desktop clients (longer lifetimes)
 */
export function generateDesktopTokens(payload: UserTokenPayload) {
    return {
        accessToken: generateAccessToken(payload, {
            accessTokenLifetime: DEFAULT_DESKTOP_ACCESS_LIFETIME,
            refreshTokenLifetime: DEFAULT_DESKTOP_REFRESH_LIFETIME,
        }),
        refreshToken: generateRefreshToken(payload, {
            accessTokenLifetime: DEFAULT_DESKTOP_ACCESS_LIFETIME,
            refreshTokenLifetime: DEFAULT_DESKTOP_REFRESH_LIFETIME,
        }),
    };
}

/**
 * Get token lifetime in milliseconds for client use
 */
export function getTokenLifetimes(isDesktop: boolean = false) {
    if (isDesktop) {
        return {
            accessTokenMs: ms(DEFAULT_DESKTOP_ACCESS_LIFETIME),
            refreshTokenMs: ms(DEFAULT_DESKTOP_REFRESH_LIFETIME),
        };
    }

    return {
        accessTokenMs: ms(DEFAULT_WEB_ACCESS_LIFETIME),
        refreshTokenMs: ms(DEFAULT_WEB_REFRESH_LIFETIME),
    };
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
    } catch (_error) {
        return null;
    }
}
