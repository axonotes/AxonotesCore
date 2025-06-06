import { redirect } from "@sveltejs/kit";

export async function POST({ cookies }) {
    cookies.delete("axonotes_session", { path: "/" });
    throw redirect(302, "/");
}