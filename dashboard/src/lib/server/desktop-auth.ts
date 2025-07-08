import NodeCache from "node-cache";
import {v4 as uuidv4} from "uuid";
import {
    generateDesktopTokens,
    getTokenLifetimes,
    type UserTokenPayload,
} from "./jwt";
import {DESKTOP_AUTH_SESSION_TTL} from "$env/static/private";

interface DesktopAuthSession {
    sessionId: string;
    userId?: string;
    email?: string;
    accessToken?: string;
    refreshToken?: string;
    expiresAt: number;
    consumed: boolean;
    createdAt: number;
}

// Initialize cache with TTL (default 5 minutes)
const sessionCache = new NodeCache({
    stdTTL: parseInt(DESKTOP_AUTH_SESSION_TTL || "300"), // 5 minutes
    checkperiod: 60, // Check for expired keys every minute
    deleteOnExpire: true, // Auto-delete expired keys
});

export class DesktopAuthManager {
    private static instance: DesktopAuthManager;

    private constructor() {}

    static getInstance(): DesktopAuthManager {
        if (!this.instance) {
            this.instance = new DesktopAuthManager();
        }
        return this.instance;
    }

    async createSession(): Promise<string> {
        const sessionId = uuidv4();

        const session: DesktopAuthSession = {
            sessionId,
            expiresAt: Date.now() + 5 * 60 * 1000, // 5 minutes
            consumed: false,
            createdAt: Date.now(),
        };

        sessionCache.set(sessionId, session);

        // Clean up expired sessions periodically
        this.cleanupExpiredSessions();

        return sessionId;
    }

    async completeAuth(
        sessionId: string,
        userPayload: UserTokenPayload
    ): Promise<void> {
        const session = sessionCache.get<DesktopAuthSession>(sessionId);

        if (!session) {
            throw new Error("Session not found or expired");
        }

        if (session.consumed) {
            throw new Error("Session already consumed");
        }

        if (Date.now() > session.expiresAt) {
            sessionCache.del(sessionId);
            throw new Error("Session expired");
        }

        // Generate desktop-specific tokens (longer lifetimes)
        const {accessToken, refreshToken} = generateDesktopTokens(userPayload);

        // Update session
        const updatedSession: DesktopAuthSession = {
            ...session,
            userId: userPayload.sub,
            email: userPayload.email,
            accessToken,
            refreshToken,
            consumed: false, // Keep false until desktop app retrieves tokens
        };

        sessionCache.set(sessionId, updatedSession);
    }

    async getTokens(sessionId: string): Promise<{
        accessToken: string;
        refreshToken: string;
        expiresAt: number;
        user: {id: string; email: string};
        workosSessionId?: string;
    } | null> {
        const session = sessionCache.get<DesktopAuthSession>(sessionId);

        if (!session) {
            return null;
        }

        if (session.consumed) {
            throw new Error("Session already consumed");
        }

        if (Date.now() > session.expiresAt) {
            sessionCache.del(sessionId);
            throw new Error("Session expired");
        }

        if (!session.accessToken || !session.refreshToken) {
            throw new Error("Session not completed");
        }

        // Mark as consumed
        session.consumed = true;
        sessionCache.set(sessionId, session);

        // Clean up after a short delay
        setTimeout(() => {
            sessionCache.del(sessionId);
        }, 30000);

        // Get desktop token lifetimes
        const {accessTokenMs} = getTokenLifetimes(true);

        return {
            accessToken: session.accessToken,
            refreshToken: session.refreshToken,
            expiresAt: Date.now() + accessTokenMs,
            user: {
                id: session.userId!,
                email: session.email!,
            },
            workosSessionId: session.sessionId,
        };
    }

    async markConsumed(sessionId: string): Promise<void> {
        const session = sessionCache.get<DesktopAuthSession>(sessionId);
        if (session) {
            session.consumed = true;
            sessionCache.set(sessionId, session);
        }
    }

    private cleanupExpiredSessions(): void {
        // Get all keys and check if they're expired
        const keys = sessionCache.keys();
        const now = Date.now();

        keys.forEach((key) => {
            const session = sessionCache.get<DesktopAuthSession>(key);
            if (session && now > session.expiresAt) {
                sessionCache.del(key);
            }
        });
    }
}

export const desktopAuthManager = DesktopAuthManager.getInstance();
