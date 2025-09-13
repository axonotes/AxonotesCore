import SqlString from "sqlstring";
import {writable, get} from "svelte/store";
import {browser} from "$app/environment";
import {DbConnection, User} from "$lib/module_bindings";
import {
    PUBLIC_ACCESS_TOKEN_COOKIE_NAME,
    PUBLIC_SPACETIME_MODULE_NAME,
    PUBLIC_SPACETIME_WS,
} from "$env/static/public";
import Cookies from "js-cookie";
import {goto} from "$app/navigation";

export const sql = SqlString.format;

type SpacetimeState = {
    connection: DbConnection | null;
    status: "disconnected" | "connecting" | "connected" | "error";
    error?: string; // Optional error message
};

export const spacetime = writable<SpacetimeState>({
    connection: null,
    status: "disconnected",
});

let _connectionInstance: DbConnection | null = null;

/**
 * Attempts to obtain a fresh access token from the server and store it in a cookie.
 *
 * Sends a POST request to "/api/auth/refresh"; on success stores the returned `accessToken`
 * in the PUBLIC_ACCESS_TOKEN_COOKIE_NAME cookie and resolves `true`. If the HTTP response is
 * not OK or a network/error occurs, resolves `false`.
 *
 * @returns `true` when a new access token was successfully retrieved and stored, otherwise `false`.
 */
async function refreshAccessToken(): Promise<boolean> {
    try {
        const response = await fetch("/api/auth/refresh", {method: "POST"});
        if (!response.ok) {
            console.error("Token refresh failed with status:", response.status);
            return false;
        }

        const data = await response.json();
        const newAccessToken = data.accessToken;
        Cookies.set(PUBLIC_ACCESS_TOKEN_COOKIE_NAME, newAccessToken);
        console.log("Access token refreshed successfully.");
        return true;
    } catch (error) {
        console.error("Failed to fetch new access token:", error);
        return false;
    }
}

/**
 * Starts a Spacetime DB connection if running in the browser and no connection is active.
 *
 * Attempts to read an access token from cookies, optionally retries token refresh up to
 * `maxRefreshCalls` times (500ms between attempts), and then establishes a DbConnection using
 * the token. On successful connection the spacetime store is updated to `connected` and a
 * subscription is created to load the current user by identity. On disconnect or connection
 * error the function will try to refresh the token and reconnect; if refresh fails it sets the
 * store to an `error` state and redirects the user to `/login`.
 *
 * @param maxRefreshCalls - Maximum recursive refresh attempts to perform when no access token is present (default: 5).
 */
export function connectToSpacetime(maxRefreshCalls: number = 5) {
    if (!browser || get(spacetime).status !== "disconnected") return;

    const accessToken = Cookies.get(PUBLIC_ACCESS_TOKEN_COOKIE_NAME);
    if (!accessToken) {
        if (maxRefreshCalls > 0) {
            refreshAccessToken().then(() => {
                setTimeout(() => {
                    connectToSpacetime(maxRefreshCalls - 1);
                }, 500);
            });
        } else {
            console.error("No AccessToken found. User needs to login.");
        }
        return;
    }

    spacetime.update((s) => ({...s, status: "connecting", error: undefined}));

    _connectionInstance = DbConnection.builder()
        .withUri(PUBLIC_SPACETIME_WS)
        .withModuleName(PUBLIC_SPACETIME_MODULE_NAME)
        .withToken(accessToken)
        .onConnect((conn, identity) => {
            console.log("SpacetimeDB connection established.");
            conn.subscriptionBuilder()
                .onApplied(() => {
                    spacetime.update((s) => ({
                        ...s,
                        connection: conn,
                        status: "connected",
                    }));
                })
                .subscribe([
                    sql("SELECT * FROM user WHERE identity = ?", [
                        identity.toHexString(),
                    ]),
                ]);
        })
        .onDisconnect(async () => {
            console.log("SpacetimeDB disconnected.");
            spacetime.set({connection: null, status: "disconnected"});

            console.log("Attempting to refresh token and reconnect...");
            const refreshed = await refreshAccessToken();
            if (refreshed) {
                connectToSpacetime(); // Reconnect with the new token
            } else {
                const errorMsg =
                    "Could not refresh session. Please log in again.";
                console.error(errorMsg);
                spacetime.set({
                    connection: null,
                    status: "error",
                    error: errorMsg,
                });
                setTimeout(() => goto("/login"), 2000);
            }
        })
        .onConnectError(async () => {
            console.log("SpacetimeDB connection error.");
            spacetime.set({connection: null, status: "disconnected"});

            console.log("Attempting to refresh token and reconnect...");
            const refreshed = await refreshAccessToken();
            if (refreshed) {
                connectToSpacetime(); // Reconnect with the new token
            } else {
                const errorMsg =
                    "Could not refresh session. Please log in again.";
                console.error(errorMsg);
                spacetime.set({
                    connection: null,
                    status: "error",
                    error: errorMsg,
                });
                setTimeout(() => goto("/login"), 2000);
            }
        })
        .build();
}

