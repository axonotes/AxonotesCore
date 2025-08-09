import NodeCache from "node-cache";
import {v4 as uuidv4} from "uuid";
import {createHash, randomBytes} from "crypto";
import {
    generateTokens,
    getTokenLifetimes,
    type UserTokenPayload,
} from "./jwt";
import {
    DESKTOP_AUTH_SESSION_TTL,
    DEVICE_CODE_TTL_MS,
    DEVICE_POLL_INTERVAL,
    JWT_ISSUER,
} from "$env/static/private";
import {rateLimiter} from "$lib/server/rate-limiter";

interface DeviceFlowSession {
    device_code: string;
    user_code: string;
    code_challenge: string;
    code_challenge_method: "S256";
    status: "pending" | "complete" | "expired";
    user_data?: UserTokenPayload;
    tokens?: {
        access_token: string;
        refresh_token: string;
    };
    expires_at: number;
    interval: number;
    created_at: number;
}

interface DeviceAuthResponse {
    device_code: string;
    user_code: string;
    verification_uri: string;
    verification_uri_complete: string;
    expires_in: number;
    interval: number;
}

interface TokenResponse {
    access_token: string;
    token_type: "Bearer";
    expires_in: number;
    refresh_token: string;
    scope: string;
}

interface PollResponse {
    error?: string;
    error_description?: string;
}

// Initialize cache with TTL (default 30 minutes)
const sessionCache = new NodeCache({
    stdTTL: parseInt(DESKTOP_AUTH_SESSION_TTL || "600"), // 10 minutes
    checkperiod: 60, // Check for expired keys every minute
    deleteOnExpire: true,
});

export class DesktopAuthManager {
    private static instance: DesktopAuthManager;
    private baseUrl: string;
    private readonly DEVICE_CODE_TTL = parseInt(DEVICE_CODE_TTL_MS || "600000"); // 10 minutes default
    private readonly POLL_INTERVAL = parseInt(DEVICE_POLL_INTERVAL || "5"); // 5 seconds default

    // User-friendly character set (excludes confusing characters like 0, I, O)
    private readonly USER_CODE_CHARS = "123456789ABCDEFGHJKLMNOPQRSTUVWXYZ";

    private constructor(baseUrl: string = "https://app.axonotes.ch") {
        this.baseUrl = baseUrl;
    }

    static getInstance(baseUrl?: string): DesktopAuthManager {
        if (!this.instance) {
            this.instance = new DesktopAuthManager(baseUrl);
        }
        return this.instance;
    }

    /**
     * Generate cryptographically secure 8-character user code
     * Uses user-friendly character set (no 0, I, O)
     * Format: XXXX-XXXX for display
     */
    private generateUserCode(): string {
        const chars = this.USER_CODE_CHARS;
        let result = "";

        for (let i = 0; i < 8; i++) {
            const randomIndex = randomBytes(1)[0] % chars.length;
            result += chars[randomIndex];
        }

        return result;
    }

    /**
     * Format user code for display with dash
     */
    private formatUserCode(userCode: string): string {
        return `${userCode.slice(0, 4)}-${userCode.slice(4, 8)}`;
    }

    /**
     * Initiate device flow - equivalent to POST /oauth/device/authorize
     */
    async initiateDeviceFlow(
        code_challenge: string,
        client_id?: string,
        scope?: string
    ): Promise<DeviceAuthResponse> {
        const device_code = uuidv4();
        const user_code = this.generateUserCode();
        const expires_at = Date.now() + this.DEVICE_CODE_TTL;

        const session: DeviceFlowSession = {
            device_code,
            user_code,
            code_challenge,
            code_challenge_method: "S256",
            status: "pending",
            expires_at,
            interval: this.POLL_INTERVAL,
            created_at: Date.now(),
        };

        // Store session by both device_code and user_code for easy lookup
        sessionCache.set(device_code, session);
        sessionCache.set(`user_${user_code}`, session);

        // Sync poll interval with rate limiter
        rateLimiter.setPollInterval(this.POLL_INTERVAL);

        const verification_uri = `${this.baseUrl}/auth/device`;
        const verification_uri_complete = `${verification_uri}?user_code=${this.formatUserCode(user_code)}`;

        return {
            device_code,
            user_code: this.formatUserCode(user_code),
            verification_uri,
            verification_uri_complete,
            expires_in: Math.floor(this.DEVICE_CODE_TTL / 1000),
            interval: this.POLL_INTERVAL,
        };
    }

