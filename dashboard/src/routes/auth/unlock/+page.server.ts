import {error, redirect} from "@sveltejs/kit";
import {JWT_COOKIE_NAME} from "$env/static/private";
import {
    PUBLIC_SPACETIME_MODULE_NAME,
    PUBLIC_SPACETIME_WS,
} from "$env/static/public";
import {DbConnection} from "$lib/module_bindings";
import {sql} from "$lib/server/spacetime";
import type {EncryptedData} from "$lib/client/crypto";

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
                        clearTimeout(timeoutId);
                        resolve(conn);
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

export async function load({locals, cookies}) {
    if (!locals.user) {
        throw redirect(302, "/login");
    }

    const token = cookies.get(JWT_COOKIE_NAME);
    if (!token) {
        cookies.delete(JWT_COOKIE_NAME, {path: "/"});
        throw redirect(302, "/login");
    }

    let connection: DbConnection | null = null;
    try {
        connection = await connectAndSync(token);

        const currentUser = Array.from(connection.db.user.iter()).find((user) =>
            user.identity.isEqual(connection!.identity!)
        );

        // If user has no public key, they haven't completed setup.
        if (
            !currentUser?.publicKey ||
            !currentUser.encryptedPrivateKey ||
            !currentUser.argonSalt
        ) {
            throw redirect(302, "/auth/setup");
        }

        // The private key is stored as a JSON string in the DB.
        // We parse it here before sending it to the client.
        const encryptedPrivateKey: EncryptedData = JSON.parse(
            currentUser.encryptedPrivateKey
        );

        return {
            encryptedPrivateKey,
            argonSalt: currentUser.argonSalt,
        };
    } catch (e) {
        if (e instanceof Error) {
            console.error("Caught an error during SpacetimeDB sync:", e);
            throw error(503, {
                message: `Service Unavailable: Could not sync with database. ${e.message}`,
            });
        }
        // Re-throw redirect objects for SvelteKit to handle
        throw e;
    } finally {
        if (connection) {
            console.log("Disconnecting from SpacetimeDB.");
            connection.disconnect();
        }
    }
}
