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
    client_type: "web" | "desktop";
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
 * Return a UserTokenPayload with standard JWT claim fields removed.
 *
 * Removes `exp`, `iat`, `nbf`, and `jti` from a decoded JWT payload to avoid
 * claim conflicts when creating a new token.
 *
 * @param payload - Decoded JWT payload (may include standard JWT claims)
 * @returns The payload cast to `UserTokenPayload` without `exp`, `iat`, `nbf`, or `jti`
 */
function cleanJWTPayload(payload: any): UserTokenPayload {
    const {exp, iat, nbf, jti, ...cleanPayload} = payload;
    return cleanPayload as UserTokenPayload;
}

/**
 * Generate a signed, short-lived access JWT for the given user payload.
 *
 * The function removes any existing JWT claim fields (exp, iat, nbf, jti) from
 * `payload` before signing. Use `options.accessTokenLifetime` to override the
 * default web access token lifetime.
 *
 * @param payload - The user JWT payload; claim fields will be stripped before signing.
 * @param options - Optional lifetimes to override defaults (only `accessTokenLifetime` is used).
 * @returns A signed JWT access token string.
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
 * Generate a signed refresh JWT for the given user payload.
 *
 * The payload is sanitized to remove any existing JWT claim fields before signing.
 * The token lifetime is taken from `options.refreshTokenLifetime` when provided,
 * otherwise the module's default refresh lifetime is used.
 *
 * @param payload - The user payload to embed in the token (will be cleaned of standard JWT claims)
 * @param options - Optional overrides for token lifetimes
 * @returns A signed JWT string representing a refresh token
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
 * Generate an access token and refresh token using the web (shorter) lifetimes.
 *
 * @returns An object containing `accessToken` and `refreshToken` as signed JWT strings.
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
 * Create an access token and a refresh token using the desktop (longer) lifetimes.
 *
 * The provided `payload` is signed into both tokens and intended for desktop clients.
 *
 * @param payload - JWT payload to embed in the tokens; `client_type` is expected to indicate the client (e.g., `"desktop"`).
 * @returns An object with `accessToken` and `refreshToken` strings signed with the desktop token lifetimes.
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
 * Generate an access token and refresh token for the given user payload.
 *
 * Chooses desktop token lifetimes when `payload.client_type === "desktop"`, otherwise uses web lifetimes.
 *
 * @param payload - The user JWT payload; its `client_type` field controls which token lifetimes are used.
 * @returns An object with `accessToken` and `refreshToken` strings.
 */
export function generateTokens(payload: UserTokenPayload) {
    if (payload.client_type === "desktop") {
        return generateDesktopTokens(payload);
    } else {
        return generateWebTokens(payload);
    }
}

/**
 * Return access and refresh token lifetimes in milliseconds for the specified client type.
 *
 * @param isDesktop - If true, use desktop token lifetimes; otherwise use web lifetimes.
 * @returns An object with `accessTokenMs` and `refreshTokenMs` (both numbers, milliseconds).
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
 * Verifies a JWT (access or refresh) and returns the decoded payload.
 *
 * Verification includes signature and standard claim checks; returns `null` if the token is invalid,
 * expired, or cannot be decoded.
 *
 * @returns The decoded `UserTokenPayload` on success, or `null` on verification failure.
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