    /**
     * Poll device status - equivalent to GET /oauth/device/token
     */
    async pollDeviceStatus(device_code: string): Promise<PollResponse> {
        const session = sessionCache.get<DeviceFlowSession>(device_code);

        if (!session) {
            return {
                error: "expired_token",
                error_description: "The device code has expired",
            };
        }

        if (Date.now() > session.expires_at) {
            sessionCache.del(device_code);
            sessionCache.del(`user_${session.user_code}`);
            return {
                error: "expired_token",
                error_description: "The device code has expired",
            };
        }

        switch (session.status) {
            case "pending":
                return {
                    error: "authorization_pending",
                    error_description:
                        "The authorization request is still pending",
                };
            case "complete":
                return {}; // No error means ready for token exchange
            case "expired":
                return {
                    error: "expired_token",
                    error_description: "The device code has expired",
                };
            default:
                return {
                    error: "invalid_grant",
                    error_description: "Invalid device code",
                };
        }
    }

    /**
     * Complete device authentication from web UI
     */
    async completeDeviceAuth(
        user_code: string,
        user_data: UserTokenPayload
    ): Promise<void> {
        // Remove dashes from user code for lookup
        const cleanUserCode = user_code.replace("-", "");
        const session = sessionCache.get<DeviceFlowSession>(
            `user_${cleanUserCode}`
        );

        if (!session) {
            throw new Error("Invalid or expired user code");
        }

        if (Date.now() > session.expires_at) {
            sessionCache.del(session.device_code);
            sessionCache.del(`user_${cleanUserCode}`);
            throw new Error("User code has expired");
        }

        if (session.status !== "pending") {
            throw new Error("Authorization already completed or expired");
        }

        // Generate tokens
        const {accessToken, refreshToken} = generateTokens(user_data);

        // Update session
        const updatedSession: DeviceFlowSession = {
            ...session,
            status: "complete",
            user_data,
            tokens: {
                access_token: accessToken,
                refresh_token: refreshToken,
            },
        };

        sessionCache.set(session.device_code, updatedSession);
        sessionCache.set(`user_${cleanUserCode}`, updatedSession);
    }

    /**
     * Exchange device code for tokens with PKCE validation
     */
    async exchangeDeviceToken(
        device_code: string,
        code_verifier: string
    ): Promise<TokenResponse> {
        const session = sessionCache.get<DeviceFlowSession>(device_code);

        if (!session) {
            throw new Error("Invalid or expired device code");
        }

        if (Date.now() > session.expires_at) {
            sessionCache.del(device_code);
            sessionCache.del(`user_${session.user_code}`);
            throw new Error("Device code has expired");
        }

        if (session.status !== "complete") {
            throw new Error("Authorization not yet complete");
        }

        // Validate PKCE
        const challenge = createHash("sha256")
            .update(code_verifier)
            .digest("base64url");

        if (challenge !== session.code_challenge) {
            throw new Error("Invalid code verifier");
        }

        if (!session.tokens) {
            throw new Error("Tokens not available");
        }

        // Clean up session (one-time use)
        sessionCache.del(device_code);
        sessionCache.del(`user_${session.user_code}`);

        const {accessTokenMs} = getTokenLifetimes(true);

        return {
            access_token: session.tokens.access_token,
            token_type: "Bearer",
            expires_in: Math.floor(accessTokenMs / 1000),
            refresh_token: session.tokens.refresh_token,
            scope: "openid profile email",
        };
    }

    /**
     * Validate user code format and existence
     */
    async validateUserCode(user_code: string): Promise<boolean> {
        // Remove dashes and validate format
        const cleanUserCode = user_code.replace("-", "");

        // Updated regex to match our character set
        if (!/^[123456789ABCDEFGHJKLMNOPQRSTUVWXYZ]{8}$/.test(cleanUserCode)) {
            return false;
        }

        const session = sessionCache.get<DeviceFlowSession>(
            `user_${cleanUserCode}`
        );

        if (!session) {
            return false;
        }

        if (Date.now() > session.expires_at) {
            sessionCache.del(session.device_code);
            sessionCache.del(`user_${cleanUserCode}`);
            return false;
        }

        return session.status === "pending";
    }

    /**
     * Get session info by user code (for web UI)
     */
    async getSessionByUserCode(
        user_code: string
    ): Promise<DeviceFlowSession | null> {
        const cleanUserCode = user_code.replace("-", "");
        const session = sessionCache.get<DeviceFlowSession>(
            `user_${cleanUserCode}`
        );

        if (!session) {
            return null;
        }

        if (Date.now() > session.expires_at) {
            sessionCache.del(session.device_code);
            sessionCache.del(`user_${cleanUserCode}`);
            return null;
        }

        return session;
    }

    /**
     * Clean up expired sessions
     */
    private cleanupExpiredSessions(): void {
        const keys = sessionCache.keys();
        const now = Date.now();

        keys.forEach((key) => {
            const session = sessionCache.get<DeviceFlowSession>(key);
            if (session && now > session.expires_at) {
                sessionCache.del(key);
                // Also clean up the corresponding user code key
                if (!key.startsWith("user_")) {
                    sessionCache.del(`user_${session.user_code}`);
                }
            }
        });
    }
}

// Export singleton instance
export const desktopAuthManager = DesktopAuthManager.getInstance(JWT_ISSUER);
