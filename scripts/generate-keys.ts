import { generateKeyPairSync } from "node:crypto";
import { writeFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

// --- Configuration ---
// These paths are relative to this script's location
const PRIVATE_KEY_FILENAME = "../server/jwt/private_key.pem";
const PUBLIC_KEY_FILENAME = "../server/jwt/public_key.pem";

// --- Get script's own directory ---
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// --- Create absolute paths for the key files ---
const privateKeyPath = path.join(__dirname, PRIVATE_KEY_FILENAME);
const publicKeyPath = path.join(__dirname, PUBLIC_KEY_FILENAME);

// --- Ensure target directory exists ---
const keyDir = path.dirname(privateKeyPath);
try {
    mkdirSync(keyDir, {recursive: true});
} catch (error) {
    console.error("❌ Failed to create key directory:", error);
    process.exit(1);
}

// --- Script ---
console.log(
    "🔑 Generating ECDSA (ES256) key pair using prime256v1 curve...",
);

const { privateKey, publicKey } = generateKeyPairSync("ec", {
    namedCurve: "prime256v1",
    publicKeyEncoding: {
        type: "spki",
        format: "pem",
    },
    privateKeyEncoding: {
        type: "pkcs8",
        format: "pem",
    },
});

try {
    console.log(`\n💾 Saving PEM keys to the script's directory:`);
    console.log(`   -> ${privateKeyPath}`);
    console.log(`   -> ${publicKeyPath}`);
    writeFileSync(privateKeyPath, privateKey);
    writeFileSync(publicKeyPath, publicKey);
    console.log("   ✅ Files created successfully.");
} catch (error) {
    console.error("   ❌ Failed to write key files:", error);
}

const privateKeyBase64 = Buffer.from(privateKey).toString("base64");
const publicKeyBase64 = Buffer.from(publicKey).toString("base64");

console.log(
    "\n📋 Copy the following lines into your dashboard/.env file for your SvelteKit app:\n",
);
console.log("=".repeat(80));
console.log(`JWT_PRIVATE_KEY_BASE64="${privateKeyBase64}"`);
console.log(`JWT_PUBLIC_KEY_BASE64="${publicKeyBase64}"`);
console.log(
    `JWT_KEY_ID="axonotes-key-${new Date().toISOString().slice(0, 10)}"`,
);
console.log("=".repeat(80));