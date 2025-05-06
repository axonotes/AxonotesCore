import NodeCache from "node-cache";
import type {Handle} from "@sveltejs/kit";

// --- Configuration ---
const WINDOW_SIZE_IN_SECONDS: number = 60; // Duration of each fixed window/bucket
const MAX_REQUESTS_PER_WINDOW_PERIOD: number = 20; // Max requests in a conceptual floating window

// --- Types ---
interface WindowData {
    previousWindowCount: number;
    currentWindowCount: number;
    currentWindowStartTime: number; // Timestamp (ms) when the current fixed window started
}

interface RateLimitResult {
    success: boolean;
    limit: number; // This will be MAX_REQUESTS_PER_WINDOW_PERIOD
    // 'remaining' is harder to calculate accurately for approximate floating windows
    // without more complex logic, so we might omit it or provide an estimate.
    // For simplicity, we'll focus on success/failure and Retry-After.
    retryAfter?: number;
}

// --- Cache Initialization ---
// The stdTTL should be at least 2x WINDOW_SIZE_IN_SECONDS to keep previous window data.
// checkperiod can be tuned.
const ipRateDataCache = new NodeCache({
    stdTTL: WINDOW_SIZE_IN_SECONDS * 3, // Keep data for 3 window periods
    checkperiod: WINDOW_SIZE_IN_SECONDS,
    useClones: true,
});

const WINDOW_SIZE_IN_MILLISECONDS = WINDOW_SIZE_IN_SECONDS * 1000;

// --- Rate Limiting Logic (Approximate Floating Window) ---
async function approximateFloatingWindowRateLimit(
    ip: string,
): Promise<RateLimitResult> {
    const now = Date.now();
    let data: WindowData | undefined = ipRateDataCache.get(ip);

    if (!data) {
        // First request from this IP or data expired
        data = {
            previousWindowCount: 0,
            currentWindowCount: 1, // Count this request
            currentWindowStartTime: now, // Start new window now
        };
    } else {
        const timeSinceCurrentWindowStart = now - data.currentWindowStartTime;

        if (timeSinceCurrentWindowStart >= WINDOW_SIZE_IN_MILLISECONDS) {
            // Current window has expired, roll over
            if (
                timeSinceCurrentWindowStart >=
                2 * WINDOW_SIZE_IN_MILLISECONDS
            ) {
                // More than two full windows have passed since last request in this window
                data.previousWindowCount = 0;
            } else {
                // Only one full window passed
                data.previousWindowCount = data.currentWindowCount;
            }
            data.currentWindowCount = 1; // This request is the first in the new window
            // Align the new window start time.
            // This ensures windows are fixed buckets rather than sliding with each request.
            data.currentWindowStartTime =
                data.currentWindowStartTime +
                Math.floor(
                    timeSinceCurrentWindowStart / WINDOW_SIZE_IN_MILLISECONDS,
                ) *
                WINDOW_SIZE_IN_MILLISECONDS;
        } else {
            // Still within the current window
            data.currentWindowCount++;
        }
    }

    // Calculate the approximate count for the "floating" window
    const timeElapsedInCurrentWindow = now - data.currentWindowStartTime;
    const weightOfPreviousWindow =
        (WINDOW_SIZE_IN_MILLISECONDS - timeElapsedInCurrentWindow) /
        WINDOW_SIZE_IN_MILLISECONDS;

    // Ensure weight is not negative if 'now' is slightly past due to processing delays
    const effectivePreviousCount =
        data.previousWindowCount * Math.max(0, weightOfPreviousWindow);
    const approximateCount = data.currentWindowCount + effectivePreviousCount;

    ipRateDataCache.set(ip, data); // Store updated data

    if (approximateCount > MAX_REQUESTS_PER_WINDOW_PERIOD) {
        // Calculate Retry-After: time until the current fixed window slot ends
        // This is a simplification; a more precise Retry-After would be complex.
        const timeRemainingInCurrentFixedWindow = Math.max(
            0,
            WINDOW_SIZE_IN_MILLISECONDS - timeElapsedInCurrentWindow,
        );
        // Or, time until enough requests from previous window "age out"
        // For simplicity, let's suggest waiting for roughly half a window or end of current.
        const retryAfterSeconds = Math.ceil(
            (timeRemainingInCurrentFixedWindow > 0
                ? timeRemainingInCurrentFixedWindow
                : WINDOW_SIZE_IN_MILLISECONDS / 2) / 1000,
        );

        return {
            success: false,
            limit: MAX_REQUESTS_PER_WINDOW_PERIOD,
            retryAfter: retryAfterSeconds > 0 ? retryAfterSeconds : 1,
        };
    }

    return {
        success: true,
        limit: MAX_REQUESTS_PER_WINDOW_PERIOD,
    };
}

// --- SvelteKit Server Hook ---
export const handle: Handle = async ({ event, resolve }) => {
    const ip = event.getClientAddress();

    if (
        event.url.pathname.startsWith("/assets/") ||
        event.url.pathname.startsWith("/favicon.ico")
    ) {
        return resolve(event);
    }

    const { success, limit, retryAfter } =
        await approximateFloatingWindowRateLimit(ip);

    // Set X-RateLimit-Limit header on all relevant responses
    event.setHeaders({
        "X-RateLimit-Limit": limit.toString(),
    });

    if (!success) {
        const headers = new Headers();
        if (retryAfter) {
            headers.set("Retry-After", retryAfter.toString());
        }
        // Ensure X-RateLimit-Limit is also on the error response
        headers.set("X-RateLimit-Limit", limit.toString());
        headers.set("X-RateLimit-Remaining", "0"); // Explicitly 0 when limited

        return new Response("Too Many Requests", {
            status: 429,
            headers: headers,
        });
    }

    return await resolve(event);
};
