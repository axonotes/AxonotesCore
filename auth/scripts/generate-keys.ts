import * as jose from 'jose';
import * as crypto from 'crypto';

async function generateKeys(): Promise<void> {
    // --- Auth.js Secret ---
    console.log('Generating AUTH_SECRET for Auth.js...');
    const authSecretBytes = crypto.randomBytes(32); // 32 bytes = 256 bits
    const authSecret = authSecretBytes.toString('base64url'); // Using base64url for env var safety
    console.log('AUTH_SECRET generated.');


    // --- EdDSA Keys for SpacetimeDB & Client ---
    const alg = 'EdDSA'; // Algorithm for Ed25519
    const keyIdFromEnv: string | undefined = process.env.SPACETIMEDB_KEY_ID;
    const keyId: string = keyIdFromEnv || `sptdb-key-${Date.now()}`;

    console.log(`\nGenerating ${alg} key pair with kid: ${keyId} for SpacetimeDB...`);
    console.log(`This key pair will also be exported in raw formats for client use.`);

    const { publicKey, privateKey }: { publicKey: jose.KeyObject; privateKey: jose.KeyObject } =
        await jose.generateKeyPair(alg, {
            crv: 'Ed25519',
            extractable: true,
        });

    // --- Keys for SpacetimeDB (JWTs, JWKS) ---
    console.log('\n--- SpacetimeDB Keys (for server-side JWT signing/verification) ---');

    const privateKeyPem: string = await jose.exportPKCS8(privateKey);
    console.log('\n1. SpacetimeDB Private Key (PKCS8 PEM):');
    console.log(privateKeyPem);

    const publicKeyPem: string = await jose.exportSPKI(publicKey);
    console.log('\n2. SpacetimeDB Public Key (SPKI PEM):');
    console.log(publicKeyPem);

    const publicJwkForSpacetimeDB: jose.JWK = await jose.exportJWK(publicKey);
    publicJwkForSpacetimeDB.alg = alg;
    publicJwkForSpacetimeDB.kid = keyId;
    publicJwkForSpacetimeDB.use = 'sig';
    console.log('\n3. SpacetimeDB Public Key (JWK for JWKS endpoint):');
    console.log(JSON.stringify(publicJwkForSpacetimeDB, null, 2));


    // --- Keys for Client Application (Ed25519 raw, for tweetnacl) ---
    console.log('\n\n--- Client Application Keys (for client-side signing with tweetnacl) ---');

    if (!publicJwkForSpacetimeDB.x) {
        throw new Error("Could not extract 'x' (public key) from JWK for client format.");
    }
    const clientEd25519PublicKeyB64Url: string = publicJwkForSpacetimeDB.x;
    console.log('\n1. Client Ed25519 Public Key (raw, 32-byte, Base64URL encoded):');
    console.log('   (Server uses this as CLIENT_ED25519_PUBLIC_KEY_B64URL for tweetnacl verification)');
    console.log(`   ${clientEd25519PublicKeyB64Url}`);

    const privateJwkForClient: jose.JWK = await jose.exportJWK(privateKey);
    if (!privateJwkForClient.d) {
        throw new Error("Could not extract 'd' (private key seed) from JWK for client format.");
    }
    const clientEd25519PrivateKeySeedB64Url: string = privateJwkForClient.d;
    console.log('\n2. Client Ed25519 Private Key SEED (raw, 32-byte, Base64URL encoded):');
    console.log('   (Client application uses this seed with tweetnacl.sign.keyPair.fromSeed())');
    console.log(`   ${clientEd25519PrivateKeySeedB64Url}`);


    // --- Summary for .env file ---
    console.log('\n\n--- Copy these values to your .env file (or client configuration) ---');

    console.log('\n# For Auth.js (NextAuth.js)');
    console.log(`AUTH_SECRET='${authSecret}'`);

    console.log('\n# For SpacetimeDB (server-side JWTs, JWKS)');
    console.log(`SPACETIMEDB_PRIVATE_KEY_PEM='${privateKeyPem.replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_PUBLIC_KEY_PEM='${publicKeyPem.replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_JWK='${JSON.stringify(publicJwkForSpacetimeDB).replace(/\n/g, '\\n')}'`);
    console.log(`SPACETIMEDB_KEY_ID='${keyId}'`);

    console.log('\n# For Client Signature Verification (server-side, using tweetnacl)');
    console.log(`# Put this in your server's .env file:`);
    console.log(`CLIENT_ED25519_PUBLIC_KEY_B64URL='${clientEd25519PublicKeyB64Url}'`);

    console.log('\n# For Client Application (to be embedded or configured securely in the client)');
    console.log(`# This is the client's private key seed. Handle with extreme care.`);
    console.log(`# CLIENT_ED25519_PRIVATE_KEY_SEED_B64URL='${clientEd25519PrivateKeySeedB64Url}'`);


    console.log(
        `\n>>> Ensure SPACETIMEDB_KEY_ID in .env matches the kid: ${keyId} for SpacetimeDB JWK.`,
    );
    console.log(
        `>>> The CLIENT_ED25519_PUBLIC_KEY_B64URL is for the server to verify client signatures.`,
    );
    console.log(
        `>>> The CLIENT_ED25519_PRIVATE_KEY_SEED_B64URL is for the client app to generate signatures.`,
    );
    console.log(
        `    It is highly sensitive. Do NOT commit it to your server's repository if the client is separate.`,
    );
    console.log(
        `    Embed it securely within your client application or use a secure configuration mechanism.`,
    );
    console.log(
        `>>> The AUTH_SECRET is used by Auth.js for session encryption, CSRF protection, etc. Keep it secret!`,
    );
}

generateKeys().catch((error) => {
    console.error('Failed to generate keys:', error);
    process.exit(1);
});
