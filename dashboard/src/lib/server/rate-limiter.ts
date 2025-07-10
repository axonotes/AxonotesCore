import NodeCache from "node-cache";
import {
    DEVICE_FLOW_MAX_REQUESTS,
    DEVICE_FLOW_WEIGHT,
    DEVICE_FLOW_WINDOW_MS,
    DEVICE_POLL_INTERVAL,
    DEVICE_POLL_MAX_REQUESTS,
    DEVICE_POLL_WEIGHT,
    DEVICE_POLL_WINDOW_MS,
    RATE_LIMIT_CACHE_TTL,
    RATE_LIMIT_CHECK_PERIOD,
    TOKEN_EXCHANGE_MAX_REQUESTS,
    TOKEN_EXCHANGE_WEIGHT,
    TOKEN_EXCHANGE_WINDOW_MS,
    TOKEN_REFRESH_MAX_REQUESTS,
    TOKEN_REFRESH_WEIGHT,
    TOKEN_REFRESH_WINDOW_MS,
} from "$env/static/private";

interface RateLimitWindow {
    count: number;
    windowStart: number;
    lastRequest: number;
}

interface RateLimitConfig {
    windowMs: number; // Window size in milliseconds
    maxRequests: number; // Max requests per window
    weight: number; // Weight of this operation (1-10)
}

class WeightedRateLimiter {
    private cache: NodeCache;
    private configs: Map<string, RateLimitConfig>;
    private pollInterval: number; // Dynamic poll interval from device flow

    constructor() {
        this.cache = new NodeCache({
            stdTTL: parseInt(RATE_LIMIT_CACHE_TTL || "3600"), // 1 hour default
            checkperiod: parseInt(RATE_LIMIT_CHECK_PERIOD || "60"),
        });

        // Get poll interval from environment or default to 5 seconds
        this.pollInterval = parseInt(DEVICE_POLL_INTERVAL || "5");

        this.configs = new Map([
            // Device flow creation - most expensive
            [
                "device_flow",
                {
                    windowMs: parseInt(DEVICE_FLOW_WINDOW_MS || "60000"), // 1 minute
                    maxRequests: parseInt(DEVICE_FLOW_MAX_REQUESTS || "5"),
                    weight: parseInt(DEVICE_FLOW_WEIGHT || "5"),
                },
            ],

            // Polling - separate slow_down from rate limiting
            [
                "device_poll",
                {
                    windowMs: parseInt(DEVICE_POLL_WINDOW_MS || "60000"), // 1 minute window
                    maxRequests: parseInt(DEVICE_POLL_MAX_REQUESTS || "12"), // 12 requests per minute (1 every 5 seconds)
                    weight: parseInt(DEVICE_POLL_WEIGHT || "1"), // Weight of 1 for simple counting
                },
            ],

            // Token exchange - expensive but one-time
            [
                "token_exchange",
                {
                    windowMs: parseInt(TOKEN_EXCHANGE_WINDOW_MS || "60000"), // 1 minute
                    maxRequests: parseInt(TOKEN_EXCHANGE_MAX_REQUESTS || "10"),
                    weight: parseInt(TOKEN_EXCHANGE_WEIGHT || "3"),
                },
            ],

            // Token refresh - light
            [
                "token_refresh",
                {
                    windowMs: parseInt(TOKEN_REFRESH_WINDOW_MS || "60000"), // 1 minute
                    maxRequests: parseInt(TOKEN_REFRESH_MAX_REQUESTS || "20"),
                    weight: parseInt(TOKEN_REFRESH_WEIGHT || "1"),
                },
            ],
        ]);
    }

    // Set poll interval dynamically (called from device flow response)
    setPollInterval(intervalSeconds: number) {
        this.pollInterval = intervalSeconds;
    }

    async checkLimit(
        identifier: string,
        operation: string,
        customConfig?: Partial<RateLimitConfig>
    ): Promise<{
        allowed: boolean;
        retryAfter?: number;
        shouldSlowDown?: boolean;
    }> {
        const config = customConfig
            ? {...this.configs.get(operation)!, ...customConfig}
            : this.configs.get(operation);

        if (!config) {
            throw new Error(`Unknown operation: ${operation}`);
        }

        const key = `${operation}:${identifier}`;
        const now = Date.now();

        let window = this.cache.get<RateLimitWindow>(key);

        if (!window) {
            window = {
                count: 0,
                windowStart: now,
                lastRequest: 0, // Initialize to 0 so first request always passes
            };
        }

        // Special handling for device polling - check slow_down FIRST
        if (operation === "device_poll" && window.lastRequest > 0) {
            const timeSinceLastRequest = now - window.lastRequest;
            const slowDownThreshold = this.pollInterval * 1000 * 1.1; // 110% of poll interval in ms

            if (timeSinceLastRequest < slowDownThreshold) {
                const retryAfter = Math.ceil(
                    (slowDownThreshold - timeSinceLastRequest) / 1000
                );

                // DON'T update lastRequest for slow_down - this was causing the infinite loop
                return {
                    allowed: false,
                    shouldSlowDown: true,
                    retryAfter,
                };
            }
        }

        // Check if we need to reset the window
        if (now - window.windowStart >= config.windowMs) {
            window = {
                count: 0,
                windowStart: now,
                lastRequest: window.lastRequest, // Keep last request time across window resets
            };
        }

        // Apply weighted counting for rate limiting
        const weightedCount = window.count + config.weight;

        // Check rate limit
        if (weightedCount > config.maxRequests) {
            const requestsPerSecond =
                config.maxRequests / (config.windowMs / 1000);
            const retryAfter = Math.ceil(config.weight / requestsPerSecond);

            return {allowed: false, retryAfter};
        }

        // Update window for successful request - THIS is where we update lastRequest
        window.count = weightedCount;
        window.lastRequest = now;

        // Store with TTL
        this.cache.set(key, window, Math.ceil(config.windowMs / 1000));

        return {allowed: true};
    }

    // Get client identifier from request
    getClientIdentifier(request: Request): string {
        // Try to get real IP from headers (for proxies)
        const forwardedFor = request.headers.get("x-forwarded-for");
        const realIp = request.headers.get("x-real-ip");
        const cfConnectingIp = request.headers.get("cf-connecting-ip");

        return (
            cfConnectingIp || realIp || forwardedFor?.split(",")[0] || "unknown"
        );
    }

    // Get current poll interval
    getPollInterval(): number {
        return this.pollInterval;
    }

    // Debug method to inspect current state
    debug(identifier: string, operation: string): any {
        const key = `${operation}:${identifier}`;
        const window = this.cache.get<RateLimitWindow>(key);
        return {
            key,
            window,
            pollInterval: this.pollInterval,
            config: this.configs.get(operation),
            now: Date.now(),
        };
    }
}

export const rateLimiter = new WeightedRateLimiter();
