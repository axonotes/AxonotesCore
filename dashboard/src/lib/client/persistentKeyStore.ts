import {type IDBPDatabase, openDB} from "idb";
import {VaultKeys} from "$lib/client/cryptoStore";

const DB_NAME = "axonotes-vault-db";
const STORE_NAME = "crypto-keys";
const KEY_ID = "user-private-key";

/**
 * Opens (or creates) the IndexedDB database for storing vault keys and ensures the object store exists.
 *
 * Opens the database named "axonotes-vault-db" at version 1. During upgrade, creates the "crypto-keys"
 * object store if it does not already exist.
 *
 * @returns The opened IDB database instance.
 */
async function getDb(): Promise<IDBPDatabase> {
    return openDB(DB_NAME, 1, {
        upgrade(db) {
            if (!db.objectStoreNames.contains(STORE_NAME)) {
                db.createObjectStore(STORE_NAME);
            }
        },
    });
}

/**
 * Persist a non-extractable VaultKeys value into the module's IndexedDB store.
 *
 * The provided `key` (a non-extractable CryptoKey handle wrapped as `VaultKeys`)
 * is saved under the internal, fixed key ID so it can be retrieved later with
 * `loadKey()`. This function does not return a value.
 *
 * @param key - The non-extractable CryptoKey handle to persist. It will be stored
 *   under the module's fixed key identifier and overwrite any existing entry.
 */
export async function storeKey(key: VaultKeys): Promise<void> {
    const db = await getDb();
    await db.put(STORE_NAME, key, KEY_ID);
    console.log("Non-extractable key stored in IndexedDB.");
}

/**
 * Retrieve the stored VaultKeys from IndexedDB.
 *
 * @returns The stored `VaultKeys` instance, or `null` if no key is found.
 */
export async function loadKey(): Promise<VaultKeys | null> {
    const db = await getDb();
    const key = await db.get(STORE_NAME, KEY_ID);
    if (key) {
        console.log("Non-extractable key loaded from IndexedDB.");
        return key;
    }
    return null;
}

/**
 * Remove the stored VaultKeys entry from IndexedDB.
 *
 * Deletes the record stored under the fixed key identifier (`KEY_ID`) in the module's object store.
 * Resolves when the delete operation completes; typically called when clearing credentials (e.g., on logout).
 */
export async function deleteKey(): Promise<void> {
    const db = await getDb();
    await db.delete(STORE_NAME, KEY_ID);
    console.log("Key deleted from IndexedDB.");
}
