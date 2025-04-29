import {SPACETIMEDB_SECRET} from "$env/static/private";
import type {RequestHandler} from "./$types";
import {error, json} from "@sveltejs/kit";
import jwt from "jsonwebtoken";

if (!SPACETIMEDB_SECRET) {
    throw new Error("SPACETIMEDB_SECRET is not set");
}

export const GET: RequestHandler = async ({locals}) => {
    const session = await locals.auth();

    if (!session?.user) {
        throw error(401, 'Unauthorized: You must be logged in.');
    }

    const userId = session.user.id;
    if (!userId) {
        throw error(500, 'User ID not found in session.');
    }

    const userName = session.user.name;
    if (!userName) {
        throw error(500, 'User Name not found in session.');
    }

    const userEmail = session.user.email;
    if (!userEmail) {
        throw error(500, 'User Email not found in session.');
    }

    const payload = {
        sub: userId,
        name: userName,
        email: userEmail,
    };

    const expiresIn = '15m';
    const spacetimedbToken = jwt.sign(
        payload,
        SPACETIMEDB_SECRET,
        {expiresIn}
    );

    return json({
        token: spacetimedbToken,
    })
}