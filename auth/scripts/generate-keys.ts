import * as jose from 'jose';
import { fileURLToPath } from 'url';

// Helper to get __dirname in ES modules with TypeScript
// @ts-ignore
const __filename = fileURLToPath(import.meta.url);

async function generateKeys(): Promise<void> {
    const alg = 'EdDSA'; // Algorithm for Ed25519
    const keyIdFromEnv: string | undefined = process.env.SPACETIMEDB_KEY_ID;
    const keyId: string = keyIdFromEnv || `sptdb-key-${Date.now()}`;

    console.log(`Generating ${alg} key pair with kid: ${keyId}...`);

    // Generate the EdDSA (Ed25519) key pair
    // @ts-ignore
    const { publicKey, privateKey }: { publicKey: jose.KeyLike; privateKey: jose.KeyLike } =
        await jose.generateKeyPair(alg, {
            crv: 'Ed25519', // Specify the curve for EdDSA
            extractable: true
        });

    // Export private key as PKCS8 PEM
    const privateKeyPem: string = await jose.exportPKCS8(privateKey);

    // Export public key as SPKI PEM (optional, but good for reference)
    const publicKeyPem: string = await jose.exportSPKI(publicKey);

    // Export public key as JWK for the JWKS endpoint
    const publicJwk: jose.JWK = await jose.exportJWK(publicKey);
    publicJwk.alg = alg; // Explicitly set the algorithm in the JWK
    publicJwk.kid = keyId; // Add the Key ID to the JWK
    publicJwk.use = 'sig'; // Specify key usage as 'signature'

    console.log('\n--- Private Key (PKCS8 PEM) ---');
    console.log(
        'Copy this to your .env file as SPACETIMEDB_PRIVATE_KEY_PEM:',
    );
    console.log(privateKeyPem);

    console.log('\n--- Public Key (SPKI PEM) ---');
    console.log(publicKeyPem);

    console.log('\n--- Public Key (JWK for JWKS endpoint) ---');
    console.log(JSON.stringify(publicJwk, null, 2));

    console.log(
        `\n>>> Ensure SPACETIMEDB_KEY_ID in .env matches the kid: ${keyId} <<<`,
    );
    console.log(
        `>>> Ensure SPACETIMEDB_PRIVATE_KEY_PEM in .env is set with the private key above. <<<`,
    );

    console.log(
        'Copy this to your .env file:'
    )

    console.log(`SPACETIMEDB_PRIVATE_KEY_PEM='${privateKeyPem.replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_PUBLIC_KEY_PEM='${publicKeyPem.replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_JWK='${JSON.stringify(publicJwk).replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_KEY_ID='${keyId}'`);
}

generateKeys().catch((error) => {
    console.error('Failed to generate keys:', error);
    process.exit(1);
});
