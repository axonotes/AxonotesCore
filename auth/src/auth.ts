import Google from '@auth/sveltekit/providers/google';
import {SvelteKitAuth} from '@auth/sveltekit';
import {importPKCS8, SignJWT} from 'jose';
import NodeCache from 'node-cache';

import {
    AUTH_SECRET,
    JWT_CACHE_TTL,
    SPACETIMEDB_AUDIENCE,
    SPACETIMEDB_ISSUER,
    SPACETIMEDB_KEY_ID,
    SPACETIMEDB_PRIVATE_KEY_PEM,
    SPACETIMEDB_JWT_EXPIRATION,
} from '$env/static/private';

const DEFAULT_JWT_TTL = 300;
const CACHE_CHECK_PERIOD = 60;
export const JWT_TLL = JWT_CACHE_TTL && !isNaN(parseInt(JWT_CACHE_TTL)) ? parseInt(JWT_CACHE_TTL) : DEFAULT_JWT_TTL

// Initialize the cache for SpacetimeDB JWTs
export const jwtCache = new NodeCache({
    stdTTL: JWT_TLL,
    checkperiod: CACHE_CHECK_PERIOD,
    deleteOnExpire: true,
});

// Function to generate the SpacetimeDB JWT
export async function generateSpacetimeDBToken(
    userId: string, // Typically from OAuth user.id or user.email
): Promise<string> {
    if (
        !SPACETIMEDB_PRIVATE_KEY_PEM ||
        !SPACETIMEDB_ISSUER ||
        !SPACETIMEDB_AUDIENCE ||
        !SPACETIMEDB_KEY_ID ||
        !SPACETIMEDB_JWT_EXPIRATION
    ) {
        console.error('Missing SpacetimeDB JWT configuration in .env');
        throw new Error('SpacetimeDB JWT configuration incomplete.');
    }

    try {
        const privateKey = await importPKCS8(
            SPACETIMEDB_PRIVATE_KEY_PEM.replaceAll("\\n", '\n'), // Ensure newlines are correct
            'EdDSA',
        ); // Algorithm must match key type

        return await new SignJWT({}) // SpacetimeDB often uses claims directly in payload
            .setProtectedHeader({
                alg: 'EdDSA', // Algorithm used to sign
                kid: SPACETIMEDB_KEY_ID, // Key ID for JWKS lookup
                typ: 'JWT',
            })
            .setSubject(userId)
            .setIssuer(SPACETIMEDB_ISSUER)
            .setAudience(SPACETIMEDB_AUDIENCE)
            .setIssuedAt()
            .setExpirationTime(SPACETIMEDB_JWT_EXPIRATION) // e.g., 5 minutes validity
            .sign(privateKey);
    } catch (error) {
        console.error('Error signing SpacetimeDB JWT:', error);
        throw new Error('Failed to generate SpacetimeDB token.');
    }
}

export const {handle, signIn, signOut} = SvelteKitAuth({
    providers: [
        Google
    ],
    secret: AUTH_SECRET,
    trustHost: true,

    callbacks: {
        // signIn and jwt can be minimal or default, as they are not involved in request_id linking
        async signIn({user, account}) {
            console.log(`[signIn callback] User ${user.id || user.email} signing in via ${account?.provider}.`);
            return true;
        },
        async jwt({token}) {
            return token;
        },

        // THIS IS THE KEY CHANGE
        async redirect({baseUrl}) {
            return `${baseUrl}/auth/complete-link`;
        },
    },

    pages: {
        signIn: "/signin",
        error: "/error",
    }
});
