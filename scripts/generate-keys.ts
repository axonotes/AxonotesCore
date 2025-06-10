import {generateKeyPairSync} from "node:crypto";
import {
    writeFileSync,
    mkdirSync,
    readFileSync,
    existsSync,
    appendFileSync,
} from "node:fs";
import path from "node:path";
import {fileURLToPath} from "node:url";
import readline from "node:readline";

// --- Configuration ---
const PRIVATE_KEY_FILENAME = "../server/jwt/private_key.pem";
const PUBLIC_KEY_FILENAME = "../server/jwt/public_key.pem";
const ENV_TEMPLATE_PATH = "../dashboard/.example.env";
const ENV_OUTPUT_PATH = "../dashboard/.env";

// --- Helper for user input ---
const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
});

const askQuestion = (query: string): Promise<string> => {
    return new Promise((resolve) => rl.question(query, resolve));
};

/**
 * Generates and saves the ECDSA key pair and returns the Base64 encoded keys.
 * @param privateKeyPath - Absolute path to save the private key.
 * @param publicKeyPath - Absolute path to save the public key.
 * @returns An object containing the Base64 encoded keys and the key ID.
 */
const generateAndSaveKeys = (privateKeyPath: string, publicKeyPath: string) => {
    // 1. Ensure target directory for PEM keys exists
    const keyDir = path.dirname(privateKeyPath);
    mkdirSync(keyDir, {recursive: true});

    // 2. Generate the ECDSA key pair
    console.log(
        "🔑 Generating ECDSA (ES256) key pair using prime256v1 curve..."
    );
    const {privateKey, publicKey} = generateKeyPairSync("ec", {
        namedCurve: "prime256v1",
        publicKeyEncoding: {type: "spki", format: "pem"},
        privateKeyEncoding: {type: "pkcs8", format: "pem"},
    });
    console.log("   ✅ Key pair generated successfully.");

    // 3. Save the PEM key files
    console.log(`\n💾 Saving PEM keys to:`);
    console.log(`   -> ${privateKeyPath}`);
    console.log(`   -> ${publicKeyPath}`);
    writeFileSync(privateKeyPath, privateKey);
    writeFileSync(publicKeyPath, publicKey);
    console.log("   ✅ PEM files created successfully.");

    // 4. Prepare values for the .env file
    const privateKeyBase64 = Buffer.from(privateKey).toString("base64");
    const publicKeyBase64 = Buffer.from(publicKey).toString("base64");
    const keyId = `axonotes-key-${new Date().toISOString().slice(0, 10)}`;

    return {
        JWT_PRIVATE_KEY_BASE64: privateKeyBase64,
        JWT_PUBLIC_KEY_BASE64: publicKeyBase64,
        JWT_KEY_ID: keyId,
    };
};

// --- Main script logic ---
(async () => {
    // --- Get script's own directory ---
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = path.dirname(__filename);

    // --- Create absolute paths for all files ---
    const privateKeyPath = path.join(__dirname, PRIVATE_KEY_FILENAME);
    const publicKeyPath = path.join(__dirname, PUBLIC_KEY_FILENAME);
    const envTemplatePath = path.join(__dirname, ENV_TEMPLATE_PATH);
    const envOutputPath = path.join(__dirname, ENV_OUTPUT_PATH);

    try {
        if (existsSync(envOutputPath)) {
            // --- .env file EXISTS ---
            console.log(
                `\nℹ️  An existing .env file was found at '${envOutputPath}'.`
            );
            let envContent = readFileSync(envOutputPath, "utf-8");

            // Check if keys are already in the file
            const keysExist = /^\s*JWT_PRIVATE_KEY_BASE64\s*=/m.test(
                envContent
            );

            if (keysExist) {
                // --- Keys EXIST, ask to overwrite ---
                const answer = await askQuestion(
                    "   JWT keys already exist in it. Overwrite them? (y/N) "
                );
                if (answer.toLowerCase() !== "y") {
                    console.log(
                        "\n🚫 Operation cancelled. Your files have not been changed."
                    );
                    rl.close();
                    return;
                }

                console.log("\n👌 OK, overwriting existing keys.");
                const replacements = generateAndSaveKeys(
                    privateKeyPath,
                    publicKeyPath
                );

                for (const [key, value] of Object.entries(replacements)) {
                    const regex = new RegExp(`^(${key}=)(.*)$`, "m");
                    if (regex.test(envContent)) {
                        envContent = envContent.replace(regex, `$1"${value}"`);
                    } else {
                        // If key was in a different format or commented, append it
                        envContent += `\n${key}="${value}"`;
                    }
                }
                writeFileSync(envOutputPath, envContent);
                console.log(`\n💾 Updated keys in: ${envOutputPath}`);
            } else {
                // --- Keys DO NOT exist, append them ---
                console.log(
                    "\nℹ️  The file does not contain JWT keys. Appending new ones."
                );
                const replacements = generateAndSaveKeys(
                    privateKeyPath,
                    publicKeyPath
                );

                const contentToAppend =
                    "\n\n# --- JWT Keys Generated by setup script ---\n" +
                    Object.entries(replacements)
                        .map(([key, value]) => `${key}="${value}"`)
                        .join("\n") +
                    "\n";

                appendFileSync(envOutputPath, contentToAppend);
                console.log(`\n💾 Appended keys to: ${envOutputPath}`);
            }
        } else {
            // --- .env file DOES NOT EXIST, create from template ---
            console.log(
                `\nℹ️  No .env file found. Creating a new one from template...`
            );
            if (!existsSync(envTemplatePath)) {
                console.error(
                    `❌ Template file missing at '${ENV_TEMPLATE_PATH}'. Cannot create .env file.`
                );
                process.exit(1);
            }

            const replacements = generateAndSaveKeys(
                privateKeyPath,
                publicKeyPath
            );
            let envContent = readFileSync(envTemplatePath, "utf-8");

            for (const [key, value] of Object.entries(replacements)) {
                const regex = new RegExp(`^(${key}=)(.*)$`, "m");
                envContent = envContent.replace(regex, `$1"${value}"`);
            }

            writeFileSync(envOutputPath, envContent);
            console.log(`\n💾 Created new .env file at: ${envOutputPath}`);
        }

        console.log(
            "\n✅ All files generated successfully! Your setup is complete."
        );
    } catch (error) {
        console.error("\n❌ An unexpected error occurred:", error);
        process.exit(1);
    } finally {
        rl.close();
    }
})();
