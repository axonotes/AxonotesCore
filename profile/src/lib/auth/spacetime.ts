

export async function getSpacetimeJWT(): Promise<{
    error: null;
    token: string;
} | {
    error: string;
    token: null;
}> {
    const response = await fetch("/api/spacetimedb-token");

    if (!response.ok) {
        if (response.status === 401) {
            return {
                error: "Unauthorized",
                token: null,
            }
        }
        return {
            error: "Unknown error",
            token: null,
        }
    }

    const { token } = await response.json();

    return {
        error: null,
        token,
    }
}