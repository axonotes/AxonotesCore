import {mnemonicToSeedSync} from "bip39";
import argon2 from "argon2-browser/dist/argon2-bundled.min";

// --- Helper Functions for data conversion ---

/**
 * Converts an ArrayBuffer to a Base64 string.
 */
export function arrayBufferToBase64(buffer: ArrayBufferLike): string {
    const bytes = new Uint8Array(buffer);
    const chunkSize = 0x8000; // Process in 32KB chunks
    let binary = "";

    for (let i = 0; i < bytes.length; i += chunkSize) {
        const chunk = bytes.subarray(i, i + chunkSize);
        binary += String.fromCharCode(...chunk);
    }

    return window.btoa(binary);
}

/**
 * Decodes a standard Base64-encoded string into a Uint8Array of raw bytes.
 *
 * @param base64 - Base64 string (standard alphabet, not URL-safe) to decode.
 * @returns A Uint8Array containing the decoded bytes.
 */
export function base64ToUint8Array(base64: string): Uint8Array<ArrayBuffer> {
    const binary_string = window.atob(base64);
    const len = binary_string.length;
    const bytes = new Uint8Array(len);
    for (let i = 0; i < len; i++) {
        bytes[i] = binary_string.charCodeAt(i);
    }
    return bytes;
}

// --- Core Crypto Functions ---

export interface EncryptedData {
    iv: string;
    data: string;
}

/**
 * Generates a 4096-bit RSA-OAEP key pair (SHA-256).
 *
 * The public key is exported in SPKI format and the private key in PKCS#8 format;
 * both are returned as Base64-encoded strings.
 *
 * @returns An object containing `publicKey` (SPKI, Base64) and `privateKey` (PKCS#8, Base64).
 */
export async function generateRsaKeyPair(): Promise<{
    publicKey: string;
    privateKey: string;
}> {
    const keyPair = await window.crypto.subtle.generateKey(
        {
            name: "RSA-OAEP",
            modulusLength: 4096,
            publicExponent: new Uint8Array([0x01, 0x00, 0x01]), // 65537
            hash: "SHA-256",
        },
        true,
        ["encrypt", "decrypt"]
    );

    const publicKey = await window.crypto.subtle.exportKey(
        "spki",
        keyPair.publicKey
    );
    const privateKey = await window.crypto.subtle.exportKey(
        "pkcs8",
        keyPair.privateKey
    );

    return {
        publicKey: arrayBufferToBase64(publicKey),
        privateKey: arrayBufferToBase64(privateKey),
    };
}

/**
 * Encrypts a UTF-8 string with AES-GCM (AES-256) and returns Base64-encoded output.
 *
 * Uses the Web Crypto API to:
 * - generate a random 12-byte IV,
 * - import the provided raw key for AES-GCM encryption,
 * - encrypt the UTF-8 encoded `data`.
 *
 * @param data - Plaintext to encrypt (UTF-8 string).
 * @param key - Raw AES key bytes; must be 32 bytes for AES-256.
 * @returns An object containing `iv` (Base64-encoded 12-byte IV) and `data` (Base64-encoded ciphertext).
 */
export async function encryptWithAes(
    data: string,
    key: Uint8Array<ArrayBuffer>
): Promise<EncryptedData> {
    const iv = window.crypto.getRandomValues(new Uint8Array(12));
    const encoder = new TextEncoder();
    const encodedData = encoder.encode(data);

    const cryptoKey = await window.crypto.subtle.importKey(
        "raw",
        key,
        {name: "AES-GCM"},
        false,
        ["encrypt"]
    );

    const encryptedData = await window.crypto.subtle.encrypt(
        {
            name: "AES-GCM",
            iv: iv,
        },
        cryptoKey,
        encodedData
    );

    return {
        iv: arrayBufferToBase64(iv.buffer),
        data: arrayBufferToBase64(encryptedData),
    };
}

/**
 * Decrypts an AES-GCM-encrypted payload and returns the plaintext string.
 *
 * Expects `encrypted.iv` and `encrypted.data` to be Base64-encoded. `key` is the raw AES key bytes (32 bytes for AES-256). The function imports the key for AES-GCM decryption and returns the UTF-8 decoded plaintext.
 *
 * The returned promise rejects if decryption fails (for example, due to authentication/tag failure or malformed input).
 *
 * @param encrypted - Object containing Base64-encoded `iv` and `data` produced by the corresponding AES-GCM encryption.
 * @param key - Raw AES key bytes (Uint8Array); use a 32-byte key for AES-256-GCM.
 * @returns The decrypted plaintext as a string.
 */
export async function decryptWithAes(
    encrypted: EncryptedData,
    key: Uint8Array<ArrayBuffer>
): Promise<string> {
    const iv = base64ToUint8Array(encrypted.iv);
    const data = base64ToUint8Array(encrypted.data);

    const cryptoKey = await window.crypto.subtle.importKey(
        "raw",
        key,
        {name: "AES-GCM"},
        false,
        ["decrypt"]
    );

    const decryptedData = await window.crypto.subtle.decrypt(
        {
            name: "AES-GCM",
            iv: iv,
        },
        cryptoKey,
        data
    );

    const decoder = new TextDecoder();
    return decoder.decode(decryptedData);
}

