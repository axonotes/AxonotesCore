import {type IDBPDatabase, openDB} from "idb";
import {VaultKeys} from "$lib/client/cryptoStore";

const DB_NAME = "axonotes-vault-db";
const STORE_NAME = "crypto-keys";
const KEY_ID = "user-private-key";

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
 * Stores a non-extractable CryptoKey in IndexedDB.
 * @param key The CryptoKey handle to store.
 */
export async function storeKey(key: VaultKeys): Promise<void> {
    const db = await getDb();
    await db.put(STORE_NAME, key, KEY_ID);
    console.log("Non-extractable key stored in IndexedDB.");
}

/**
 * Retrieves the CryptoKey from IndexedDB.
 * @returns The CryptoKey handle or null if not found.
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
 * Deletes the key from IndexedDB (e.g., on logout).
 */
export async function deleteKey(): Promise<void> {
    const db = await getDb();
    await db.delete(STORE_NAME, KEY_ID);
    console.log("Key deleted from IndexedDB.");
}
