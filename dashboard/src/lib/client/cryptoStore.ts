import {writable} from "svelte/store";
import * as crypto from "$lib/client/crypto";
import * as persistentKeyStore from "$lib/client/persistentKeyStore";
import {browser} from "$app/environment";

/**
 * This Svelte store holds the client-side cryptographic key.
 * It is initialized to `null`, representing a "locked" state.
 * The value will be a non-extractable `CryptoKey` handle when "unlocked".
 */
const {subscribe, set} = writable<CryptoKey | null>(null);

/**
 * On application startup, this code block executes immediately.
 * It attempts to load the persistent key from IndexedDB. If successful,
 * the store is updated, and the user's vault is unlocked for the session.
 */
if (browser) {
    persistentKeyStore.loadKey().then((key) => {
        if (key) {
            console.log("Vault unlocked via persistent key on startup.");
            set(key);
        }
    });
}

/**
 * The public interface for the crypto store.
 * Components can subscribe to this to react to lock/unlock state changes.
 */
export const privateKeyStore = {
    subscribe,
};

/**
 * Unlocks the vault using the user's master password.
 * This is the fallback method if a key is not found in IndexedDB.
 * It decrypts the private key, imports it as non-extractable,
 * and stores it in both IndexedDB and the in-memory Svelte store.
 *
 * @param encryptedPrivateKey The encrypted private key object from the server.
 * @param password The user's master password.
 * @param salt The salt used to hash the password.
 * @returns A boolean indicating if the unlock was successful.
 */
export async function unlockVaultWithPassword(
    encryptedPrivateKey: crypto.EncryptedData,
    password: string,
    salt: Uint8Array
): Promise<boolean> {
    try {
        const passwordHash = await crypto.hashPassword(password, salt);

        const privateKeyString = await crypto.decryptWithAes(
            encryptedPrivateKey,
            passwordHash
        );

        const privateKeyBytes = crypto.base64ToUint8Array(privateKeyString);
        const keyHandle = await crypto.importPrivateKey(privateKeyBytes);
        await persistentKeyStore.storeKey(keyHandle);

        set(keyHandle);
        return true;
    } catch (error) {
        console.error("Failed to decrypt key. Invalid password.", error);
        set(null); // Ensure the store is in a locked state on failure.
        return false;
    }
}

/**
 * Locks the vault by clearing the key from both persistent and in-memory storage.
 * This should be called on user logout.
 */
export async function lockVault() {
    await persistentKeyStore.deleteKey();
    set(null);
    console.log("Vault has been locked.");
}
