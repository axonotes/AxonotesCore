import {writable} from "svelte/store";
import * as crypto from "$lib/client/crypto";
import * as persistentKeyStore from "$lib/client/persistentKeyStore";
import {browser} from "$app/environment";

export type VaultKeys = {
    decryptionKey: CryptoKey; // RSA-OAEP private key
    signingKey: CryptoKey; // Ed25519 private key
};

/**
 * This Svelte store holds the client-side cryptographic key.
 * It is initialized to `null`, representing a "locked" state.
 * The value will be a non-extractable `CryptoKey` handle when "unlocked".
 */
const {subscribe, set} = writable<VaultKeys | null>(null);

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
export const vaultStore = {
    subscribe,
};

/**
 * Directly sets the vault keys in the Svelte store and persists them
 * to IndexedDB. This is used during the initial account setup.
 * @param keys The VaultKeys object containing the non-extractable key handles.
 */
export async function setAndStoreVaultKeys(keys: VaultKeys) {
    await persistentKeyStore.storeKey(keys);
    set(keys);
    console.log("Vault keys set and stored during initial setup.");
}

/**
 * Unlocks the vault using the user's master password.
 * This is the fallback method if a key is not found in IndexedDB.
 * It decrypts the private key, imports it as non-extractable,
 * and stores it in both IndexedDB and the in-memory Svelte store.
 *
 * @param encryptedPrivateKey The encrypted private key object from the server.
 * @param encryptedPrivateSigningKey The encrypted private key object for signing.
 * @param password The user's master password.
 * @param salt The salt used to hash the password.
 * @returns A boolean indicating if the unlock was successful.
 */
export async function unlockVaultWithPassword(
    encryptedPrivateKey: crypto.EncryptedData,
    encryptedPrivateSigningKey: crypto.EncryptedData,
    password: string,
    salt: Uint8Array
): Promise<boolean> {
    try {
        const passwordHash = await crypto.hashPassword(password, salt);

        const privateKeyString = await crypto.decryptWithAes(
            encryptedPrivateKey,
            passwordHash
        );
        const privateSigningKeyString = await crypto.decryptWithAes(
            encryptedPrivateSigningKey,
            passwordHash
        );

        const privateKeyBytes = crypto.base64ToUint8Array(privateKeyString);
        const decryptionKeyHandle =
            await crypto.importPrivateKey(privateKeyBytes);

        const privateSigningKeyBytes = crypto.base64ToUint8Array(
            privateSigningKeyString
        );
        const signingKeyHandle = await crypto.importEd25519PrivateKey(
            privateSigningKeyBytes
        );

        const keys: VaultKeys = {
            decryptionKey: decryptionKeyHandle,
            signingKey: signingKeyHandle,
        };

        await persistentKeyStore.storeKey(keys);
        set(keys);
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
