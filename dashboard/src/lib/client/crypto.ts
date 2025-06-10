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

/** Converts a Base64 string to a Uint8Array. */
export function base64ToUint8Array(base64: string): Uint8Array {
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

/** Generates a 4096-bit RSA-OAEP key pair. */
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

/** Encrypts data using AES-256-GCM with the Web Crypto API. */
export async function encryptWithAes(
    data: string,
    key: Uint8Array
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

/** Decrypts data using AES-256-GCM with the Web Crypto API. */
export async function decryptWithAes(
    encrypted: EncryptedData,
    key: Uint8Array
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

/** Hashes a password with Argon2id. */
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

/** Derives a key from a BIP39 mnemonic. */
export function getKeyFromMnemonic(mnemonic: string): Uint8Array {
    const seedBuffer = mnemonicToSeedSync(mnemonic);
    const seed = Uint8Array.from(seedBuffer);
    return seed.slice(0, 32);
}

/** Generates a cryptographically secure random salt. */
export function generateSalt(byteLength = 16): Uint8Array {
    return window.crypto.getRandomValues(new Uint8Array(byteLength));
}

/**
 * Imports a raw private key into the browser's crypto engine as a
 * non-extractable CryptoKey object.
 * @param rawPrivateKey The private key in BufferSource format.
 * @returns A non-extractable CryptoKey handle.
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

/** Generates a Ed25519 key pair for signing. */
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
 * Imports a raw Ed25519 private key for signing.
 * @param rawPrivateKey The private key in BufferSource format.
 * @returns A non-extractable CryptoKey handle for signing.
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
 * Signs data with a given Ed25519 private key.
 * @param privateKey The CryptoKey handle for the private signing key.
 * @param data The string data to sign.
 * @returns The signature as a Base64 string.
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