// A variable to track the connection promise, preventing multiple connection attempts.
let connectionPromise: Promise<SpacetimeState> | null = null;

/**
 * Ensure the spacetime store reaches a connected state and return the connected handle.
 *
 * Waits for the module-level `spacetime` store to become `status === "connected"`. Concurrent
 * callers are de-duplicated (only one connection attempt is performed). If the store does not
 * become connected within `timeoutMs`, the returned promise rejects.
 *
 * @param timeoutMs - Maximum time in milliseconds to wait for a connection (default: 3000).
 * @returns A promise that resolves with the connected SpacetimeState.
 * @throws Error if the connection times out or the store enters an `error` state.
 */
export async function ensureSpacetimeConnected(
    timeoutMs: number = 3000
): Promise<SpacetimeState> {
    const currentSpacetime = get(spacetime);
    if (currentSpacetime.status === "connected") {
        return currentSpacetime;
    }

    if (connectionPromise) {
        return connectionPromise;
    }

    connectionPromise = new Promise((resolve, reject) => {
        // Set up a timeout that rejects the promise.
        const timeoutId = setTimeout(() => {
            unsub();
            reject(new Error("Spacetime connection timed out."));
        }, timeoutMs);

        const unsub = spacetime.subscribe((s) => {
            if (s.status === "connected" && s.connection) {
                clearTimeout(timeoutId);
                unsub();
                resolve(s);
            } else if (s.status === "error") {
                clearTimeout(timeoutId);
                unsub();
                reject(new Error(s.error || "Spacetime connection failed."));
            }
        });

        // Initiate the connection only if we are not already connecting.
        if (get(spacetime).status === "disconnected") {
            try {
                connectToSpacetime();
            } catch (err) {
                clearTimeout(timeoutId);
                unsub();
                reject(err);
            }
        }
    });

    try {
        return await connectionPromise;
    } finally {
        connectionPromise = null;
    }
}

/**
 * Retrieves the currently authenticated Spacetime DB user, waiting for a connection if needed.
 *
 * Ensures a Spacetime connection (up to `timeoutMs`) and then locates the user whose identity
 * matches the active connection. If not running in the browser, or if no connection is available,
 * returns `null`. If the user record is missing any required cryptographic fields, navigates the
 * browser to `/auth/setup` and returns `null`.
 *
 * @param timeoutMs - Maximum time in milliseconds to wait for a connected Spacetime handle (default: 3000)
 * @returns The matched `User` when connected and fully provisioned, otherwise `null`
 */
export async function getSpacetimeUser(
    timeoutMs: number = 3000
): Promise<User | null> {
    if (!browser) return null;

    const handle = await ensureSpacetimeConnected(timeoutMs);
    if (!handle.connection) {
        console.error("Error, no handle was provided");
        return null;
    }

    const currentUser = Array.from(handle.connection.db.user.iter()).find(
        (user) => user.identity.isEqual(handle.connection!.identity!)
    );

    if (
        !currentUser?.publicKey ||
        !currentUser.encryptedPrivateKey ||
        !currentUser.encryptedBackupKey ||
        !currentUser.publicSigningKey ||
        !currentUser.encryptedPrivateSigningKey ||
        !currentUser.encryptedPrivateBackupSigningKey ||
        !currentUser.argonSalt
    ) {
        await goto("/auth/setup");
        return null;
    }

    return currentUser;
}
