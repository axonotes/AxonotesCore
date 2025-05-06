import {json} from '@sveltejs/kit';
// @ts-ignore
import {SPACETIMEDB_JWK} from '$env/static/private';

export async function GET() {
    try {
        if (!SPACETIMEDB_JWK) {
            console.error('Missing SPACETIMEDB_JWK in environment variables');
            return json({error: 'JWKS not configured.'}, {status: 500});
        }

        const jwk = JSON.parse(SPACETIMEDB_JWK);
        return json({
            keys: [jwk], // The JWKS is an object containing a 'keys' array
        });
    } catch (error) {
        console.error('Failed to serve JWKS:', error);
        return json(
            {error: 'Failed to generate JWKS.', details: error.message},
            {status: 500},
        );
    }
}
