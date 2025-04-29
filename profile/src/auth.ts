import {SvelteKitAuth} from "@auth/sveltekit";
import GitHub from "@auth/sveltekit/providers/github"
import Google from "@auth/sveltekit/providers/google"
import {AUTH_SECRET} from "$env/static/private";
import {devLog, inDev} from "$lib/utils";

export const {handle, signIn, signOut} = SvelteKitAuth({
    providers: [GitHub, Google],
    session: {strategy: "jwt"},
    secret: AUTH_SECRET,
    callbacks: {
        async jwt({token, account, profile}) {
            devLog("token: ", token);
            devLog("account: ", account);
            devLog("profile: ", profile);

            if (account && profile) {
                token.provider = account.provider;
                token.providerAccountId = account.providerAccountId;

                token.email = profile.email ?? token.email;

                token.name = profile.name ?? token.name;
                token.picture = profile.picture ?? token.image ?? token.picture;

                token.sub = `${account.provider}:${account.providerAccountId}`;
            }

            return token;
        },
        async session({session, token}) {
            if (token && token.sub && token.email && token.accessToken) {
                session.user.id = token.sub;
                session.user.email = token.email;
                session.user.image = token.picture;
                session.user.name = token.name;
            }
            devLog("session: ", session);
            return session;
        }
    },
    useSecureCookies: !inDev(),
})