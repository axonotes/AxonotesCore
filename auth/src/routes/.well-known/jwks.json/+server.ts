import {json} from '@sveltejs/kit';
import {SPACETIMEDB_JWK} from '$env/static/private';

export async function GET() {
    try {
        if (!SPACETIMEDB_JWK) {
            console.error('Missing SPACETIMEDB_JWK in environment variables');
            return json({error: 'JWKS not configured.'}, {status: 500});
        }

        const jwk = JSON.parse(SPACETIMEDB_JWK);
        // Validate basic JWK requirements
        if (!jwk.kty || !jwk.kid) {
            console.error('Invalid JWK format in SPACETIMEDB_JWK');
            return json({error: 'Invalid JWKS configuration.'}, {status: 500});
        }
        return json({
            keys: [jwk], // The JWKS is an object containing a 'keys' array
        });
    } catch (error) {
        console.error('Failed to serve JWKS:', error);
        return json(
            {error: 'Failed to generate JWKS.'},
            {status: 500},
        );
    }
}
