import { error, json, type RequestEvent } from '@sveltejs/kit';
import {
    PUBLIC_SPACETIME_MODULE_NAME,
    PUBLIC_SPACETIME_WS,
} from '$env/static/public';
import { JWT_COOKIE_NAME } from '$env/static/private';
import { DbConnection, type ErrorContext } from '$lib/module_bindings';

const OPERATION_TIMEOUT_MS = 5000; // 5 seconds

/**
 * Defines the structure of the payload for the `setEncryption` reducer.
 */
type SetEncryptionPayload = {
    publicKey: string;
    encryptedPrivateKey: string;
    encryptedBackupKey: string;
    argonSalt: string;
};

/**
 * A helper function that connects to SpacetimeDB and calls the `setEncryption`
 * reducer. It wraps the callback-based connection logic in a Promise to
 * simplify asynchronous handling, including a timeout.
 *
 * @param token The user's authentication token.
 * @param data The payload for the `setEncryption` reducer.
 * @returns A Promise that resolves on success and rejects on failure or timeout.
 */
function callSetEncryptionReducer(
    token: string,
    data: SetEncryptionPayload,
): Promise<void> {
    return new Promise((resolve, reject) => {
        let connection: DbConnection | null = null;

        const timeoutId = setTimeout(() => {
            connection?.disconnect();
            reject(new Error('Database operation timed out.'));
        }, OPERATION_TIMEOUT_MS);

        const cleanupAndDisconnect = () => {
            clearTimeout(timeoutId);
            connection?.disconnect();
        };

        connection = DbConnection.builder()
            .withUri(PUBLIC_SPACETIME_WS)
            .withModuleName(PUBLIC_SPACETIME_MODULE_NAME)
            .withToken(token)
            .onConnect((conn) => {
                console.log('SpacetimeDB connection established for reducer call.');

                conn.reducers.setEncryption(
                    data.publicKey,
                    data.encryptedPrivateKey,
                    data.encryptedBackupKey,
                    data.argonSalt,
                );

                setTimeout(() => {
                    cleanupAndDisconnect();
                    resolve();
                }, 100);
            })
            .onConnectError((_ctx: ErrorContext, err: Error) => {
                console.error('Error connecting to SpacetimeDB:', err);
                cleanupAndDisconnect();
                reject(err);
            })
            .build();
    });
}

/**
 * Handles the POST request to set a user's encryption keys in the database.
 * @param {RequestEvent} event - The SvelteKit request event.
 */
export async function POST({ request, locals, cookies }: RequestEvent) {
    if (!locals.user) {
        throw error(401, 'Not authenticated');
    }

    const token = cookies.get(JWT_COOKIE_NAME);
    if (!token) {
        throw error(401, 'Authentication token is missing');
    }

    const body: Partial<SetEncryptionPayload> = await request.json();
    const { publicKey, encryptedPrivateKey, encryptedBackupKey, argonSalt } =
        body;

    // Validate that all required fields are present
    if (
        !publicKey ||
        !encryptedPrivateKey ||
        !encryptedBackupKey ||
        !argonSalt
    ) {
        throw error(400, 'Missing required fields');
    }

    try {
        await callSetEncryptionReducer(token, {
            publicKey,
            encryptedPrivateKey,
            encryptedBackupKey,
            argonSalt,
        });

        return json({ success: true }, { status: 200 });
    } catch (e) {
        console.error('Failed to execute setEncryption reducer:', e);
        if (e instanceof Error) {
            throw error(
                503,
                `Service Unavailable: Could not update database. ${e.message}`,
            );
        }
        throw e;
    }
}