/**
 * Derives a 32-byte Argon2id hash from a plaintext password.
 *
 * Uses Argon2id with parameters: time=3, memory=128 MB, parallelism=1, and output length 32 bytes.
 *
 * @param password - The plaintext password to hash.
 * @param salt - Cryptographically random salt bytes (recommended >= 16 bytes).
 * @returns The raw 32-byte hash as a Uint8Array.
 */
export async function hashPassword(
    password: string,
    salt: Uint8Array
): Promise<Uint8Array<ArrayBufferLike>> {
    const hashResult = await argon2.hash({
        pass: password,
        salt: salt,
        time: 3,
        mem: 128 * 1024,
        hashLen: 32,
        parallelism: 1,
        type: argon2.ArgonType.Argon2id,
    });
    return hashResult.hash;
}

/**
 * Derives a 32-byte key from a BIP39 mnemonic.
 *
 * @param mnemonic - A BIP39 mnemonic phrase to derive the key from.
 * @returns A 32-byte Uint8Array representing the derived key.
 */
</result>
export function getKeyFromMnemonic(mnemonic: string): Uint8Array {
    const seedBuffer = mnemonicToSeedSync(mnemonic);
    const seed = Uint8Array.from(seedBuffer);
    return seed.slice(0, 32);
}

/**
 * Produce cryptographically secure random bytes to be used as a salt.
 *
 * Returns a Uint8Array of length `byteLength` filled with cryptographically secure random values from the Web Crypto API.
 *
 * @param byteLength - Number of random bytes to generate (default: 16).
 * @returns A `Uint8Array` containing `byteLength` random bytes.
 */
export function generateSalt(byteLength = 16): Uint8Array {
    return window.crypto.getRandomValues(new Uint8Array(byteLength));
}

/**
 * Imports a PKCS#8 RSA-OAEP private key (SHA-256) into the browser Web Crypto API as a non-extractable CryptoKey for decryption.
 *
 * @param rawPrivateKey - Private key bytes in PKCS#8 (BufferSource) format.
 * @returns A non-extractable CryptoKey configured for the "decrypt" usage.
 */
export async function importPrivateKey(
    rawPrivateKey: BufferSource
): Promise<CryptoKey> {
    return await window.crypto.subtle.importKey(
        "pkcs8", // Private key format
        rawPrivateKey,
        {
            name: "RSA-OAEP",
            hash: "SHA-256",
        },
        false, // VERY IMPORTANT
        ["decrypt"] // Key usage
    );
}

/**
 * Generates an Ed25519 signing key pair and returns both keys Base64-encoded.
 *
 * The public key is exported in raw format; the private key is exported in PKCS#8.
 *
 * @returns An object with `publicKey` and `privateKey` as Base64 strings.
 */
export async function generateEd25519KeyPair(): Promise<{
    publicKey: string;
    privateKey: string;
}> {
    const keyPair = await window.crypto.subtle.generateKey(
        {
            name: "Ed25519",
        },
        true, // Can be exported
        ["sign", "verify"]
    );

    const publicKey = await window.crypto.subtle.exportKey(
        "raw",
        keyPair.publicKey
    );
    const privateKey = await window.crypto.subtle.exportKey(
        "pkcs8",
        keyPair.privateKey
    );

    return {
        publicKey: arrayBufferToBase64(publicKey),
        privateKey: arrayBufferToBase64(privateKey),
    };
}

/**
 * Imports a PKCS#8 Ed25519 private key and returns a non-extractable CryptoKey usable for signing.
 *
 * The provided key must be in PKCS#8 format (BufferSource). The resulting CryptoKey is marked
 * non-extractable and has the "sign" usage.
 *
 * @param rawPrivateKey - PKCS#8-encoded private key material as a BufferSource.
 * @returns A non-extractable CryptoKey configured for Ed25519 signing.
 */
export async function importEd25519PrivateKey(
    rawPrivateKey: BufferSource
): Promise<CryptoKey> {
    return await window.crypto.subtle.importKey(
        "pkcs8",
        rawPrivateKey,
        {
            name: "Ed25519",
        },
        false, // Non-extractable
        ["sign"]
    );
}

/**
 * Sign a UTF-8 string with an Ed25519 private key and return the signature as Base64.
 *
 * The `privateKey` must be an Ed25519 private CryptoKey usable for signing. The input
 * `data` is encoded as UTF-8 before signing.
 *
 * @param privateKey - Ed25519 private key CryptoKey for signing.
 * @param data - The string message to sign (encoded as UTF-8).
 * @returns The signature encoded as a Base64 string.
 */
export async function signData(
    privateKey: CryptoKey,
    data: string
): Promise<string> {
    const encoder = new TextEncoder();
    const encodedData = encoder.encode(data);

    const signature = await window.crypto.subtle.sign(
        "Ed25519",
        privateKey,
        encodedData
    );

    return arrayBufferToBase64(signature);
}
