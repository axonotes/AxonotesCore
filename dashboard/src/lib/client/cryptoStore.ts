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
 * Persistently stores the provided VaultKeys in IndexedDB and updates the in-memory vault store to the unlocked state.
 *
 * The `keys` object contains non-extractable CryptoKey handles (RSA-OAEP private key and Ed25519 private key).
 *
 * @param keys - VaultKeys to persist and set in the store
 * @returns A promise that resolves once the keys have been stored and the store updated
 */
export async function setAndStoreVaultKeys(keys: VaultKeys) {
    await persistentKeyStore.storeKey(keys);
    set(keys);
    console.log("Vault keys set and stored during initial setup.");
}

/**
 * Decrypts server-provided encrypted private keys with the user's master password, imports them as non-extractable CryptoKey handles, persists them to IndexedDB, and updates the in-memory vault store.
 *
 * @param encryptedPrivateKey - The server-provided encrypted RSA private key blob.
 * @param encryptedPrivateSigningKey - The server-provided encrypted Ed25519 private key blob.
 * @param password - The user's master password used to derive the decryption key.
 * @param salt - Salt used when hashing the password to derive the AES key.
 * @returns True if keys were successfully decrypted, imported, persisted, and the vault unlocked; false on error (the vault will be left locked).
 */
export async function unlockVaultWithPassword(
    encryptedPrivateKey: crypto.EncryptedData,
    encryptedPrivateSigningKey: crypto.EncryptedData,
    password: string,
    salt: Uint8Array<ArrayBuffer>
): Promise<boolean> {
    try {
        const passwordHash = (await crypto.hashPassword(
            password,
            salt
        )) as Uint8Array<ArrayBuffer>;

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
