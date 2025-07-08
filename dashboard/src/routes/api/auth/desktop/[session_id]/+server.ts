import {json, error} from "@sveltejs/kit";
import {desktopAuthManager} from "$lib/server/desktop-auth";

export async function GET({params}) {
    const sessionId = params.session_id;

    if (!sessionId) {
        throw error(400, "Session ID is required");
    }

    try {
        const tokens = await desktopAuthManager.getTokens(sessionId);

        if (!tokens) {
            throw error(404, "Session not found or expired");
        }

        return json(tokens);
    } catch (err) {
        console.error("Desktop auth token retrieval failed:", err);
        throw error(400, (err as Error).message);
    }
}
