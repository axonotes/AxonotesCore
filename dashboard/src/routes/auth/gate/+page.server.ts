import {error, redirect} from "@sveltejs/kit";
import {JWT_COOKIE_NAME} from "$env/static/private";
import {
    PUBLIC_SPACETIME_MODULE_NAME,
    PUBLIC_SPACETIME_WS,
} from "$env/static/public";
import {DbConnection} from "$lib/module_bindings";
import {sql} from "$lib/server/spacetime";

const CONNECTION_TIMEOUT_MS = 5000; // 5 seconds

/**
 * Connects to SpacetimeDB and waits for the initial data subscription to be
 * applied. This function wraps the callback-based SDK in a Promise to enable
 * async/await usage and includes a connection timeout.
 *
 * @param token The authentication token for the database connection.
 * @returns A Promise that resolves with the `DbConnection` object once synced.
 * @throws An `Error` on connection failure or timeout.
 */
function connectAndSync(token: string): Promise<DbConnection> {
    return new Promise((resolve, reject) => {
        let connection: DbConnection | null = null;

        const timeoutId = setTimeout(() => {
            connection?.disconnect();
            reject(
                new Error(
                    "Timed out while waiting for SpacetimeDB connection and data sync."
                )
            );
        }, CONNECTION_TIMEOUT_MS);

        connection = DbConnection.builder()
            .withUri(PUBLIC_SPACETIME_WS)
            .withModuleName(PUBLIC_SPACETIME_MODULE_NAME)
            .withToken(token)
            .onConnect((conn, identity) => {
                console.log("SpacetimeDB connection established.");
                conn.subscriptionBuilder()
                    .onApplied(() => {
                        clearTimeout(timeoutId); // Don't disconnect, just clear the timeout
                        resolve(conn); // Resolve with the active connection
                    })
                    .subscribe([
                        sql("SELECT * FROM user WHERE identity = ?", [
                            identity.toHexString(),
                        ]),
                    ]);
            })
            .build();
    });
}

/**
 * Server-side load function. It authenticates the user, connects to the
 * database, and redirects them based on their account setup status.
 */
export async function load({locals, cookies}) {
    if (!locals.user) {
        throw redirect(302, "/login");
    }

    const token = cookies.get(JWT_COOKIE_NAME);
    if (!token) {
        // Clear a potentially stale cookie and redirect to login
        cookies.delete(JWT_COOKIE_NAME, {path: "/"});
        throw redirect(302, "/login");
    }

    let connection: DbConnection | null = null;
    try {
        // Await the helper, pausing execution until the DB is connected and synced.
        connection = await connectAndSync(token);

        // db.user should only contain 1 user because of our sync call
        const currentUser = Array.from(connection.db.user.iter()).find((user) =>
            user.identity.isEqual(connection!.identity!)
        );

        if (currentUser?.publicKey) {
            // User has a public key, so they need to unlock their account.
            throw redirect(302, "/auth/unlock");
        } else {
            // User needs to complete the initial encryption setup.
            throw redirect(302, "/auth/setup");
        }
    } catch (e) {
        if (e instanceof Error) {
            console.error("Caught an error during SpacetimeDB sync:", e);
            throw error(503, {
                message: `Service Unavailable: Could not sync with database. ${e.message}`,
            });
        }

        // Re-throw the object (likely a Redirect) for SvelteKit to handle.
        throw e;
    } finally {
        if (connection) {
            console.log("Disconnecting from SpacetimeDB.");
            connection.disconnect();
        }
    }
}
